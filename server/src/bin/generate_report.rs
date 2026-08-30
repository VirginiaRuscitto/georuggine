// src/bin/generate_report.rs

#[path = "../dao/mod.rs"]
mod dao;
#[path = "../database/mod.rs"]
mod database;
#[path = "../models.rs"]
mod models;

use dao::{movement_sessions_dao, position_log_dao};
use database::connection::{shared_connection, SharedDb};
use models::{MovementState, MovementSession, Position, UserState};

use std::collections::HashMap;
use std::env;
use std::fs::File;
use std::path::PathBuf;
use std::sync::{Arc, Mutex};

use chrono::{DateTime, TimeZone, Utc};
use csv::ReaderBuilder;

const DEFAULT_STALE_AFTER_SECS: i64 = 180;
const DEFAULT_DISCONNECT_AFTER_SECS: i64 = 120;
pub const COORD_EPSILON: f64 = 0.0001;

/// Stato runtime per il replay, equivalente a UserSession in handler.rs
#[derive(Debug)]
struct ReplaySession {
    pub state: UserState,
    pub last_seen_at: DateTime<Utc>,
    pub last_position: Option<Position>,
    pub last_coord_change_at: DateTime<Utc>,
    pub last_change_at: DateTime<Utc>,
    pub last_message_at: Option<DateTime<Utc>>,
}

impl ReplaySession {
    pub fn new(now: DateTime<Utc>) -> Self {
        Self {
            state: UserState::Stopped,
            last_seen_at: now,
            last_position: None,
            last_coord_change_at: now,
            last_change_at: now,
            last_message_at: None,
        }
    }
}

#[derive(Debug)]
struct CsvRow {
    user_id: i64,
    email: String,
    lat: f64,
    lon: f64,
    timestamp_ms: i64,
    interval_ms: i64,
}

fn parse_csv(path: &PathBuf) -> Vec<CsvRow> {
    let file = File::open(path).expect("impossibile aprire il CSV");
    let mut rdr = ReaderBuilder::new()
        .has_headers(true)
        .delimiter(b',')
        .from_reader(file);

    let mut rows = Vec::new();
    for result in rdr.records() {
        let record = result.expect("errore lettura riga CSV");
        let row = CsvRow {
            user_id: record[0].parse().expect("user_id non valido"),
            email: record[1].to_string(),
            lat: record[2].parse().expect("lat non valido"),
            lon: record[3].parse().expect("lon non valido"),
            timestamp_ms: record[4].parse().expect("timestamp_ms non valido"),
            interval_ms: record[5].parse().expect("interval_ms non valido"),
        };
        rows.push(row);
    }
    rows
}

fn coords_changed(old_pos: Option<&Position>, new_pos: &Position) -> bool {
    match old_pos {
        Some(prev) => {
            (prev.lat - new_pos.lat).abs() > COORD_EPSILON
                || (prev.lon - new_pos.lon).abs() > COORD_EPSILON
        }
        None => false,
    }
}

fn check_state_transition(
    moved: bool,
    last_coord_change_at: DateTime<Utc>,
    now: DateTime<Utc>,
    stale_after: i64,
) -> Option<(UserState, DateTime<Utc>)> {
    if moved {
        Some((UserState::Moving, now))
    } else if now.signed_duration_since(last_coord_change_at).num_seconds() >= stale_after {
        Some((UserState::Stopped, last_coord_change_at))
    } else {
        None
    }
}

fn ensure_active_session(
    active: &Arc<Mutex<HashMap<i64, ReplaySession>>>,
    db: &SharedDb,
    user_id: i64,
    now: DateTime<Utc>,
    disconnect_after: i64,
) {
    let stale_last_seen = {
        let users = active.lock().unwrap();
        users.get(&user_id).and_then(|s| {
            let elapsed = now.signed_duration_since(s.last_seen_at).num_seconds();
            (elapsed >= disconnect_after).then_some(s.last_seen_at)
        })
    };

    if let Some(last_seen_at) = stale_last_seen {
        active.lock().unwrap().remove(&user_id);
        if let Err(e) =
            movement_sessions_dao::close_movement_session_for_user(db, user_id, last_seen_at)
        {
            eprintln!("errore chiusura sessione stale per user {user_id}: {e}");
        }
    }

    let is_new = {
        let mut users = active.lock().unwrap();
        let is_new = !users.contains_key(&user_id);
        users
            .entry(user_id)
            .or_insert_with(|| ReplaySession::new(now));
        is_new
    };

    if is_new {
        if let Err(e) =
            movement_sessions_dao::close_all_open_sessions_for_user(db, user_id, now)
        {
            eprintln!("errore chiusura sessioni residue per user {user_id}: {e}");
        }
        if let Err(e) = movement_sessions_dao::insert_movement_session(
            db,
            user_id,
            MovementState::Stopped,
            now,
        ) {
            eprintln!("errore creazione sessione per user {user_id}: {e}");
        }
    }
}

fn update_session_position(
    active: &Arc<Mutex<HashMap<i64, ReplaySession>>>,
    user_id: i64,
    new_pos: Position,
    now: DateTime<Utc>,
    stale_after: i64,
) -> Option<(UserState, DateTime<Utc>)> {
    let mut users = active.lock().unwrap();
    let session = users
        .get_mut(&user_id)
        .expect("sessione creata da ensure_active_session");

    session.last_seen_at = now;

    let moved = coords_changed(session.last_position.as_ref(), &new_pos);
    if moved {
        session.last_coord_change_at = now;
    }

    let mut transition = None;
    if let Some((new_state, transition_at)) =
        check_state_transition(moved, session.last_coord_change_at, now, stale_after)
    {
        if session.state != new_state {
            session.state = new_state;
            session.last_change_at = transition_at;
            transition = Some((new_state, transition_at));
        }
    }

    session.last_position = Some(new_pos);
    transition
}

fn validate_coordinates(lat: f64, lon: f64) -> Result<(), &'static str> {
    if !lat.is_finite() || !lon.is_finite() {
        return Err("Coordinate non valide");
    }
    if !(-90.0..=90.0).contains(&lat) || !(-180.0..=180.0).contains(&lon) {
        return Err("Coordinate fuori range");
    }
    Ok(())
}

fn run_replay(
    db: SharedDb,
    csv: PathBuf,
    start: DateTime<Utc>,
    stale_after: i64,
    disconnect_after: i64,
) {
    let mut rows = parse_csv(&csv);
    rows.sort_by_key(|r| r.timestamp_ms);

    let start_ts = start.timestamp_millis();
    let active: Arc<Mutex<HashMap<i64, ReplaySession>>> =
        Arc::new(Mutex::new(HashMap::new()));

    for row in &rows {
        let real_time = Utc
            .timestamp_millis_opt(start_ts + row.timestamp_ms)
            .single()
            .expect("timestamp non valido");

        if let Err(msg) = validate_coordinates(row.lat, row.lon) {
            eprintln!("riga scartata per user {}: {msg}", row.user_id);
            continue;
        }

        ensure_active_session(&active, &db, row.user_id, real_time, disconnect_after);

        let new_pos = Position {
            lat: row.lat,
            lon: row.lon,
            recorded_at: real_time,
        };

        if let Err(e) = position_log_dao::insert_position(
            &db,
            row.user_id,
            row.lat,
            row.lon,
            real_time,
        ) {
            eprintln!("errore salvataggio posizione per user {}: {e}", row.user_id);
            continue;
        }

        let transition = update_session_position(
            &active,
            row.user_id,
            new_pos,
            real_time,
            stale_after,
        );

        if let Some((new_state, transition_at)) = transition {
            if let Err(e) = movement_sessions_dao::transition_session(
                &db,
                row.user_id,
                MovementState::from(new_state),
                transition_at,
            ) {
                eprintln!(
                    "errore transizione stato per user {}: {e}",
                    row.user_id
                );
            }
        }
    }

    // Chiusura finale
    let users_to_close: Vec<(i64, DateTime<Utc>)> = {
        let users = active.lock().unwrap();
        users
            .iter()
            .map(|(&uid, session)| (uid, session.last_seen_at))
            .collect()
    };

    for (user_id, last_seen) in users_to_close {
        if let Err(e) =
            movement_sessions_dao::close_movement_session_for_user(&db, user_id, last_seen)
        {
            eprintln!("errore chiusura sessione finale per user {user_id}: {e}");
        }
    }

    println!("Replay completato. Inserite {} righe.", rows.len());
}

fn main() {
    let args: Vec<String> = env::args().collect();

    if args.len() < 3{
        eprintln!("Uso: cargo run --bin generate_report <db_path> <csv_path> <start_iso> [stale_after] [disconnect_after]");
        eprintln!("Esempio: cargo run --bin generate_report app.db dati.csv 2024-01-15T08:00:00Z");
        std::process::exit(1);
    }

    let csv_path = args[1].clone().into();
    let start = args[2].parse().expect("START non valido (formato RFC3339)");
    let stale_after = args.get(3).and_then(|s| s.parse().ok()).unwrap_or(DEFAULT_STALE_AFTER_SECS);
    let disconnect_after = args.get(4).and_then(|s| s.parse().ok()).unwrap_or(DEFAULT_DISCONNECT_AFTER_SECS);

    let db = shared_connection().expect("errore db");
    run_replay(db, csv_path, start, stale_after, disconnect_after);
}