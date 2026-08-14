use chrono::{DateTime, Utc};
use std::collections::HashMap;
use std::sync::{Arc, RwLock};
use rumqttc::AsyncClient;
use crate::database::connection::SharedDb;
use crate::models::{Position, UserState};

pub struct UserSession {
    pub last_position: Option<Position>,
    pub last_change_at: DateTime<Utc>, //si aggiorna quando cambia lo stato
    pub last_coord_change_at: DateTime<Utc>, //istante dell'ultimo cambio reale di coordinate
    pub last_seen_at: DateTime<Utc>, //aggiornato ad ogni posizione ricevuta
    pub last_message_at: Option<DateTime<Utc>>, //aggiornato ad ogni messaggio accettato, per il rate limiting
    pub state: UserState,
}

impl UserSession {
    pub fn new(now: DateTime<Utc>) -> Self {
        Self {
            last_position: None,
            last_change_at: now,
            last_coord_change_at: now,
            last_seen_at: now,
            last_message_at: None,
            state: UserState::Stopped,
        }
    }
}

pub type ActiveUsers = Arc<RwLock<HashMap<i64, UserSession>>>;

#[derive(Clone)]
pub struct AppState {
    pub db: SharedDb,
    pub active_users: ActiveUsers,
    pub mqtt_client: AsyncClient,
}

impl axum::extract::FromRef<AppState> for SharedDb {
    fn from_ref(state: &AppState) -> Self {
        state.db.clone()
    }
}

impl axum::extract::FromRef<AppState> for ActiveUsers {
    fn from_ref(state: &AppState) -> Self {
        state.active_users.clone()
    }
}