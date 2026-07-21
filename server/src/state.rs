use chrono::{DateTime, Utc};
use std::collections::HashMap;
use std::sync::{Arc, RwLock};
use crate::models::{Position, UserState};
use rumqttc::AsyncClient; 

pub struct UserSession {
    pub last_position: Option<Position>,
    pub last_change_at: DateTime<Utc>,
    pub state: UserState,
}

pub type ActiveUsers = Arc<RwLock<HashMap<i64, UserSession>>>;