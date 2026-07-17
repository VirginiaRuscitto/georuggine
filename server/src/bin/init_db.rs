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
            username TEXT NOT NULL UNIQUE,
            name TEXT NOT NULL,
            surname TEXT NOT NULL,
            email TEXT NOT NULL UNIQUE,
            password TEXT NOT NULL,
            salt TEXT NOT NULL,
            created_at TEXT NOT NULL DEFAULT (datetime('now'))
        );

        CREATE TABLE position_log (
            id INTEGER PRIMARY KEY AUTOINCREMENT,
            user_id INTEGER NOT NULL,
            lat REAL NOT NULL,
            lon REAL NOT NULL,
            recorded_at TEXT NOT NULL DEFAULT (datetime('now')),
            FOREIGN KEY (user_id) REFERENCES users(id),
            CHECK (lat BETWEEN -90 AND 90),
            CHECK (lon BETWEEN -180 AND 180)
        );

        CREATE INDEX idx_position_log_user_time
            ON position_log(user_id, recorded_at);

        CREATE TABLE movement_sessions (
            id INTEGER PRIMARY KEY AUTOINCREMENT,
            user_id INTEGER NOT NULL,
            state TEXT NOT NULL CHECK(state IN ('fermo','in_movimento')),
            started_at TEXT NOT NULL,
            ended_at TEXT,
            FOREIGN KEY (user_id) REFERENCES users(id),
            CHECK (ended_at IS NULL OR ended_at >= started_at)
        );

        CREATE TABLE messages (
            id INTEGER PRIMARY KEY AUTOINCREMENT,
            sender_id INTEGER,
            recipient_id INTEGER,
            content TEXT NOT NULL,
            sent_at TEXT NOT NULL DEFAULT (datetime('now')),
            FOREIGN KEY (sender_id) REFERENCES users(id),
            FOREIGN KEY (recipient_id) REFERENCES users(id)
        );
        "#,
    )?;

    println!("Database creato in src/database/georuggine.db");
    Ok(())
}