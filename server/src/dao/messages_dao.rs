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

pub fn get_conversation(db: &SharedDb, user_a: i64, user_b: i64, limit: i64) -> Result<Vec<Message>> {
    let conn = db.lock().unwrap();
    let mut stmt = conn.prepare(
        "SELECT id, sender_id, recipient_id, content, sent_at 
         FROM messages
         WHERE (sender_id = ?1 AND recipient_id = ?2) 
            OR (sender_id = ?2 AND recipient_id = ?1)
            OR (sender_id = ?1 AND recipient_id IS NULL)  -- anche broadcast inviati da A
            OR (sender_id IS NULL AND recipient_id = ?1)  -- broadcast ricevuti da A
         ORDER BY sent_at DESC LIMIT ?3",
    )?;
    
    let rows = stmt.query_map(params![user_a, user_b, limit], row_to_message)?;
    rows.collect()
}

//se `with` è Some(user_id) restituisce i messaggi che coinvolgono quell'utente
//se è None restituisce i messaggi broadcast (recipient_id IS NULL).
pub fn get_messages(db: &SharedDb, with: Option<i64>, limit: i64) -> Result<Vec<Message>> {
    let conn = db.lock().unwrap();
    let mut stmt = match with {
        Some(_) => conn.prepare(
            "SELECT id, sender_id, recipient_id, content, sent_at FROM messages
             WHERE sender_id = ?1 OR recipient_id = ?1
             ORDER BY sent_at DESC LIMIT ?2",
        )?,
        None => conn.prepare(
            "SELECT id, sender_id, recipient_id, content, sent_at FROM messages
             WHERE recipient_id IS NULL
             ORDER BY sent_at DESC LIMIT ?1",
        )?,
    };

    let rows = match with {
        Some(user_id) => stmt.query_map(params![user_id, limit], row_to_message)?,
        None => stmt.query_map(params![limit], row_to_message)?,
    };
    rows.collect()
}
