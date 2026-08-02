use crate::database::connection::SharedDb;
use crate::models::{NewUser, User};
use rusqlite::{params, OptionalExtension, Result, Row};

fn row_to_user(row: &Row) -> Result<User> {
    Ok(User {
        id: row.get("id")?,
        name: row.get("name")?,
        surname: row.get("surname")?,
        email: row.get("email")?,
        created_at: row.get("created_at")?, //DateTime<Utc> letto direttamente
    })
}

pub fn insert_user(db: &SharedDb, new_user: &NewUser) -> Result<i64> { //la data viene inserita di default nel db
    let conn = db.lock().unwrap();
    conn.execute(
        "INSERT INTO users (name, surname, email, password_hash)
        VALUES (?1, ?2, ?3, ?4)",
        params![
            &new_user.name,
            &new_user.surname,
            &new_user.email,
            &new_user.password_hash,
        ],
    )?;
    Ok(conn.last_insert_rowid())
}

pub fn get_credentials_by_email(db: &SharedDb, email: &str) -> Result<Option<(i64, String)>> {
    let conn = db.lock().unwrap();
    conn.query_row(
        "SELECT id, password_hash FROM users WHERE email = ?1",
        params![email],
        |row| {
            Ok((row.get(0)?, row.get(1)?))
        },
    )
    .optional()
}

pub fn get_user_by_id(db: &SharedDb, user_id: i64) -> Result<Option<User>> {
    let conn = db.lock().unwrap();
    conn.query_row(
        "SELECT id, name, surname, email, created_at
         FROM users WHERE id = ?1",
        params![user_id],
        row_to_user,
    )
    .optional()
}

pub fn email_exists(db: &SharedDb, email: &str) -> Result<bool> {
    let conn = db.lock().unwrap();
    let count: i32 = conn.query_row(
        "SELECT COUNT(*) FROM users WHERE email = ?1",
        params![email],
        |row| row.get(0),
    )?;
    Ok(count > 0)
}

pub fn get_all_users(db: &SharedDb) -> Result<Vec<User>> {
    let conn = db.lock().unwrap();
    let mut stmt = conn.prepare(
        "SELECT id, name, surname, email, created_at
         FROM users ORDER BY id",
    )?;
    let rows = stmt.query_map([], row_to_user)?;
    rows.collect()
}