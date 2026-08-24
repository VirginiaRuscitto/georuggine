use crate::database::connection::SharedDb;
use crate::models::{MovementSession, MovementState};
use chrono::{DateTime, Utc};
use rusqlite::{params, Result, Row};

fn row_to_session(row: &Row) -> Result<MovementSession> {
    Ok(MovementSession {
        id: row.get("id")?,
        user_id: row.get("user_id")?,
        state: row.get("state")?,
        started_at: row.get("started_at")?,
        ended_at: row.get("ended_at")?,
    })
}

pub fn insert_movement_session(db: &SharedDb, user_id: i64, state: MovementState, started_at: DateTime<Utc>) -> Result<i64> {
    let conn = db.lock().unwrap();
    conn.execute(
        "INSERT INTO movement_sessions (user_id, state, started_at) VALUES (?1, ?2, ?3)",
        params![user_id, state, started_at],
    )?;
    Ok(conn.last_insert_rowid())
}

pub fn close_movement_session_for_user(db: &SharedDb, user_id: i64, ended_at: DateTime<Utc>) -> Result<()> {
    let conn = db.lock().unwrap();
    conn.execute(
        "UPDATE movement_sessions SET ended_at = ?1 WHERE user_id = ?2 AND ended_at IS NULL AND started_at <= ?1", //l'ultimo check è per evitare la race condition con la rimozione dell'utente da active users (se in quella finestra arriva una nuova posizione dallo stesso utente, update_session_position lo vede assente dalla mappa e crea una nuova sessione che si potrebbe confondere con quella da eliminare)
        params![ended_at, user_id],
    )?;
    Ok(())
}


pub fn transition_session(db: &SharedDb, user_id: i64, new_state: MovementState, at: DateTime<Utc>) -> Result<i64> {
    let mut conn = db.lock().unwrap();
    let tx = conn.transaction()?;
    tx.execute("UPDATE movement_sessions SET ended_at = ?1 WHERE user_id = ?2 AND ended_at IS NULL", params![at, user_id])?;
    tx.execute("INSERT INTO movement_sessions (user_id, state, started_at) VALUES (?1, ?2, ?3)", params![user_id, new_state, at])?;
    let id = tx.last_insert_rowid();
    tx.commit()?;
    Ok(id)
}

pub fn get_sessions_in_range(db: &SharedDb, user_id: i64, from: DateTime<Utc>, to: DateTime<Utc>,) -> Result<Vec<MovementSession>> {
    let conn = db.lock().unwrap();
    let mut stmt = conn.prepare(
        "SELECT id, user_id, state, started_at, ended_at FROM movement_sessions
         WHERE user_id = ?1
           AND started_at < ?3
           AND (ended_at IS NULL OR ended_at > ?2) 
         ORDER BY started_at ASC",
    )?;
    let rows = stmt.query_map(params![user_id, from, to], row_to_session)?;
    rows.collect()
}

pub fn get_open_session_for_user(db: &SharedDb, user_id: i64) -> Result<Option<MovementSession>> {
    let conn = db.lock().unwrap();
    let mut stmt = conn.prepare(
        "SELECT id, user_id, state, started_at, ended_at 
         FROM movement_sessions
         WHERE user_id = ?1 AND ended_at IS NULL
         ORDER BY started_at DESC
         LIMIT 1",
    )?;
    let mut rows = stmt.query_map(params![user_id], row_to_session)?;
    Ok(rows.next().transpose()?)
}

/// Chiude TUTTE le sessioni aperte (ended_at IS NULL) per un utente,
/// settando ended_at al timestamp fornito. Usata al riavvio del server
/// per il crash recovery e da ensure_active_session per pulizia preventiva.
pub fn close_all_open_sessions_for_user(db: &SharedDb, user_id: i64, ended_at: DateTime<Utc>) -> Result<usize> {
    let conn = db.lock().unwrap();
    let affected = conn.execute(
        "UPDATE movement_sessions SET ended_at = ?1 WHERE user_id = ?2 AND ended_at IS NULL",
        params![ended_at, user_id],
    )?;
    Ok(affected)
}

/// Chiude TUTTE le sessioni aperte nel database (crash recovery allo startup).
/// Restituisce il numero di sessioni chiuse.
pub fn close_all_open_sessions(db: &SharedDb, ended_at: DateTime<Utc>) -> Result<usize> {
    let conn = db.lock().unwrap();
    let affected = conn.execute(
        "UPDATE movement_sessions SET ended_at = ?1 WHERE ended_at IS NULL",
        params![ended_at],
    )?;
    Ok(affected)
}