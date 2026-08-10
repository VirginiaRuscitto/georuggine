use chrono::{DateTime, Utc};
use rusqlite::types::{FromSql, FromSqlError, FromSqlResult, ToSql, ToSqlOutput, ValueRef};
use serde::{Deserialize, Serialize};
use std::str::FromStr;
//TODO: aggiustare i derive

//TODO: posso metterlo nel file state.rs solo per mauro o viene richiamato da qualche api?
#[derive(Serialize, Deserialize, Debug, Clone, Copy, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum UserState {
    Disconnected,
    Stopped,
    Moving,
}

#[derive(Serialize, Deserialize, Debug)]
pub struct User {
    pub id: i64,
    pub name: String,
    pub surname: String,
    pub email: String,
    pub created_at: DateTime<Utc>,
    pub is_admin: bool,
}

#[derive(Debug)] //non metto serialize e deserialize perchè non conto di farlo arrivare da json
pub struct NewUser {
    pub name: String,
    pub surname: String,
    pub email: String,
    pub password_hash: String,
    pub is_admin: bool,
}

#[derive(Serialize, Deserialize, Debug, Copy, Clone)]
pub struct Position {
    pub lat: f64,
    pub lon: f64,
    pub recorded_at: DateTime<Utc>,
}

#[derive(Serialize, Deserialize, Debug, Clone, Copy)]
#[serde(rename_all = "snake_case")]
pub enum ReportPeriod {
    Day,
    Week,
    Month,
}

#[derive(Serialize, Deserialize, Debug)]
pub struct RouteReport {
    pub user_id: i64,
    pub period: ReportPeriod,
    pub trajectory: Vec<Position>,
    pub avg_speed_kmh: f64,
    pub movement_duration_secs: i64,
    pub pause_duration_secs: i64,
}

#[derive(Debug, Serialize, Deserialize, Clone, Copy, PartialEq)]
#[serde(rename_all = "snake_case")]
pub enum MovementState {
    Stopped,
    Moving,
}

impl MovementState {
    pub fn db_value(&self) -> &'static str {
        match self {
            Self::Stopped => "stopped",
            Self::Moving => "moving",
        }
    }
}
impl FromStr for MovementState {
    type Err = ();
    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s {
            "stopped" => Ok(Self::Stopped),
            "moving" => Ok(Self::Moving),
            _ => Err(()),
        }
    }
}
impl ToSql for MovementState {
    fn to_sql(&self) -> rusqlite::Result<ToSqlOutput<'_>> {
        Ok(self.db_value().into())
    }
}
impl FromSql for MovementState {
    fn column_result(value: ValueRef<'_>) -> FromSqlResult<Self> {
        let s = value.as_str()?;

        s.parse()
            .map_err(|_| FromSqlError::InvalidType)
    }
}
impl From<UserState> for MovementState {
    fn from(s: UserState) -> Self {
        match s {
            UserState::Disconnected | UserState::Stopped => MovementState::Stopped,
            UserState::Moving => MovementState::Moving,
        }
    }
}

#[derive(Serialize, Deserialize, Debug)]
pub struct MovementSession {
    pub id: i64,
    pub user_id: i64,
    pub state: MovementState,
    pub started_at: DateTime<Utc>,
    pub ended_at: Option<DateTime<Utc>>,
}

#[derive(Serialize, Deserialize, Debug)]
pub struct Message {
    pub id: i64,
    pub sender_id: Option<i64>,
    pub recipient_id: Option<i64>,
    pub content: String,
    pub sent_at: DateTime<Utc>,
}
