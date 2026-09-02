//! src/bin/bake_simulation.rs
//!
//! Simulates user movement and saves positions + messages to CSV.
//! Each row includes user_id AND email so replay can login directly.
//!
//!     cargo run --bin bake_simulation <MINUTES>
//!
//! Example:
//!     cargo run --bin bake_simulation 120   # simulate 2 hours
//!     cargo run --bin bake_simulation 10    # simulate 10 minutes (default: 60)

use chrono::{DateTime, Utc};
use csv::{Reader, WriterBuilder};
use rand::seq::{IndexedRandom, SliceRandom};
use rand::{rng, RngExt};
use reqwest::{Client, Error};
use serde::{Deserialize, Serialize};
use std::fs::OpenOptions;
use std::path::Path;
use std::sync::{Mutex, OnceLock};
use std::time::{Duration, Instant};
use futures::future::join_all;
use serde_json::json;
use serde_json::Value::Null;
use tokio;

#[path = "../osrm.rs"]
mod osrm;
use osrm::VehicleSimulator;

const POSITIONS_CSV: &str = "positions.csv";
const MESSAGES_CSV: &str = "messages.csv";
const TICK_SECONDS: u64 = 30;
const PAUSE_SECONDS: u64 = 120;

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
    limit: Option<u32>,
    offset: Option<u32>,
}

#[derive(Deserialize, Debug)]
pub struct UsersResponse {
    pub users: Vec<User>,
    pub has_next_page: bool,
}

fn build_http_client() -> Client {
    Client::builder()
        .danger_accept_invalid_certs(true)
        .build()
        .expect("failed to build HTTP client")
}

async fn get_users(num_users: Option<usize>) -> Result<Vec<User>, Error> {
    let client = build_http_client();
    let credentials = ("marco.rossi@example.com", "Password123!");

    let token = client
        .post("https://127.0.0.1:3001/api/login")
        .json(&credentials)
        .send()
        .await?
        .json::<TokenResponse>()
        .await?
        .token;

    let users_option = UsersOption {
        is_admin: false,
        limit: Some(2000),
        offset: None,
    };

    let mut response = client
        .get("https://127.0.0.1:3001/api/users")
        .header("Authorization", format!("Bearer {}", token))
        .query(&users_option)
        .send()
        .await?
        .json::<UsersResponse>()
        .await?;

    let mut users = response.users;

    let mut offset: u32 = 0;
    while response.has_next_page {
        offset += 2000;

        let users_option = UsersOption {
            is_admin: false,
            limit: Some(offset),
            offset: Some(2000 as u32),
        };

        response = client
            .get("https://127.0.0.1:3001/api/users")
            .header("Authorization", format!("Bearer {}", token))
            .query(&users_option)
            .send()
            .await?
            .json::<UsersResponse>()
            .await?;

        users.append(&mut response.users);
    }

    let mut rng = rng();
    users.shuffle(&mut rng);

    let target_user_ids = [10_i64, 11, 12, 13, 14, 15, 16];

    let users: Vec<User> = users
        .into_iter()
        .filter(|user| target_user_ids.contains(&user.id))
        .collect();

    let take: usize = num_users.unwrap_or(users.len()).min(users.len());
    Ok(users[..take].to_vec())
}

async fn simulate_user_movement(
    user: User,
    initial_origin: (f64, f64),
    initial_destination: (f64, f64),
    initial_destination_name: String,
    kebab_shops: Vec<KebabShop>,
    simulation_duration_ms: u64,
) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    let mut current_origin = initial_origin;
    let mut current_destination = initial_destination;
    let mut current_destination_name = initial_destination_name;

    let mut simulated_elapsed_ms: u64 = 0;
    let mut last_pos_time_ms: u64 = 0;
    let mut last_msg_time_ms: u64 = 0;
    let mut last_pos = (0.0, 0.0);

    // Outer loop: each iteration is a new leg (origin -> destination)
    loop {
        let origin_lon_lat = (current_origin.1, current_origin.0);
        let destination_lon_lat = (current_destination.1, current_destination.0);

        let mut simulator = VehicleSimulator::new(
            &user.id.to_string(),
            origin_lon_lat,
            destination_lon_lat,
        ).await?;

        // Inner loop: drive the current leg
        loop {
            if simulated_elapsed_ms >= simulation_duration_ms {
                println!(
                    "[{}] Simulation time limit reached ({} min)",
                    simulator.vehicle_id(),
                    simulation_duration_ms / 60_000
                );
                return Ok(());
            }

            simulated_elapsed_ms += TICK_SECONDS * 1_000;

            match simulator.next_position(TICK_SECONDS as f64) {
                Some(pos) => {
                    last_pos = (pos.lat, pos.lon);
                    add_position(
                        POSITIONS_CSV,
                        user.id,
                        &user.email,
                        pos.lat,
                        pos.lon,
                        simulated_elapsed_ms,
                        &mut last_pos_time_ms,
                    )?;
                    println!(
                        "[{}] {:?}  (sim +{} ms)",
                        simulator.vehicle_id(), pos, simulated_elapsed_ms
                    );
                }
                None => {
                    println!(
                        "[{}] Destination reached: {}  (sim +{} ms)",
                        simulator.vehicle_id(),
                        current_destination_name,
                        simulated_elapsed_ms
                    );

                    let choice = {
                        let mut rng = rng();
                        rng.random_range(1..=100)
                    };

                    match choice {
                        // ~10%: sosta "Fermo" osservabile. La transizione in_movimento -> fermo
                        // scatta dopo 3 min di coordinate invariate: mandiamo quindi 5-6 min
                        // di posizione ferma ogni 30s, così lo stato ha tempo di scattare
                        // E di restare visibile per un po' prima di ripartire.
                        1..=10 => {
                            let stop_ticks = {
                                let mut rng = rng();
                                rng.random_range(10..=12) // 5-6 min (30s * 10-12 tick)
                            };
                            for _ in 0..stop_ticks {
                                simulated_elapsed_ms += TICK_SECONDS * 1_000;
                                add_position(
                                    POSITIONS_CSV,
                                    user.id,
                                    &user.email,
                                    last_pos.0,
                                    last_pos.1,
                                    simulated_elapsed_ms,
                                    &mut last_pos_time_ms,
                                )?;
                            }
                        }
                        // ~5%: disconnessione osservabile. Il timeout di disconnessione è 2 min
                        // di silenzio totale: restiamo in silenzio 4-5 min, così l'utente
                        // risulta "Disconnesso" abbastanza a lungo da essere visibile nel
                        // report/dashboard, non solo per un istante al limite della soglia.
                        11..=15 => {
                            let disconnect_ticks = {
                                let mut rng = rng();
                                rng.random_range(8..=10) // 4-5 min (30s * 8-10 tick, nessun invio)
                            };
                            simulated_elapsed_ms += disconnect_ticks * TICK_SECONDS * 1_000;
                        }
                        // ~85%: riparte subito, nessuna sosta
                        _ => {}
                    }

                    // La nuova gamba parte SEMPRE dall'ultima posizione vera, mai da un
                    // kebab casuale (fix del teleport).
                    let next_kebab = {
                        let mut rng = rng();
                        kebab_shops.choose(&mut rng).unwrap().clone()
                    };
                    current_origin = last_pos;
                    current_destination = (next_kebab.lat, next_kebab.lon);
                    current_destination_name = next_kebab.name.clone();
                    break;
                }
            }
        }
    }
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

// =======================================================================
// CSV records now include email
// =======================================================================

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
    // IMPORTANTE: ogni run di bake_simulation deve ripartire da file puliti.
    // Se non si azzerano qui, un secondo run (con destinazioni casuali diverse)
    // si accumula sulle righe del run precedente. `replay.rs` raggruppa gli
    // eventi solo per user_id e li ordina per offset_ms, quindi due run diversi
    // mescolati insieme vengono rispediti via MQTT quasi simultaneamente pur
    // rappresentando due percorsi scollegati: il sintomo è una posizione che
    // "teletrasporta" l'utente da una parte all'altra della città in pochi
    // millisecondi (visto nel report admin).
    truncate_file(positions_path)?;
    truncate_file(messages_path)?;
    Ok(())
}

fn truncate_file(path: &str) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    // File::create tronca il file se esiste già, o lo crea se non esiste.
    std::fs::File::create(path)?;
    Ok(())
}

/// Lock globale che serializza TUTTE le scritture sui file CSV.
///
/// `simulate_user_movement` gira in un `tokio::spawn` per ogni utente, e più
/// task scrivono nello stesso file (`positions.csv` / `messages.csv`)
/// concorrentemente. Senza questo lock, il controllo "il file è vuoto?" +
/// apertura + scrittura dell'header non è atomico: più task possono vedere
/// il file ancora vuoto nello stesso istante e scrivere l'header ciascuno,
/// producendo un CSV con più header intervallati a righe di dati (che poi
/// manda in errore `replay.rs` con un ParseIntError sulla colonna user_id).
fn csv_write_lock() -> &'static Mutex<()> {
    static LOCK: OnceLock<Mutex<()>> = OnceLock::new();
    LOCK.get_or_init(|| Mutex::new(()))
}

fn append_record<T: Serialize>(
    path: &str,
    record: &T,
) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    // Tiene il lock per l'intera sezione critica: controllo "file vuoto?",
    // apertura in append e scrittura, così due task non possono mai
    // decidere entrambi di scrivere l'header.
    let _guard = csv_write_lock().lock().unwrap();

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
    simulated_now_ms: u64,
    last_time_ms: &mut u64,
) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    let elapsed_from_last = simulated_now_ms.saturating_sub(*last_time_ms);
    *last_time_ms = simulated_now_ms;

    let record = PositionRecord {
        user_id,
        email: email.to_string(),
        lat,
        lon,
        elapsed_from_start_ms: simulated_now_ms,
        elapsed_from_last_ms: elapsed_from_last,
    };
    append_record(path, &record)
}

pub fn add_message(
    path: &str,
    user_id: i64,
    email: &str,
    message: &str,
    simulated_now_ms: u64,
    last_time_ms: &mut u64,
) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    let elapsed_from_last = simulated_now_ms.saturating_sub(*last_time_ms);
    *last_time_ms = simulated_now_ms;

    let record = MessageRecord {
        user_id,
        email: email.to_string(),
        message: message.to_string(),
        elapsed_from_start_ms: simulated_now_ms,
        elapsed_from_last_ms: elapsed_from_last,
    };
    append_record(path, &record)
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    // ------------------------------------------------------------------
    // CLI: first positional argument = simulation duration in minutes
    // ------------------------------------------------------------------
    let simulation_minutes: u64 = std::env::args()
        .nth(1)
        .and_then(|s| s.parse().ok())
        .unwrap_or(60);
    let simulation_duration_ms = simulation_minutes * 60_000;

    let num_users: Option<usize> = std::env::args()
        .nth(2)
        .and_then(|s| s.parse().ok());

    println!(
        "=== Fleet Simulation ===\nDuration: {} minutes ({} ms simulated)\n",
        simulation_minutes, simulation_duration_ms
    );

    let real_start = Instant::now();

    let kebab_shops = read_csv()?;

    if kebab_shops.is_empty() {
        return Err(format!(
            "kebab_torino_google.csv was read correctly but contains no data rows. \
             Make sure the file exists in the folder from which you run `cargo run` \
             (same folder as Cargo.toml) and that it contains at least one row \
             besides the header."
        ).into());
    }

    let users = get_users(num_users).await?;

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

        let handle = tokio::spawn(simulate_user_movement(
            user.clone(),
            (kebab.lat, kebab.lon),
            (dest.lat, dest.lon),
            dest.name.clone(),
            kebab_shops.clone(),
            simulation_duration_ms,
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

    let real_elapsed = real_start.elapsed();
    println!(
        "\n=== Simulation complete ===\nSimulated: {} min  |  Real time: {:?}",
        simulation_minutes, real_elapsed
    );

    Ok(())
}