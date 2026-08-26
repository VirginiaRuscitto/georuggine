use crate::auth;
use crate::database::connection::SharedDb;
use crate::models::{UserState, Position, MovementState};
use crate::state::{ActiveUsers, UserSession};
use chrono::{DateTime, Utc};
use rumqttc::{AsyncClient, Event, EventLoop, Packet, QoS};
use serde::Deserialize;
use std::env;
use std::sync::OnceLock;
use crate::dao::{messages_dao, movement_sessions_dao, position_log_dao};
use crate::handlers::messages;
use crate::mqtt::outbound;

const DEFAULT_STALE_AFTER_SECS: i64 = 180;
const DEFAULT_DISCONNECT_AFTER_SECS: i64 = 120;
pub const COORD_EPSILON: f64 = 0.0001;
const MIN_MESSAGE_INTERVAL_SECS: i64 = 1;

/// Secondi senza cambio di coordinate prima di passare a `Stopped`.
///
/// Configurabile con la variabile d'ambiente `STALE_AFTER_SECS`: il default
/// di produzione (180s) assume GPS reali in tempo reale. Se stai facendo un
/// replay/simulazione accelerata (`speed_factor` > 1 in `replay.rs`), 180
/// secondi REALI possono non passare mai prima che la simulazione decida di
/// ripartire, e la sessione resta `moving` per sempre. In quel caso lancia
/// il server con una soglia più bassa, es. `STALE_AFTER_SECS=15`.
pub fn stale_after_secs() -> i64 {
    static VALUE: OnceLock<i64> = OnceLock::new();
    *VALUE.get_or_init(|| {
        env::var("STALE_AFTER_SECS")
            .ok()
            .and_then(|v| v.parse().ok())
            .unwrap_or(DEFAULT_STALE_AFTER_SECS)
    })
}

/// Secondi senza NESSUN aggiornamento (di qualsiasi tipo) prima di considerare
/// l'utente disconnesso e chiudere la sessione. Configurabile con
/// `DISCONNECT_AFTER_SECS`, stesso discorso di `stale_after_secs`.
pub fn disconnect_after_secs() -> i64 {
    static VALUE: OnceLock<i64> = OnceLock::new();
    *VALUE.get_or_init(|| {
        env::var("DISCONNECT_AFTER_SECS")
            .ok()
            .and_then(|v| v.parse().ok())
            .unwrap_or(DEFAULT_DISCONNECT_AFTER_SECS)
    })
}

#[derive(Deserialize)]
pub struct PositionUpdatePayload {
    pub token: String,
    pub lat: f64,
    pub lon: f64,
}

#[derive(Deserialize)]
pub struct UserMessagePayload {
    pub token: String,
    pub content: String,
}

fn is_mqtt_token_valid(token: &str, user_id: i64) -> bool {
    match auth::verify_jwt(token) {
        Ok(claims) if claims.sub == user_id => true,
        Ok(claims) => {
            tracing::warn!(
                "MQTT rifiutato: token di user {} usato sul topic di user {}",
                claims.sub, user_id
            );
            false
        }
        Err(_) => {
            tracing::warn!("MQTT rifiutato: token non valido per user {user_id}");
            false
        }
    }
}

fn coords_changed(old_pos: Option<&Position>, new_pos: &Position) -> bool {
    match old_pos {
        Some(prev) => {
            (prev.lat - new_pos.lat).abs() > COORD_EPSILON || (prev.lon - new_pos.lon).abs() > COORD_EPSILON
        }
        None => false,
    }
}

fn ensure_active_session(active: &ActiveUsers, db: &SharedDb, user_id: i64, now: DateTime<Utc>) {
    let is_new = {
        let mut users = active.write().unwrap();
        let is_new = !users.contains_key(&user_id);
        users.entry(user_id).or_insert_with(|| UserSession::new(now));
        is_new
    };

    if is_new {
        if let Err(e) = movement_sessions_dao::close_all_open_sessions_for_user(db, user_id, now) {
            tracing::error!("errore chiusura sessioni residue per user {user_id}: {e}");
        }
        if let Err(e) = movement_sessions_dao::insert_movement_session(
            db,
            user_id,
            MovementState::Stopped,
            now,
        ) {
            tracing::error!("errore creazione sessione per user {user_id}: {e}");
        }
    }
}

fn check_state_transition(
    moved: bool,
    last_coord_change_at: DateTime<Utc>,
    now: DateTime<Utc>,
) -> Option<(UserState, DateTime<Utc>)> {
    if moved {
        Some((UserState::Moving, now))
    } else if now.signed_duration_since(last_coord_change_at).num_seconds() >= stale_after_secs() {
        Some((UserState::Stopped, last_coord_change_at))
    } else {
        None
    }
}

fn update_session_position(
    active: &ActiveUsers,
    user_id: i64,
    new_pos: Position,
    now: DateTime<Utc>,
) -> Option<(UserState, DateTime<Utc>)> {
    let mut users = active.write().unwrap();
    let session = users
        .get_mut(&user_id)
        .expect("sessione creata da ensure_active_session");

    session.last_seen_at = now;

    let moved = coords_changed(session.last_position.as_ref(), &new_pos);
    if moved {
        session.last_coord_change_at = now;
    }

    let mut transition = None;
    if let Some((new_state, transition_at)) =
        check_state_transition(moved, session.last_coord_change_at, now)
    {
        if session.state != new_state {
            session.state = new_state;
            session.last_change_at = transition_at;
            transition = Some((new_state, transition_at));
        }
    }

    session.last_position = Some(new_pos);
    transition
}

fn validate_coordinates(lat: f64, lon: f64) -> Result<(), &'static str> {
    if !lat.is_finite() || !lon.is_finite() {
        return Err("Coordinate non valide");
    }
    if !(-90.0..=90.0).contains(&lat) || !(-180.0..=180.0).contains(&lon) {
        return Err("Coordinate fuori range");
    }
    Ok(())
}

async fn handle_position_update(
    user_id: i64,
    payload: PositionUpdatePayload,
    db: &SharedDb,
    active: &ActiveUsers,
    mqtt_client: &AsyncClient,
) {
    if !is_mqtt_token_valid(&payload.token, user_id) {
        return;
    }

    if let Err(msg) = validate_coordinates(payload.lat, payload.lon) {
        tracing::warn!("posizione MQTT scartata da user {user_id}: {msg}");
        outbound::notify_error(mqtt_client, user_id, msg).await;
        return;
    }

    let now = Utc::now();
    ensure_active_session(active, db, user_id, now);
    {
        let mut users = active.write().unwrap();
        if let Some(session) = users.get_mut(&user_id) {
            session.last_seen_at = now;
        }
    }
    let new_pos = Position {
        lat: payload.lat,
        lon: payload.lon,
        recorded_at: now,
    };

    if let Err(e) = position_log_dao::insert_position(db, user_id, payload.lat, payload.lon, now) {
        tracing::error!("errore salvataggio posizione per user {user_id}: {e}");
        outbound::notify_error(mqtt_client, user_id, "Impossibile salvare la posizione").await;
        return;
    }

    let transition = update_session_position(active, user_id, new_pos, now);

    if let Some((new_state, transition_at)) = transition {
        if let Err(e) = movement_sessions_dao::transition_session(
            db,
            user_id,
            MovementState::from(new_state),
            transition_at,
        ) {
            tracing::error!("errore transizione stato per user {user_id}: {e}");
        }
        outbound::notify_state_change(mqtt_client, user_id, new_state, transition_at)
        .await;
    }
}

fn check_and_update_rate_limit(active: &ActiveUsers, user_id: i64, now: DateTime<Utc>) -> bool {
    let mut users = active.write().unwrap();
    let session = users.entry(user_id).or_insert_with(|| UserSession::new(now));
    session.last_seen_at = now;

    if let Some(last) = session.last_message_at {
        if now.signed_duration_since(last).num_seconds() < MIN_MESSAGE_INTERVAL_SECS {
            return false;
        }
    }
    session.last_message_at = Some(now);
    true
}

async fn handle_user_message(
    user_id: i64,
    payload: UserMessagePayload,
    db: &SharedDb,
    active: &ActiveUsers,
    mqtt_client: &AsyncClient,
) {
    if !is_mqtt_token_valid(&payload.token, user_id) {
        return;
    }

    let content = match messages::validate_content(&payload.content) {
        Ok(c) => c,
        Err(msg) => {
            tracing::warn!("messaggio MQTT scartato da user {user_id}: {msg}");
            outbound::notify_error(mqtt_client, user_id, msg).await;
            return;
        }
    };

    let now = Utc::now();
    ensure_active_session(active, db, user_id, now);
    if !check_and_update_rate_limit(active, user_id, now) {
        tracing::warn!("messaggio MQTT scartato da user {user_id}: rate limit superato");
        outbound::notify_error(mqtt_client, user_id, "Troppi messaggi in un breve intervallo, riprova tra poco").await;
        return;
    }

    if let Err(e) = messages_dao::insert_message(db, Some(user_id), None, content) {
        tracing::error!("errore salvataggio messaggio da user {user_id}: {e}");
        outbound::notify_error(mqtt_client, user_id, "Impossibile salvare il messaggio").await;
    }
}

pub async fn stale_state_watcher(active: ActiveUsers, db: SharedDb, mqtt_client: AsyncClient) {
    let mut interval = tokio::time::interval(std::time::Duration::from_secs(10));

    loop {
        interval.tick().await;
        let now = Utc::now();
        let mut became_stopped = Vec::new();
        let mut became_disconnected = Vec::new();

        {
            let mut users = active.write().unwrap();
            users.retain(|&user_id, session| {
                if now.signed_duration_since(session.last_seen_at).num_seconds() >= disconnect_after_secs() {
                    became_disconnected.push((user_id, session.last_seen_at));
                    return false;
                }

                if session.state == UserState::Moving
                    && now.signed_duration_since(session.last_coord_change_at).num_seconds() >= stale_after_secs()
                {
                    session.state = UserState::Stopped;
                    session.last_change_at = session.last_coord_change_at;
                    became_stopped.push((user_id, session.last_coord_change_at));
                }

                true
            });
        }

        for (user_id, transition_at) in became_stopped {
            if let Err(e) = movement_sessions_dao::transition_session(&db, user_id, MovementState::Stopped, transition_at) {
                tracing::error!("errore transizione stato per user {}: {:?}", user_id, e);
            }
            outbound::notify_state_change(&mqtt_client, user_id, UserState::Stopped, transition_at).await;
        }

        for (user_id, disconnected_at) in became_disconnected {
            if let Err(e) = movement_sessions_dao::close_movement_session_for_user(&db, user_id, disconnected_at) {
                tracing::error!("errore chiusura sessione per user disconnesso {}: {:?}", user_id, e);
            }
            outbound::notify_state_change(&mqtt_client, user_id, UserState::Disconnected, disconnected_at).await;
        }
    }
}

pub async fn start_mqtt_listener(
    mut eventloop: EventLoop,
    db: SharedDb,
    active: ActiveUsers,
    mqtt_client: AsyncClient,
) {
    let _ = mqtt_client.subscribe("georuggine/client/+/position", QoS::AtMostOnce).await;
    let _ = mqtt_client.subscribe("georuggine/client/+/message", QoS::AtLeastOnce).await;

    loop {
        match eventloop.poll().await {
            Ok(Event::Incoming(Packet::Publish(publish))) => {
                let topic = publish.topic;
                let parts: Vec<&str> = topic.split('/').collect();

                if parts.len() != 4 || parts[0] != "georuggine" || parts[1] != "client" {
                    tracing::warn!("topic MQTT con formato inatteso: {topic}");
                    continue;
                }

                let Ok(user_id) = parts[2].parse::<i64>() else {
                    tracing::warn!("topic MQTT con user_id non numerico: {topic}");
                    continue;
                };

                match parts[3] {
                    "position" => match serde_json::from_slice::<PositionUpdatePayload>(&publish.payload) {
                        Ok(payload) => handle_position_update(user_id, payload, &db, &active, &mqtt_client).await,
                        Err(e) => tracing::warn!("payload posizione non valido da user {user_id}: {e}"),
                    },
                    "message" => match serde_json::from_slice::<UserMessagePayload>(&publish.payload) {
                        Ok(payload) => handle_user_message(user_id, payload, &db, &active, &mqtt_client).await,
                        Err(e) => tracing::warn!("payload messaggio non valido da user {user_id}: {e}"),
                    },
                    other => tracing::warn!("topic MQTT sconosciuto: {other} (user {user_id})"),
                }
            }
            Ok(_) => {}
            Err(e) => {
                tracing::error!("errore nell'EventLoop MQTT: {:?}", e);
                tokio::time::sleep(std::time::Duration::from_secs(1)).await;
            }
        }
    }
}