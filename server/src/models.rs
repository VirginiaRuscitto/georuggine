use chrono::{DateTime, Utc};
use rusqlite::types::{FromSql, FromSqlError, FromSqlResult, ToSql, ToSqlOutput, ValueRef};
use serde::{Deserialize, Serialize};
use std::str::FromStr;
//TODO: aggiustare i derive

#[derive(Serialize, Deserialize, Debug)]
pub enum UserState {
    Disconnected,
    Stopped,
    Moving,
}
impl UserState {
    pub fn db_value(self) -> &'static str {
        match self {
            Self::Disconnected => "disconnesso",
            Self::Stopped => "fermo",
            Self::Moving => "in_movimento",
        }
    }

    pub fn label(self) -> &'static str {
        match self {
            Self::Disconnected => "Disconnesso",
            Self::Stopped => "Fermo",
            Self::Moving => "In movimento",
        }
    }
}
impl FromStr for UserState {
    type Err = ();

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s {
            "disconnesso" => Ok(UserState::Disconnected),
            "fermo" => Ok(UserState::Stopped),
            "in_movimento" => Ok(UserState::Moving),
            _ => Err(()),
        }
    }
}
impl ToSql for UserState {
    fn to_sql(&self) -> rusqlite::Result<ToSqlOutput<'_>> {
        Ok(self.db_value().into())
    }
}
impl FromSql for UserState {
    fn column_result(value: ValueRef<'_>) -> FromSqlResult<Self> {
        let s = value.as_str()?;

        s.parse()
            .map_err(|_| FromSqlError::InvalidType)
    }
}

#[derive(Serialize, Deserialize, Debug)]
pub struct User {
    pub id: i64,
    pub name: String,
    pub surname: String,
    pub email: String,
    pub created_at: DateTime<Utc>,
}

#[derive(Debug)] //non metto serialize e deserialize perchè non conto di farlo arrivare da json
pub struct NewUser {
    pub name: String,
    pub surname: String,
    pub email: String,
    pub password_hash: String,
}

#[derive(Serialize, Deserialize, Debug)]
pub struct Position {
    pub lat: f64,
    pub lon: f64,
    pub recorded_at: DateTime<Utc>,
}

//TODO: valutare quale serve

// #[derive(Serialize, Deserialize, Debug)]
// pub enum ReportPeriod {
//     Day,
//     Week,
//     Month,
// }
//
// #[derive(Serialize, Deserialize, Debug)]
// pub struct RouteReport {
//     pub user_id: i64,
//     pub period: ReportPeriod,
//     pub trajectory: Vec<Position>,
//     pub avg_speed_kmh: f64,
//     pub movement_duration_secs: i64,
//     pub pause_duration_secs: i64,
// }
//
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum MovementState {
    Stopped,
    Moving,
}

impl MovementState {
    //database
    pub fn db_value(self) -> &'static str {
        match self {
            Self::Stopped => "fermo",
            Self::Moving => "in_movimento",
        }
    }
    //testo mostrato all'utente
    pub fn label(self) -> &'static str {
        match self {
            Self::Stopped => "Fermo",
            Self::Moving => "In movimento",
        }
    }
}
impl FromStr for MovementState {
    type Err = ();

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s {
            "fermo" => Ok(Self::Stopped),
            "in_movimento" => Ok(Self::Moving),
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
