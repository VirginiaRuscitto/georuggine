use crate::database::connection::SharedDb;
use crate::models::{NewUser, User};
use rusqlite::{params, params_from_iter, OptionalExtension, Result, Row, ToSql};
use serde::Deserialize;

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

#[derive(Debug, Clone, Copy, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum OrderByField {
    Name,
    Surname,
    CreatedAt,
}

impl OrderByField {
    fn column(&self) -> &'static str {
        match self {
            OrderByField::Name => "name",
            OrderByField::Surname => "surname",
            OrderByField::CreatedAt => "created_at",
        }
    }
}

#[derive(Debug, Clone, Copy, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum OrderDirection {
    Asc,
    Desc,
}

impl OrderDirection {
    fn to_sql(&self) -> &'static str {
        match self {
            OrderDirection::Asc => "ASC",
            OrderDirection::Desc => "DESC",
        }
    }
}

pub fn get_all_users(
    db: &SharedDb,
    search: Option<String>,
    order_by_field: Option<OrderByField>,
    order_by_dir: Option<OrderDirection>,
    is_admin: Option<bool>,
    limit: Option<u32>,
    offset: Option<u32>,
) -> Result<Vec<User>> {
    let mut query = String::from("SELECT id, name, surname, email, created_at, is_admin FROM users");

    let mut conditions: Vec<&str> = Vec::new();
    let mut params: Vec<Box<dyn ToSql>> = Vec::new();

    // 1. Search Filter
    if let Some(s) = search {
        if !s.trim().is_empty() {
            conditions.push("(name LIKE ? OR surname LIKE ? OR email LIKE ?)");
            let pattern = format!("%{}%", s.trim());
            params.push(Box::new(pattern.clone()));
            params.push(Box::new(pattern.clone()));
            params.push(Box::new(pattern));
        }
    }

    // 2. Is Admin Filter
    if let Some(admin) = is_admin {
        conditions.push("is_admin = ?");
        params.push(Box::new(admin));
    }

    if !conditions.is_empty() {
        query.push_str(" WHERE ");
        query.push_str(&conditions.join(" AND "));
    }

    let field = order_by_field.unwrap_or(OrderByField::Name).column();
    let dir = order_by_dir.unwrap_or(OrderDirection::Asc).to_sql();
    query.push_str(&format!(" ORDER BY {field} {dir}"));

    let limit_val = limit.unwrap_or(10).min(100);
    let offset_val = offset.unwrap_or(0);
    query.push_str(" LIMIT ? OFFSET ?");
    params.push(Box::new(limit_val));
    params.push(Box::new(offset_val));

    let conn = db.lock().unwrap();
    let mut stmt = conn.prepare(&query)?;
    let rows = stmt.query_map(params_from_iter(params.iter()), row_to_user)?;
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