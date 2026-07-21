use serde::{Deserialize, Serialize};
use chrono::{DateTime, Utc};

//TODO: aggiustare i derive

#[derive(Serialize, Deserialize, Debug)]
pub enum UserState {
    Disconnected,
    Stopped,
    Moving,
}

#[derive(Serialize, Deserialize, Debug)]
pub struct Position {
    pub lat: f64,
    pub lon: f64,
    pub recorded_at: DateTime<Utc>,
}

#[derive(Serialize, Deserialize, Debug)]
pub struct User {
    pub id: i64,
    pub username: String,
    pub name: String,
    pub surname: String,
    pub email: String,
}

 #[derive(Serialize, Deserialize, Debug)]
 pub struct Message {
     pub id: i64,
     pub sender_id: Option<i64>,
     pub recipient_id: Option<i64>,
     pub content: String,
     pub sent_at: DateTime<Utc>,
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
// #[derive(Serialize, Deserialize, Debug)]
// pub enum MovementState {
//     Stopped,
//     Moving,
// }
// 
// #[derive(Serialize, Deserialize, Debug)]
// pub struct MovementSession {
//     pub id: i64,
//     pub user_id: i64,
//     pub state: MovementState,
//     pub started_at: DateTime<Utc>,
//     pub ended_at: Option<DateTime<Utc>>,
// }
