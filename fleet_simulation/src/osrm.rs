//! Simulazione movimento veicoli su rete stradale reale
//! =======================================================
//!
//! 1. `ottieni_percorso` chiede a OSRM il percorso reale tra due punti
//!    (con velocità stimata tratto per tratto).
//! 2. `SimulatoreVeicolo` mantiene lo stato di avanzamento lungo quel
//!    percorso: ad ogni chiamata a `prossima_posizione(dt_sec)` avanza
//!    di `dt_sec` secondi e restituisce la posizione corrispondente,
//!    oppure `None` se il veicolo ha raggiunto la destinazione.
//!
//! Questo permette di chiamare `prossima_posizione(30.0)` dentro un loop
//! con `tokio::time::sleep(30s)` per simulare il movimento in tempo
//! reale (es. pubblicando ogni posizione via MQTT), invece di generare
//! tutto il tracciato in anticipo.

use anyhow::{Context, Result};
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

const VELOCITA_FALLBACK_KMH: f64 = 35.0; // usata se OSRM non fornisce velocità per un tratto

// ----------------------------------------------------------------------
// Strutture dati
// ----------------------------------------------------------------------

#[derive(Debug, Clone, Serialize)]
pub struct Posizione {
    pub id_veicolo: String,
    pub timestamp: DateTime<Utc>,
    pub lat: f64,
    pub lon: f64,
    pub velocita_kmh: f64,
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
    speed: Option<Vec<f64>>, // metri/secondo, uno per ogni segmento coords[i]->coords[i+1]
}

/// Un segmento della rotta: punto iniziale, punto finale (lon, lat) e
/// velocità stimata in m/s, più la lunghezza precalcolata in metri.
#[derive(Debug, Clone)]
struct Segmento {
    lon1: f64,
    lat1: f64,
    lon2: f64,
    lat2: f64,
    velocita_m_s: f64,
    lunghezza_m: f64,
    durata_sec: f64,
}

// ----------------------------------------------------------------------
// 1. Richiesta del percorso reale a OSRM
// ----------------------------------------------------------------------

/// Chiama OSRM e restituisce il percorso come lista di segmenti pronti
/// per il campionamento (con lunghezza e durata già calcolate).
async fn ottieni_percorso(lon1: f64, lat1: f64, lon2: f64, lat2: f64) -> Result<Vec<Segmento>> {
    let url = format!(
        "https://router.project-osrm.org/route/v1/driving/{},{};{},{}\
         ?overview=full&geometries=geojson&annotations=speed",
        lon1, lat1, lon2, lat2
    );

    let risposta: OsrmResponse = reqwest::get(&url)
        .await
        .context("richiesta a OSRM fallita")?
        .json()
        .await
        .context("risposta OSRM non valida")?;

    let rotta = risposta
        .routes
        .into_iter()
        .next()
        .context("nessun percorso trovato tra i due punti")?;

    let coords = rotta.geometry.coordinates;

    // Le annotazioni di velocità sono per-leg (una leg per ogni coppia di
    // waypoint passati); qui usiamo un solo leg (origine->destinazione),
    // ma il codice regge comunque più leg concatenate.
    let mut velocita_per_segmento: Vec<f64> = Vec::new();
    for leg in &rotta.legs {
        if let Some(ann) = &leg.annotation {
            if let Some(speeds) = &ann.speed {
                velocita_per_segmento.extend(speeds.iter().copied());
            }
        }
    }

    let mut segmenti = Vec::with_capacity(coords.len().saturating_sub(1));
    for i in 0..coords.len().saturating_sub(1) {
        let [lon1, lat1] = coords[i];
        let [lon2, lat2] = coords[i + 1];

        let velocita_m_s = velocita_per_segmento
            .get(i)
            .copied()
            .filter(|v| *v > 0.1) // scarta valori nulli/assurdi
            .unwrap_or(VELOCITA_FALLBACK_KMH * 1000.0 / 3600.0);

        let lunghezza_m = distanza_metri(lon1, lat1, lon2, lat2);
        let durata_sec = lunghezza_m / velocita_m_s;

        segmenti.push(Segmento {
            lon1,
            lat1,
            lon2,
            lat2,
            velocita_m_s,
            lunghezza_m,
            durata_sec,
        });
    }

    if segmenti.is_empty() {
        anyhow::bail!("percorso OSRM senza segmenti percorribili");
    }

    Ok(segmenti)
}

// ----------------------------------------------------------------------
// 2. Distanza haversine (metri) tra due punti lat/lon
// ----------------------------------------------------------------------

fn distanza_metri(lon1: f64, lat1: f64, lon2: f64, lat2: f64) -> f64 {
    const RAGGIO_TERRA_M: f64 = 6_371_000.0;
    let (lat1r, lat2r) = (lat1.to_radians(), lat2.to_radians());
    let dlat = (lat2 - lat1).to_radians();
    let dlon = (lon2 - lon1).to_radians();

    let a = (dlat / 2.0).sin().powi(2) + lat1r.cos() * lat2r.cos() * (dlon / 2.0).sin().powi(2);
    let c = 2.0 * a.sqrt().atan2((1.0 - a).sqrt());
    RAGGIO_TERRA_M * c
}

// ----------------------------------------------------------------------
// 3. Simulatore stateful: una posizione ogni N secondi
// ----------------------------------------------------------------------

/// Tiene lo stato di avanzamento di un veicolo lungo un percorso OSRM.
/// Ad ogni chiamata a `prossima_posizione` avanza nel tempo e restituisce
/// la posizione corrispondente, finché non raggiunge la destinazione.
pub struct SimulatoreVeicolo {
    id_veicolo: String,
    segmenti: Vec<Segmento>,
    indice_segmento: usize,
    tempo_nel_segmento_sec: f64, // secondi già "percorsi" nel segmento corrente
    terminato: bool,
}

impl SimulatoreVeicolo {
    /// Scarica il percorso reale da OSRM e prepara il simulatore.
    /// `origine` e `destinazione` sono tuple (lon, lat).
    pub async fn nuovo(
        id_veicolo: &str,
        origine: (f64, f64),
        destinazione: (f64, f64),
    ) -> Result<Self> {
        let segmenti = ottieni_percorso(origine.0, origine.1, destinazione.0, destinazione.1).await?;

        Ok(Self {
            id_veicolo: id_veicolo.to_string(),
            segmenti,
            indice_segmento: 0,
            tempo_nel_segmento_sec: 0.0,
            terminato: false,
        })
    }

    /// Avanza di `dt_sec` secondi lungo il percorso (tipicamente 30.0) e
    /// restituisce la nuova posizione. Restituisce `None` quando il
    /// veicolo ha raggiunto la destinazione (percorso terminato).
    pub fn prossima_posizione(&mut self, dt_sec: f64) -> Option<Posizione> {
        if self.terminato {
            return None;
        }

        let mut tempo_da_avanzare = dt_sec;

        while self.indice_segmento < self.segmenti.len() {
            let seg = &self.segmenti[self.indice_segmento];
            let tempo_rimasto_nel_segmento = seg.durata_sec - self.tempo_nel_segmento_sec;

            if tempo_da_avanzare < tempo_rimasto_nel_segmento {
                // Il veicolo resta in questo segmento: interpola la posizione.
                self.tempo_nel_segmento_sec += tempo_da_avanzare;
                let frazione = if seg.durata_sec > 0.0 {
                    self.tempo_nel_segmento_sec / seg.durata_sec
                } else {
                    1.0
                };

                let lat = seg.lat1 + (seg.lat2 - seg.lat1) * frazione;
                let lon = seg.lon1 + (seg.lon2 - seg.lon1) * frazione;

                return Some(Posizione {
                    id_veicolo: self.id_veicolo.clone(),
                    timestamp: Utc::now(),
                    lat,
                    lon,
                    velocita_kmh: seg.velocita_m_s * 3.6,
                });
            }

            // Il tempo da avanzare "sfora" il segmento corrente: passa al prossimo.
            tempo_da_avanzare -= tempo_rimasto_nel_segmento;
            self.indice_segmento += 1;
            self.tempo_nel_segmento_sec = 0.0;
        }

        // Percorso esaurito: restituisce l'ultima posizione (destinazione) una volta,
        // poi segna il simulatore come terminato.
        self.terminato = true;
        let ultimo = self.segmenti.last()?;
        Some(Posizione {
            id_veicolo: self.id_veicolo.clone(),
            timestamp: Utc::now(),
            lat: ultimo.lat2,
            lon: ultimo.lon2,
            velocita_kmh: 0.0,
        })
    }

    pub fn e_terminato(&self) -> bool {
        self.terminato
    }

    pub fn id_veicolo(&self) -> &str {
        &self.id_veicolo
    }
}