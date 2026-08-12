use crate::database::connection::SharedDb;
use crate::models::Message;
use rusqlite::{params, Result, Row};

const MAX_MESSAGE_LEN: usize = 1000;

fn row_to_message(row: &Row) -> Result<Message> {
    Ok(Message {
        id: row.get("id")?,
        sender_id: row.get("sender_id")?,
        recipient_id: row.get("recipient_id")?,
        content: row.get("content")?,
        sent_at: row.get("sent_at")?,
    })
}

pub fn validate_content(content: &str) -> std::result::Result<&str, &'static str> {
    let c = content.trim();
    if c.is_empty() {
        return Err("Il contenuto del messaggio non può essere vuoto");
    }
    if c.chars().count() > MAX_MESSAGE_LEN {
        return Err("Messaggio troppo lungo (max 1000 caratteri)");
    }
    Ok(c)
}

pub fn insert_message(db: &SharedDb, sender_id: Option<i64>, recipient_id: Option<i64>, content: &str) -> Result<i64> {
    let conn = db.lock().unwrap();
    conn.execute(
        "INSERT INTO messages (sender_id, recipient_id, content) VALUES (?1, ?2, ?3)",
        params![sender_id, recipient_id, content.trim()],
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
    let mut messages: Vec<Message> = rows.collect::<Result<Vec<_>>>()?;
    messages.reverse();
    Ok(messages)
}

/// Solo i messaggi diretti scambiati con `user_id` (admin<->utente), senza broadcast.
/// Usata dalla vista admin "conversazione con utente X".
pub fn get_direct_conversation(db: &SharedDb, user_id: i64, limit: i64) -> Result<Vec<Message>> {
    let conn = db.lock().unwrap();
    let mut stmt = conn.prepare(
        "SELECT id, sender_id, recipient_id, content, sent_at FROM messages
         WHERE sender_id = ?1 OR recipient_id = ?1
         ORDER BY sent_at DESC LIMIT ?2",
    )?;
    let rows = stmt.query_map(params![user_id, limit], row_to_message)?;
    let mut messages: Vec<Message> = rows.collect::<Result<Vec<_>>>()?;
    messages.reverse();
    Ok(messages)
}

/// Messaggi diretti con `user_id` UNITI ai broadcast, ordinati insieme cronologicamente.
/// Usata dalla vista dell'utente normale, che deve vedere tutto in un'unica chiamata.
pub fn get_conversation_with_broadcasts(db: &SharedDb, user_id: i64, limit: i64) -> Result<Vec<Message>> {
    let conn = db.lock().unwrap();
    let mut stmt = conn.prepare(
        "SELECT id, sender_id, recipient_id, content, sent_at FROM messages
         WHERE (sender_id = ?1 OR recipient_id = ?1)
            OR (sender_id IS NULL AND recipient_id IS NULL)
         ORDER BY sent_at DESC LIMIT ?2",
    )?;
    let rows = stmt.query_map(params![user_id, limit], row_to_message)?;
    let mut messages: Vec<Message> = rows.collect::<Result<Vec<_>>>()?;
    messages.reverse();
    Ok(messages)
}