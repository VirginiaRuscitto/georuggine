//! src/bin/bake_simulation.rs
//!
//! Simulates user movement and saves positions + messages to CSV.
//! Each row includes user_id AND email so replay can login directly.
//!
//!     cargo run --bin bake_simulation

use chrono::{DateTime, Utc};
use csv::{Reader, WriterBuilder};
use rand::seq::IndexedRandom;
use rand::{rng, RngExt};
use reqwest::{Client, Error};
use serde::{Deserialize, Serialize};
use std::fs::OpenOptions;
use std::path::Path;
use std::sync::LazyLock;
use std::time::{Duration, Instant};
use futures::future::join_all;
use tokio;

#[path = "../osrm.rs"]
mod osrm;
use osrm::SimulatoreVeicolo;

const POSITIONS_CSV: &str = "positions.csv";
const MESSAGES_CSV: &str = "messages.csv";

static PROGRAM_START: LazyLock<Instant> = LazyLock::new(Instant::now);

#[derive(Deserialize, Debug, Clone)]
pub struct User {
    pub id: i64,
    pub name: String,
    pub surname: String,
    pub email: String,
    pub created_at: DateTime<Utc>,
    pub is_admin: bool,
    pub state: UserState,
}

#[derive(Serialize, Deserialize, Debug, Clone, Copy, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum UserState {
    Disconnected,
    Stopped,
    Moving,
}

#[derive(Deserialize, Debug)]
struct TokenResponse {
    token: String,
}

#[derive(Serialize, Debug)]
struct UsersOption {
    is_admin: bool,
}

fn build_http_client() -> Client {
    Client::builder()
        .danger_accept_invalid_certs(true)
        .build()
        .expect("failed to build HTTP client")
}

async fn get_users() -> Result<Vec<User>, Error> {
    let client = build_http_client();
    let credentials = ("admin@example.com", "Password123!");

    let token = client
        .post("https://127.0.0.1:3001/api/login")
        .json(&credentials)
        .send()
        .await?
        .json::<TokenResponse>()
        .await?
        .token;

    let users_option = UsersOption { is_admin: false };

    let users = client
        .get("https://127.0.0.1:3001/api/users")
        .header("Authorization", format!("Bearer {}", token))
        .query(&users_option)
        .send()
        .await?
        .json::<Vec<User>>()
        .await?;

    Ok(users)
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

async fn simulate_user_movement(
    user: User,
    origin: (f64, f64),
    destination: (f64, f64),
    destination_name: String,
    kebab_shops: Vec<KebabShop>,
    token: String,
) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    let origin_lon_lat = (origin.1, origin.0);
    let destination_lon_lat = (destination.1, destination.0);

    let mut simulator = SimulatoreVeicolo::nuovo(
        &user.id.to_string(),
        origin_lon_lat,
        destination_lon_lat,
    ).await?;

    let mut last_pos = (0.0, 0.0);
    let mut last_pos_time = *PROGRAM_START;
    let mut last_msg_time = *PROGRAM_START;

    loop {
        tokio::time::sleep(Duration::from_secs(30)).await;

        match simulator.prossima_posizione(30.0) {
            Some(pos) => {
                last_pos = (pos.lat, pos.lon);
                add_position(
                    POSITIONS_CSV,
                    user.id,
                    &user.email,
                    pos.lat,
                    pos.lon,
                    &mut last_pos_time,
                )?;
                println!("[{}] {:?}", simulator.id_veicolo(), pos);
            }
            None => {
                let choice = {
                    let mut rng = rng();
                    rng.random_range(1..=15)
                };

                match choice {
                    1 => {
                        println!("Destination reached: {}", destination_name);
                        add_message(
                            MESSAGES_CSV,
                            user.id,
                            &user.email,
                            "destinazione raggiunta",
                            &mut last_msg_time,
                        )?;

                        let next_kebab = {
                            let mut rng = rng();
                            kebab_shops.choose(&mut rng).unwrap().clone()
                        };

                        start_next_leg(
                            user.clone(),
                            destination,
                            (next_kebab.lat, next_kebab.lon),
                            next_kebab.name.clone(),
                            kebab_shops.clone(),
                        );

                        return Ok(());
                    }
                    2 => {
                        add_message(
                            MESSAGES_CSV,
                            user.id,
                            &user.email,
                            "pausa di 120 secondi",
                            &mut last_msg_time,
                        )?;
                        tokio::time::sleep(Duration::from_secs(120)).await;
                    }
                    _ => {
                        add_position(
                            POSITIONS_CSV,
                            user.id,
                            &user.email,
                            last_pos.0,
                            last_pos.1,
                            &mut last_pos_time,
                        )?;
                    }
                }
            }
        }
    }
}

fn start_next_leg(
    user: User,
    origin: (f64, f64),
    destination: (f64, f64),
    destination_name: String,
    kebab_shops: Vec<KebabShop>,
) {
    let user_id = user.id;
    tokio::spawn(async move {
        match login_user(&user.email).await {
            Ok(token) => {
                if let Err(e) = simulate_user_movement(
                    user,
                    origin,
                    destination,
                    destination_name,
                    kebab_shops,
                    token,
                ).await {
                    eprintln!("Next leg error for user {}: {}", user_id, e);
                }
            }
            Err(e) => {
                eprintln!("Login error for user {}: {}", user_id, e);
            }
        }
    });
}

#[derive(Debug, Deserialize, Clone)]
struct KebabShop {
    name: String,
    lat: f64,
    lon: f64,
}

fn read_csv() -> Result<Vec<KebabShop>, Box<dyn std::error::Error + Send + Sync>> {
    let mut rdr = Reader::from_path("kebab_torino_google.csv")?;
    let mut shops: Vec<KebabShop> = Vec::new();
    for result in rdr.deserialize() {
        let shop: KebabShop = result?;
        shops.push(shop);
    }
    Ok(shops)
}

// ===================================================================
// CSV records now include email
// ===================================================================

#[derive(Debug, Clone, Serialize)]
pub struct PositionRecord {
    pub user_id: i64,
    pub email: String,
    pub lat: f64,
    pub lon: f64,
    pub elapsed_from_start_ms: u64,
    pub elapsed_from_last_ms: u64,
}

#[derive(Debug, Clone, Serialize)]
pub struct MessageRecord {
    pub user_id: i64,
    pub email: String,
    pub message: String,
    pub elapsed_from_start_ms: u64,
    pub elapsed_from_last_ms: u64,
}

pub fn init_csv_files(
    positions_path: &str,
    messages_path: &str,
) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    create_if_missing(positions_path)?;
    create_if_missing(messages_path)?;
    Ok(())
}

fn create_if_missing(path: &str) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    if !Path::new(path).exists() {
        std::fs::File::create(path)?;
    }
    Ok(())
}

fn append_record<T: Serialize>(
    path: &str,
    record: &T,
) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    let file_is_empty = !Path::new(path).exists() || std::fs::metadata(path)?.len() == 0;
    let file = OpenOptions::new().create(true).append(true).open(path)?;
    let mut wtr = WriterBuilder::new()
        .has_headers(file_is_empty)
        .from_writer(file);
    wtr.serialize(record)?;
    wtr.flush()?;
    Ok(())
}

pub fn add_position(
    path: &str,
    user_id: i64,
    email: &str,
    lat: f64,
    lon: f64,
    last_time: &mut Instant,
) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    let now = Instant::now();
    let elapsed_from_start = now.duration_since(*PROGRAM_START).as_millis() as u64;
    let elapsed_from_last = now.duration_since(*last_time).as_millis() as u64;
    *last_time = now;

    let record = PositionRecord {
        user_id,
        email: email.to_string(),
        lat,
        lon,
        elapsed_from_start_ms: elapsed_from_start,
        elapsed_from_last_ms: elapsed_from_last,
    };
    append_record(path, &record)
}

pub fn add_message(
    path: &str,
    user_id: i64,
    email: &str,
    message: &str,
    last_time: &mut Instant,
) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    let now = Instant::now();
    let elapsed_from_start = now.duration_since(*PROGRAM_START).as_millis() as u64;
    let elapsed_from_last = now.duration_since(*last_time).as_millis() as u64;
    *last_time = now;

    let record = MessageRecord {
        user_id,
        email: email.to_string(),
        message: message.to_string(),
        elapsed_from_start_ms: elapsed_from_start,
        elapsed_from_last_ms: elapsed_from_last,
    };
    append_record(path, &record)
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    let kebab_shops = read_csv()?;

    if kebab_shops.is_empty() {
        return Err(format!(
            "kebab_torino_google.csv was read correctly but contains no data rows. \
             Make sure the file exists in the folder from which you run `cargo run` \
             (same folder as Cargo.toml) and that it contains at least one row \
             besides the header."
        ).into());
    }

    let users = get_users().await?;

    if users.is_empty() {
        return Err("no users returned by GET /api/users: create some users first before starting the simulation".into());
    }

    init_csv_files(POSITIONS_CSV, MESSAGES_CSV)?;

    let mut handles = Vec::new();

    for user in users.into_iter() {
        let mut rng = rng();

        let kebab = kebab_shops.choose(&mut rng)
            .expect("kebab_shops should not be empty at this point, checked above");

        let dest = kebab_shops.choose(&mut rng)
            .expect("kebab_shops should not be empty at this point, checked above");

        let token = login_user(&user.email).await?;

        let handle = tokio::spawn(simulate_user_movement(
            user.clone(),
            (kebab.lat, kebab.lon),
            (dest.lat, dest.lon),
            dest.name.clone(),
            kebab_shops.clone(),
            token,
        ));
        handles.push(handle);
    }

    let results = join_all(handles).await;
    for r in results {
        match r {
            Err(e) => eprintln!("task panicked: {:?}", e),
            Ok(Err(e)) => eprintln!("task returned error: {:?}", e),
            Ok(Ok(())) => {}
        }
    }

    Ok(())
}