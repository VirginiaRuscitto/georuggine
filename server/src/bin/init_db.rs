use rusqlite::Connection;
use std::fs;

fn main() -> rusqlite::Result<()> {
    fs::create_dir_all("src/database").expect("impossibile creare la cartella database");

    let conn = Connection::open("src/database/georuggine.db")?;
    conn.execute_batch("PRAGMA foreign_keys = ON;")?;

    conn.execute_batch(
        r#"
        DROP TABLE IF EXISTS messages;
        DROP TABLE IF EXISTS movement_sessions;
        DROP TABLE IF EXISTS position_log;
        DROP TABLE IF EXISTS users;

        CREATE TABLE users (
            id INTEGER PRIMARY KEY AUTOINCREMENT,
            name TEXT NOT NULL,
            surname TEXT NOT NULL,
            email TEXT NOT NULL UNIQUE,
            is_admin INTEGER NOT NULL DEFAULT 0 CHECK (is_admin IN (0, 1)),
            password_hash TEXT NOT NULL,
            created_at TEXT NOT NULL DEFAULT (datetime('now'))
        );

        CREATE TABLE position_log (
            id INTEGER PRIMARY KEY AUTOINCREMENT,
            user_id INTEGER NOT NULL,
            lat REAL NOT NULL,
            lon REAL NOT NULL,
            recorded_at TEXT NOT NULL DEFAULT (datetime('now')),
            FOREIGN KEY (user_id) REFERENCES users(id) ON DELETE CASCADE,
            CHECK (lat BETWEEN -90 AND 90),
            CHECK (lon BETWEEN -180 AND 180)
        );

        CREATE INDEX idx_position_log_user_time
            ON position_log(user_id, recorded_at);

        CREATE TABLE movement_sessions (
            id INTEGER PRIMARY KEY AUTOINCREMENT,
            user_id INTEGER NOT NULL,
            state TEXT NOT NULL CHECK(state IN ('stopped','moving')),
            started_at TEXT NOT NULL,
            ended_at TEXT,
            FOREIGN KEY (user_id) REFERENCES users(id) ON DELETE CASCADE,
            CHECK (ended_at IS NULL OR ended_at >= started_at)
        );

        --sender_id = NULL, recipient_id = NULL → broadcast dal server
        --sender_id = X, recipient_id = NULL → messaggio dell'utente X al server
        --sender_id = NULL, recipient_id = X → messaggio dal server all'utente X 
        CREATE TABLE messages (
            id INTEGER PRIMARY KEY AUTOINCREMENT,
            sender_id INTEGER,
            recipient_id INTEGER,
            content TEXT NOT NULL,
            sent_at TEXT NOT NULL DEFAULT (datetime('now')),
            FOREIGN KEY (sender_id) REFERENCES users(id) ON DELETE CASCADE,
            FOREIGN KEY (recipient_id) REFERENCES users(id) ON DELETE CASCADE
        );
        "#,
    )?;

    println!("Database creato in src/database/georuggine.db");
    Ok(())
}