use rusqlite::{Connection, Result};
use std::sync::{Arc, Mutex};

//alias per evitare di scrivere ogni volta Arc<Mutex<Connection>>
pub type SharedDb = Arc<Mutex<Connection>>;

fn get_connection() -> Result<Connection> {
    let conn = Connection::open("src/database/georuggine.db")?;
    conn.execute_batch("PRAGMA foreign_keys = ON;")?;
    Ok(conn)
}

pub fn shared_connection() -> Result<SharedDb> {
    let conn = get_connection()?;
    Ok(Arc::new(Mutex::new(conn)))
}