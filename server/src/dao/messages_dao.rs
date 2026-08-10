use crate::database::connection::SharedDb;
use crate::models::Message;
use rusqlite::{params, Result, Row};

fn row_to_message(row: &Row) -> Result<Message> {
    Ok(Message {
        id: row.get("id")?,
        sender_id: row.get("sender_id")?,
        recipient_id: row.get("recipient_id")?,
        content: row.get("content")?,
        sent_at: row.get("sent_at")?,
    })
}

pub fn insert_message(db: &SharedDb, sender_id: Option<i64>, recipient_id: Option<i64>, content: &str,) -> Result<i64> {
    let conn = db.lock().unwrap();
    conn.execute(
        "INSERT INTO messages (sender_id, recipient_id, content)
         VALUES (?1, ?2, ?3)",
        params![sender_id, recipient_id, content],
    )?;
    Ok(conn.last_insert_rowid())
}

pub fn get_broadcast_messages(db: &SharedDb, limit: i64) -> Result<Vec<Message>> {
    let conn = db.lock().unwrap();
    let mut stmt = conn.prepare(
        "SELECT id, sender_id, recipient_id, content, sent_at FROM messages
         WHERE sender_id IS NULL AND recipient_id IS NULL
         ORDER BY sent_at DESC LIMIT ?1",
    )?;
    let rows = stmt.query_map(params![limit], row_to_message)?;
    rows.collect()
}

pub fn get_conversation(db: &SharedDb, user_id: i64, limit: i64) -> Result<Vec<Message>> {
    let conn = db.lock().unwrap();
    let mut stmt = conn.prepare(
        "SELECT id, sender_id, recipient_id, content, sent_at FROM messages
         WHERE sender_id = ?1 OR recipient_id = ?1
         ORDER BY sent_at DESC LIMIT ?2",
    )?;
    let rows = stmt.query_map(params![user_id, limit], row_to_message)?;
    rows.collect()
}
