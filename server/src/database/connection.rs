use rusqlite::{Connection};

pub fn get_connection() -> Result<Connection> {
    let conn = Connection::open("src/database/georuggine.db")?;
    conn.execute_batch("PRAGMA foreign_keys = ON;")?;
    Ok(conn)
}
