use crate::database::connection::SharedDb;
use crate::models::{NewUser, User};
use rusqlite::{params, OptionalExtension, Result, Row};

fn row_to_user(row: &Row) -> Result<User> {
    Ok(User {
        id: row.get("id")?,
        name: row.get("name")?,
        surname: row.get("surname")?,
        email: row.get("email")?,
        created_at: row.get("created_at")?,
        is_admin: row.get::<_, i64>("is_admin")? != 0,
    })
}

pub fn insert_user(db: &SharedDb, new_user: &NewUser) -> Result<i64> {
    let conn = db.lock().unwrap();
    conn.execute(
        "INSERT INTO users (name, surname, email, password_hash, is_admin)
        VALUES (?1, ?2, ?3, ?4, ?5)",
        params![
            &new_user.name,
            &new_user.surname,
            &new_user.email,
            &new_user.password_hash,
            new_user.is_admin as i64,
        ],
    )?;
    Ok(conn.last_insert_rowid())
}

pub fn get_credentials_by_email(db: &SharedDb, email: &str) -> Result<Option<(i64, String, bool)>> {
    let conn = db.lock().unwrap();
    conn.query_row(
        "SELECT id, password_hash, is_admin FROM users WHERE email = ?1",
        params![email],
        |row| Ok((row.get(0)?, row.get(1)?, row.get::<_, i64>(2)? != 0)),
    )
    .optional()
}

pub fn get_user_by_id(db: &SharedDb, user_id: i64) -> Result<Option<User>> {
    let conn = db.lock().unwrap();
    conn.query_row(
        "SELECT id, name, surname, email, created_at, is_admin
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
        "SELECT id, name, surname, email, created_at, is_admin
         FROM users ORDER BY id",
    )?;
    let rows = stmt.query_map([], row_to_user)?;
    rows.collect()
}

pub fn delete_user(db: &SharedDb, user_id: i64) -> Result<bool> {
    let conn = db.lock().unwrap();
    let rows_affected = conn.execute(
        "DELETE FROM users WHERE id = ?1",
        params![user_id],
    )?;
    Ok(rows_affected > 0)
}

pub fn set_user_admin(db: &SharedDb, user_id: i64, is_admin: bool) -> Result<bool> {
    let conn = db.lock().unwrap();
    let rows_affected = conn.execute(
        "UPDATE users SET is_admin = ?1 WHERE id = ?2",
        params![is_admin as i64, user_id],
    )?;
    Ok(rows_affected > 0)
}