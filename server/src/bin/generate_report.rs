// src/bin/generate_report.rs

#[path = "../database/mod.rs"]
mod database;
#[path = "../models.rs"]
mod models;

use database::connection::{shared_connection, SharedDb};
use models::{Position, UserState};

use std::collections::HashMap;
use std::env;
use std::fs::File;
use std::path::PathBuf;
use std::sync::{Arc, Mutex};

use chrono::{DateTime, TimeZone, Utc};
use csv::ReaderBuilder;
use rusqlite::params;

const DEFAULT_STALE_AFTER_SECS: i64 = 180;
const DEFAULT_DISCONNECT_AFTER_SECS: i64 = 120;
pub const COORD_EPSILON: f64 = 0.0001;

/// Stato runtime per il replay, equivalente a UserSession in handler.rs
#[derive(Debug, Clone)]
struct ReplaySession {
    pub state: UserState,
    pub last_seen_at: DateTime<Utc>,
    pub last_position: Option<Position>,
    pub last_coord_change_at: DateTime<Utc>,
    pub last_change_at: DateTime<Utc>,
    pub open_session_started_at: Option<DateTime<Utc>>,
    pub open_session_state: Option<String>,
}

impl ReplaySession {
    pub fn new(now: DateTime<Utc>) -> Self {
        Self {
            state: UserState::Stopped,
            last_seen_at: now,
            last_position: None,
            last_coord_change_at: now,
            last_change_at: now,
            open_session_started_at: None,
            open_session_state: None,
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

/// Operazione da eseguire nel DB durante il replay
#[derive(Debug, Clone)]
enum DbOp {
    /// Inserisci nuova sessione aperta: (user_id, state, started_at)
    InsertSession(i64, String, DateTime<Utc>),
    /// Chiudi sessione esistente: (user_id, ended_at)
    CloseSession(i64, DateTime<Utc>),
    /// Inserisci posizione: (user_id, lat, lon, recorded_at)
    InsertPosition(i64, f64, f64, DateTime<Utc>),
    /// Inserisci utente se non esiste: (user_id, email)
    InsertUser(i64, String),
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

    let mut ops: Vec<DbOp> = Vec::new();
    let mut seen_users: HashMap<i64, String> = HashMap::new();

    for row in &rows {
        let real_time = Utc
            .timestamp_millis_opt(start_ts + row.timestamp_ms)
            .single()
            .expect("timestamp non valido");

        if !(-90.0..=90.0).contains(&row.lat) || !(-180.0..=180.0).contains(&row.lon) {
            eprintln!("riga scartata per user {}: coordinate fuori range", row.user_id);
            continue;
        }

        // Traccia utente per INSERT OR IGNORE
        seen_users.entry(row.user_id).or_insert_with(|| row.email.clone());

        // --- Logica ensure_active_session + stale check ---
        let mut users = active.lock().unwrap();

        // Se esiste ma è stale da più di disconnect_after, chiudi e rimuovi
        if let Some(session) = users.get(&row.user_id) {
            let elapsed = real_time.signed_duration_since(session.last_seen_at).num_seconds();
            if elapsed >= disconnect_after {
                // Chiudi sessione aperta
                if let Some(started_at) = session.open_session_started_at {
                    ops.push(DbOp::CloseSession(row.user_id, session.last_seen_at));
                }
                users.remove(&row.user_id);
            }
        }

        // Se nuovo, crea sessione Stopped iniziale
        let is_new = !users.contains_key(&row.user_id);
        let session = users.entry(row.user_id).or_insert_with(|| {
            let s = ReplaySession::new(real_time);
            s
        });

        if is_new {
            // Chiudi eventuali sessioni residue (crash recovery) - fatto nel batch finale
            // Crea nuova sessione Stopped
            ops.push(DbOp::InsertSession(row.user_id, "stopped".to_string(), real_time));
            session.open_session_started_at = Some(real_time);
            session.open_session_state = Some("stopped".to_string());
        }

        session.last_seen_at = real_time;

        let new_pos = Position {
            lat: row.lat,
            lon: row.lon,
            recorded_at: real_time,
        };

        let moved = coords_changed(session.last_position.as_ref(), &new_pos);
        if moved {
            session.last_coord_change_at = real_time;
        }

        // --- Logica transizione stato ---
        if let Some((new_state, transition_at)) =
            check_state_transition(moved, session.last_coord_change_at, real_time, stale_after)
        {
            if session.state != new_state {
                let state_str = match new_state {
                    UserState::Moving => "moving",
                    UserState::Stopped => "stopped",
                    UserState::Disconnected => "disconnected",
                };

                // Chiudi sessione precedente (se aperta)
                if let Some(started_at) = session.open_session_started_at {
                    // ended_at deve essere >= started_at (vincolo DB)
                    // Se la transizione è prima di started_at, usa started_at
                    let ended_at = if transition_at < started_at { started_at } else { transition_at };
                    ops.push(DbOp::CloseSession(row.user_id, ended_at));
                }

                // Apri nuova sessione
                ops.push(DbOp::InsertSession(row.user_id, state_str.to_string(), transition_at));
                session.open_session_started_at = Some(transition_at);
                session.open_session_state = Some(state_str.to_string());
                session.state = new_state;
                session.last_change_at = transition_at;
            }
        }

        session.last_position = Some(new_pos);
        drop(users);

        // Accumula posizione
        ops.push(DbOp::InsertPosition(row.user_id, row.lat, row.lon, real_time));
    }

    // --- Chiusura finale di tutte le sessioni ancora aperte ---
    {
        let users = active.lock().unwrap();
        for (user_id, session) in users.iter() {
            if let Some(started_at) = session.open_session_started_at {
                let ended_at = if session.last_seen_at < started_at { started_at } else { session.last_seen_at };
                ops.push(DbOp::CloseSession(*user_id, ended_at));
            }
        }
    }

    // --- ESECUZIONE IN UN'UNICA TRANSAZIONE ---
    let mut conn = db.lock().unwrap();
    let tx = conn.transaction().expect("errore inizio transazione");

    // 1. Inserisci utenti mancanti
    {
        let mut stmt = tx.prepare("INSERT OR IGNORE INTO users (id, email, created_at) VALUES (?1, ?2, ?3)")
            .expect("errore prepare users");
        for (user_id, email) in &seen_users {
            stmt.execute(params![user_id, email, Utc::now()])
                .expect("errore insert user");
        }
    }

    // 2. Esegui tutte le operazioni accumulate
    let mut insert_session_stmt = tx.prepare(
        "INSERT INTO movement_sessions (user_id, state, started_at) VALUES (?1, ?2, ?3)"
    ).expect("errore prepare insert session");

    let mut close_session_stmt = tx.prepare(
        "UPDATE movement_sessions SET ended_at = ?1
         WHERE user_id = ?2 AND ended_at IS NULL AND started_at <= ?1"
    ).expect("errore prepare close session");

    let mut insert_position_stmt = tx.prepare(
        "INSERT INTO position_log (user_id, lat, lon, recorded_at) VALUES (?1, ?2, ?3, ?4)"
    ).expect("errore prepare insert position");

    for op in &ops {
        match op {
            DbOp::InsertUser(_, _) => {} // già fatto sopra
            DbOp::InsertSession(user_id, state, started_at) => {
                insert_session_stmt.execute(params![user_id, state, started_at])
                    .expect("errore insert session");
            }
            DbOp::CloseSession(user_id, ended_at) => {
                close_session_stmt.execute(params![ended_at, user_id])
                    .expect("errore close session");
            }
            DbOp::InsertPosition(user_id, lat, lon, recorded_at) => {
                insert_position_stmt.execute(params![user_id, lat, lon, recorded_at])
                    .expect("errore insert position");
            }
        }
    }

    drop(insert_session_stmt);
    drop(close_session_stmt);
    drop(insert_position_stmt);

    tx.commit().expect("errore commit transazione");

    let pos_count = ops.iter().filter(|o| matches!(o, DbOp::InsertPosition(_, _, _, _))).count();
    let session_count = ops.iter().filter(|o| matches!(o, DbOp::InsertSession(_, _, _))).count();
    let close_count = ops.iter().filter(|o| matches!(o, DbOp::CloseSession(_, _))).count();

    println!("Replay completato. Inserite {} posizioni, {} sessioni aperte, {} sessioni chiuse.", pos_count, session_count, close_count);
}

fn main() {
    let args: Vec<String> = env::args().collect();

    if args.len() < 3 {
        eprintln!("Uso: cargo run --bin generate_report <csv_path> <start_iso> [stale_after] [disconnect_after]");
        eprintln!("Esempio: cargo run --bin generate_report dati.csv 2024-01-15T08:00:00Z");
        std::process::exit(1);
    }

    let csv_path = args[1].clone().into();
    let start = args[2].parse().expect("START non valido (formato RFC3339)");
    let stale_after = args.get(3).and_then(|s| s.parse().ok()).unwrap_or(DEFAULT_STALE_AFTER_SECS);
    let disconnect_after = args.get(4).and_then(|s| s.parse().ok()).unwrap_or(DEFAULT_DISCONNECT_AFTER_SECS);

    let db = shared_connection().expect("errore db");
    run_replay(db, csv_path, start, stale_after, disconnect_after);
}