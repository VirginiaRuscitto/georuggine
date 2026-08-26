//! Vehicle movement simulation on a real road network
//! =======================================================
//!
//! 1. `get_route` asks OSRM for the real route between two points
//!    (with estimated speed segment by segment).
//! 2. `VehicleSimulator` keeps the progress state along that
//!    route: each call to `next_position(dt_sec)` advances
//!    by `dt_sec` seconds and returns the corresponding position,
//!    or `None` if the vehicle has reached its destination.
//!
//! This allows calling `next_position(30.0)` inside a loop
//! with `tokio::time::sleep(30s)` to simulate movement in real
//! time (e.g. publishing each position via MQTT), instead of generating
//! the whole track in advance.

use anyhow::{Context, Result};
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

const FALLBACK_SPEED_KMH: f64 = 35.0; // used if OSRM doesn't provide speed for a segment

// ----------------------------------------------------------------------
// Data structures
// ----------------------------------------------------------------------

#[derive(Debug, Clone, Serialize)]
pub struct Position {
    pub vehicle_id: String,
    pub timestamp: DateTime<Utc>,
    pub lat: f64,
    pub lon: f64,
    pub speed_kmh: f64,
}

#[derive(Debug, Deserialize)]
struct OsrmResponse {
    routes: Vec<OsrmRoute>,
}

#[derive(Debug, Deserialize)]
struct OsrmRoute {
    geometry: OsrmGeometry,
    legs: Vec<OsrmLeg>,
}

#[derive(Debug, Deserialize)]
struct OsrmGeometry {
    coordinates: Vec<[f64; 2]>, // [lon, lat]
}

#[derive(Debug, Deserialize)]
struct OsrmLeg {
    annotation: Option<OsrmAnnotation>,
}

#[derive(Debug, Deserialize)]
struct OsrmAnnotation {
    speed: Option<Vec<f64>>, // meters/second, one per segment coords[i]->coords[i+1]
}

/// A segment of the route: start point, end point (lon, lat) and
/// estimated speed in m/s, plus precomputed length in meters.
#[derive(Debug, Clone)]
struct Segment {
    lon1: f64,
    lat1: f64,
    lon2: f64,
    lat2: f64,
    speed_m_s: f64,
    length_m: f64,
    duration_sec: f64,
}

// ----------------------------------------------------------------------
// 1. Requesting the real route from OSRM
// ----------------------------------------------------------------------

/// Calls OSRM and returns the route as a list of segments ready
/// for sampling (with length and duration already calculated).
async fn get_route(lon1: f64, lat1: f64, lon2: f64, lat2: f64) -> Result<Vec<Segment>> {
    let url = format!(
        "http://localhost:5000/route/v1/driving/{},{};{},{}\
         ?overview=full&geometries=geojson&annotations=speed",
        lon1, lat1, lon2, lat2
    );

    let response: OsrmResponse = reqwest::get(&url)
        .await
        .context("request to OSRM failed")?
        .json()
        .await
        .context("invalid OSRM response")?;

    let route = response
        .routes
        .into_iter()
        .next()
        .context("no route found between the two points")?;

    let coords = route.geometry.coordinates;

    // Speed annotations are per-leg (one leg for each waypoint pair
    // passed in); here we use a single leg (origin->destination),
    // but the code still handles multiple concatenated legs.
    let mut speed_per_segment: Vec<f64> = Vec::new();
    for leg in &route.legs {
        if let Some(ann) = &leg.annotation {
            if let Some(speeds) = &ann.speed {
                speed_per_segment.extend(speeds.iter().copied());
            }
        }
    }

    let mut segments = Vec::with_capacity(coords.len().saturating_sub(1));
    for i in 0..coords.len().saturating_sub(1) {
        let [lon1, lat1] = coords[i];
        let [lon2, lat2] = coords[i + 1];

        let speed_m_s = speed_per_segment
            .get(i)
            .copied()
            .filter(|v| *v > 0.1) // discard null/absurd values
            .unwrap_or(FALLBACK_SPEED_KMH * 1000.0 / 3600.0);

        let length_m = distance_meters(lon1, lat1, lon2, lat2);
        let duration_sec = length_m / speed_m_s;

        segments.push(Segment {
            lon1,
            lat1,
            lon2,
            lat2,
            speed_m_s,
            length_m,
            duration_sec,
        });
    }

    if segments.is_empty() {
        anyhow::bail!("OSRM route has no traversable segments");
    }

    Ok(segments)
}

// ----------------------------------------------------------------------
// 2. Haversine distance (meters) between two lat/lon points
// ----------------------------------------------------------------------

fn distance_meters(lon1: f64, lat1: f64, lon2: f64, lat2: f64) -> f64 {
    const EARTH_RADIUS_M: f64 = 6_371_000.0;
    let (lat1r, lat2r) = (lat1.to_radians(), lat2.to_radians());
    let dlat = (lat2 - lat1).to_radians();
    let dlon = (lon2 - lon1).to_radians();

    let a = (dlat / 2.0).sin().powi(2) + lat1r.cos() * lat2r.cos() * (dlon / 2.0).sin().powi(2);
    let c = 2.0 * a.sqrt().atan2((1.0 - a).sqrt());
    EARTH_RADIUS_M * c
}

// ----------------------------------------------------------------------
// 3. Stateful simulator: one position every N seconds
// ----------------------------------------------------------------------

/// Keeps the progress state of a vehicle along an OSRM route.
/// Each call to `next_position` advances in time and returns
/// the corresponding position, until it reaches the destination.
pub struct VehicleSimulator {
    vehicle_id: String,
    segments: Vec<Segment>,
    segment_index: usize,
    time_in_segment_sec: f64, // seconds already "traveled" in the current segment
    finished: bool,
}

impl VehicleSimulator {
    /// Downloads the real route from OSRM and prepares the simulator.
    /// `origin` and `destination` are (lon, lat) tuples.
    pub async fn new(
        vehicle_id: &str,
        origin: (f64, f64),
        destination: (f64, f64),
    ) -> Result<Self> {
        let segments = get_route(origin.0, origin.1, destination.0, destination.1).await?;

        Ok(Self {
            vehicle_id: vehicle_id.to_string(),
            segments,
            segment_index: 0,
            time_in_segment_sec: 0.0,
            finished: false,
        })
    }

    /// Advances by `dt_sec` seconds along the route (typically 30.0) and
    /// returns the new position. Returns `None` when the
    /// vehicle has reached the destination (route finished).
    pub fn next_position(&mut self, dt_sec: f64) -> Option<Position> {
        if self.finished {
            return None;
        }

        let mut time_to_advance = dt_sec;

        while self.segment_index < self.segments.len() {
            let seg = &self.segments[self.segment_index];
            let time_remaining_in_segment = seg.duration_sec - self.time_in_segment_sec;

            if time_to_advance < time_remaining_in_segment {
                // The vehicle stays in this segment: interpolate the position.
                self.time_in_segment_sec += time_to_advance;
                let fraction = if seg.duration_sec > 0.0 {
                    self.time_in_segment_sec / seg.duration_sec
                } else {
                    1.0
                };

                let lat = seg.lat1 + (seg.lat2 - seg.lat1) * fraction;
                let lon = seg.lon1 + (seg.lon2 - seg.lon1) * fraction;

                return Some(Position {
                    vehicle_id: self.vehicle_id.clone(),
                    timestamp: Utc::now(),
                    lat,
                    lon,
                    speed_kmh: seg.speed_m_s * 3.6,
                });
            }

            // The time to advance "overshoots" the current segment: move to the next one.
            time_to_advance -= time_remaining_in_segment;
            self.segment_index += 1;
            self.time_in_segment_sec = 0.0;
        }

        // Route exhausted: return the last position (destination) once,
        // then mark the simulator as finished.
        self.finished = true;
        let last = self.segments.last()?;
        Some(Position {
            vehicle_id: self.vehicle_id.clone(),
            timestamp: Utc::now(),
            lat: last.lat2,
            lon: last.lon2,
            speed_kmh: 0.0,
        })
    }

    pub fn is_finished(&self) -> bool {
        self.finished
    }

    pub fn vehicle_id(&self) -> &str {
        &self.vehicle_id
    }
}
