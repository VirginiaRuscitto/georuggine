use crate::database::connection::SharedDb;
use crate::models::Position;
use chrono::{DateTime, Utc};
use rusqlite::{params, OptionalExtension, Result, Row};
 
fn row_to_position(row: &Row) -> Result<Position> {
    Ok(Position {
        lat: row.get("lat")?,
        lon: row.get("lon")?,
        recorded_at: row.get("recorded_at")?,
    })
}

pub fn insert_position(db: &SharedDb, user_id: i64, lat: f64, lon: f64, recorded_at: DateTime<Utc>) -> Result<i64> {
    let conn = db.lock().unwrap();
    conn.execute(
        "INSERT INTO position_log (user_id, lat, lon, recorded_at) VALUES (?1, ?2, ?3, ?4)",
        params![user_id, lat, lon, recorded_at],
    )?;
    Ok(conn.last_insert_rowid())
}

pub fn get_positions_in_range(db: &SharedDb, user_id: i64, from: DateTime<Utc>, to: DateTime<Utc>) -> Result<Vec<Position>> {
    let conn = db.lock().unwrap();
    let mut stmt = conn.prepare(
        "SELECT lat, lon, recorded_at FROM position_log
         WHERE user_id = ?1 AND recorded_at >= ?2 AND recorded_at < ?3
         ORDER BY recorded_at ASC",
    )?;
    let rows = stmt.query_map(params![user_id, from, to], row_to_position)?;
    rows.collect()
}