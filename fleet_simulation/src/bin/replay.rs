//! src/bin/replay.rs
//!
//! Reads positions.csv and messages.csv (both with email column),
//! then replays all MQTT calls with the original timing.
//!
//! Tokens are calculated at replay time by logging in each user
//! using the email stored in the CSV, so they are always fresh.
//!
//! Run with:
//!     cargo run --bin replay
//!     cargo run --bin replay -- positions.csv messages.csv

use csv::Reader;
use reqwest::{Client, Error};
use serde::Deserialize;
use std::collections::BTreeMap;
use std::path::Path;
use std::time::Duration;
use tokio::task::JoinHandle;
use tokio::time::{sleep, Instant};

use fleet_simulation::mqtt::{initialize_mqtt_client, send_message, send_position};

// ===================================================================
// HTTP client
// ===================================================================

fn build_http_client() -> Client {
    Client::builder()
        .danger_accept_invalid_certs(true)
        .build()
        .expect("failed to build HTTP client")
}

async fn login_user(email: &str) -> Result<String, Error> {
    let client = build_http_client();
    let credentials = (email, "Password123!");

    let token = client
        .post("https://127.0.0.1:3001/api/login")
        .json(&credentials)
        .send()
        .await?
        .json::<TokenResponse>()
        .await?
        .token;

    Ok(token)
}

#[derive(Deserialize, Debug)]
struct TokenResponse {
    token: String,
}

// ===================================================================
// CSV reading
// ===================================================================

#[derive(Debug, Deserialize, Clone)]
struct PositionRow {
    user_id: i64,
    email: String,
    lat: f64,
    lon: f64,
    elapsed_from_start_ms: u64,
    #[allow(dead_code)]
    elapsed_from_last_ms: u64,
}

#[derive(Debug, Deserialize, Clone)]
struct MessageRow {
    user_id: i64,
    email: String,
    message: String,
    elapsed_from_start_ms: u64,
    #[allow(dead_code)]
    elapsed_from_last_ms: u64,
}

#[derive(Debug, Clone)]
enum Event {
    Position { lat: f64, lon: f64, offset_ms: u64 },
    Message { content: String, offset_ms: u64 },
}

fn read_positions_csv(path: &str) -> Result<Vec<PositionRow>, Box<dyn std::error::Error + Send + Sync>> {
    let mut rdr = Reader::from_path(path)?;
    let rows: Vec<PositionRow> = rdr.deserialize().collect::<Result<Vec<_>, _>>()?;
    Ok(rows)
}

fn read_messages_csv(path: &str) -> Result<Vec<MessageRow>, Box<dyn std::error::Error + Send + Sync>> {
    if !Path::new(path).exists() {
        return Ok(Vec::new());
    }
    let metadata = std::fs::metadata(path)?;
    if metadata.len() == 0 {
        return Ok(Vec::new());
    }
    let mut rdr = Reader::from_path(path)?;
    let rows: Vec<MessageRow> = rdr.deserialize().collect::<Result<Vec<_>, _>>()?;
    Ok(rows)
}

// ===================================================================
// Replay logic
// ===================================================================

async fn replay_user(
    user_id: i64,
    email: String,
    mut events: Vec<Event>,
) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    if events.is_empty() {
        println!("[user {}] No events to replay", user_id);
        return Ok(());
    }

    events.sort_by_key(|e| match e {
        Event::Position { offset_ms, .. } => *offset_ms,
        Event::Message { offset_ms, .. } => *offset_ms,
    });

    println!("[user {}] Logging in as {} ...", user_id, email);
    let token = match login_user(&email).await {
        Ok(t) => {
            println!("[user {}] Login successful", user_id);
            t
        }
        Err(e) => {
            return Err(format!("Login failed for user {} ({}): {}", user_id, email, e).into());
        }
    };

    let client_id = format!("replay_user_{}", user_id);
    let mqtt_client = initialize_mqtt_client(&client_id, "broker.emqx.io", 8883).await?;
    sleep(Duration::from_millis(500)).await;

    let start = Instant::now();

    for event in &events {
        let offset_ms = match &event {
            Event::Position { offset_ms, .. } => *offset_ms,
            Event::Message { offset_ms, .. } => *offset_ms,
        };
        let target = Duration::from_millis(offset_ms);
        let elapsed = start.elapsed();

        if target > elapsed {
            sleep(target - elapsed).await;
        } else if elapsed > target + Duration::from_millis(100) {
            eprintln!(
                "[user {}] Behind schedule by {:?} at offset {:?}",
                user_id,
                elapsed - target,
                target
            );
        }

        match &event {
            Event::Position { lat, lon, .. } => {
                send_position(&mqtt_client, user_id, &token, *lat, *lon).await?;
                println!(
                    "[user {} | +{:?}] position: {:.5}, {:.5}",
                    user_id, target, lat, lon
                );
            }
            Event::Message { content, .. } => {
                send_message(&mqtt_client, user_id, &token, content).await?;
                println!(
                    "[user {} | +{:?}] message: {}",
                    user_id, target, content
                );
            }
        }
    }

    println!("[user {}] Replay completed ({} events)", user_id, events.len());
    Ok(())
}

// ===================================================================
// main
// ===================================================================

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    let positions_path = std::env::args()
        .nth(1)
        .unwrap_or_else(|| "positions.csv".to_string());
    let messages_path = std::env::args()
        .nth(2)
        .unwrap_or_else(|| "messages.csv".to_string());

    println!("Loading positions from: {}", positions_path);
    println!("Loading messages from: {}", messages_path);

    let positions = read_positions_csv(&positions_path)?;
    let messages = read_messages_csv(&messages_path)?;

    if positions.is_empty() && messages.is_empty() {
        return Err("Both CSV files are empty or missing".into());
    }

    println!("Read {} position rows, {} message rows", positions.len(), messages.len());

    // Group events by user_id, keeping the email from the first row seen
    let mut events_by_user: BTreeMap<i64, (String, Vec<Event>)> = BTreeMap::new();

    for pos in positions {
        let entry = events_by_user
            .entry(pos.user_id)
            .or_insert_with(|| (pos.email.clone(), Vec::new()));
        entry.1.push(Event::Position {
            lat: pos.lat,
            lon: pos.lon,
            offset_ms: pos.elapsed_from_start_ms,
        });
    }

    for msg in messages {
        let entry = events_by_user
            .entry(msg.user_id)
            .or_insert_with(|| (msg.email.clone(), Vec::new()));
        entry.1.push(Event::Message {
            content: msg.message,
            offset_ms: msg.elapsed_from_start_ms,
        });
    }

    println!("Replaying for {} user(s)...", events_by_user.len());

    let mut handles: Vec<JoinHandle<Result<(), Box<dyn std::error::Error + Send + Sync>>>> = Vec::new();

    for (user_id, (email, events)) in events_by_user {
        handles.push(tokio::spawn(replay_user(user_id, email, events)));
    }

    for handle in handles {
        match handle.await {
            Ok(Ok(())) => {}
            Ok(Err(e)) => eprintln!("Replay task error: {}", e),
            Err(e) => eprintln!("Replay task panicked: {:?}", e),
        }
    }

    println!("All replays finished.");
    Ok(())
}