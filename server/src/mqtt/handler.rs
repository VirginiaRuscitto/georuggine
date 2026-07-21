use crate::db::SharedDb;
use crate::models::UserState;
use crate::state::ActiveUsers;

pub async fn start_mqtt_listener(mut eventloop: rumqttc::EventLoop, db: SharedDb, active: ActiveUsers, mqtt_client: AsyncClient);
pub async fn handle_position_update(user_id: i64, lat: f64, lon: f64, db: &SharedDb, active: &ActiveUsers, mqtt_client: &AsyncClient);
pub async fn handle_user_message(user_id: i64, content: String, db: &SharedDb);
pub async fn broadcast_to_all(mqtt_client: &AsyncClient, content: &str);
pub async fn send_direct(mqtt_client: &AsyncClient, user_id: i64, content: &str);
pub async fn stale_state_watcher(active: ActiveUsers, db: SharedDb, mqtt_client: AsyncClient);

pub fn check_state_transition(old_pos: Option<&Position>, new_pos: &Position, last_change_at: DateTime<Utc>, now: DateTime<Utc>) -> Option<UserState>;