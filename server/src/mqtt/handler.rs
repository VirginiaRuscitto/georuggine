use crate::auth;
use crate::database::connection::SharedDb;
use crate::models::{UserState, Position, MovementState};
use crate::state::{ActiveUsers, UserSession};
use chrono::{DateTime, Utc};
use rumqttc::{AsyncClient, Event, EventLoop, Packet, QoS};
use serde::Deserialize;
use crate::dao::{messages_dao, movement_sessions_dao, position_log_dao};
use crate::handlers::messages;
use crate::mqtt::outbound;

pub const STALE_AFTER_SECS: i64 = 180; //movimento -> fermo senza cambi coordinate
pub const COORD_EPSILON: f64 = 0.0001; //soglia per considerare due coordinate "diverse"
pub const DISCONNECT_AFTER_SECS: i64 = 120; //fermo/movimento -> disconnesso senza posizioni ricevute - 4x l'intervallo di invio (30s) per tollerare qualche pacchetto perso
const MIN_MESSAGE_INTERVAL_SECS: i64 = 1; //intervallo minimo tra due messaggi accettati dallo stesso utente via MQTT

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

/// Verifica la transizione di stato tra due coordinate o in base al tempo
fn check_state_transition(
    old_pos: Option<&Position>,
    new_pos: &Position,
    last_change_at: DateTime<Utc>,
    now: DateTime<Utc>,
) -> Option<UserState> {
    let coords_changed = match old_pos {
        Some(prev) => {
            (prev.lat - new_pos.lat).abs() > COORD_EPSILON || (prev.lon - new_pos.lon).abs() > COORD_EPSILON
        }
        None => false,
    };

    if coords_changed {
        Some(UserState::Moving)
    } else if now.signed_duration_since(last_change_at).num_seconds() >= STALE_AFTER_SECS {
        Some(UserState::Stopped)
    } else {
        None
    }
}

/// Aggiorna (o crea) la sessione attiva dell'utente con la nuova posizione,
/// restituendo l'eventuale nuovo stato e se si tratta di una sessione nuova.
fn update_session_position(
    active: &ActiveUsers,
    user_id: i64,
    new_pos: Position,
    now: DateTime<Utc>,
) -> (Option<UserState>, bool) {
    let mut users = active.write().unwrap();
    let is_new_session = !users.contains_key(&user_id);
    let session = users.entry(user_id).or_insert_with(|| UserSession {
        last_position: None,
        last_change_at: now,
        last_seen_at: now,
        last_message_at: None,
        state: UserState::Stopped,
    });

    session.last_seen_at = now; //ogni posizione ricevuta conta come "visto ora"

    let mut transition_to = None;
    if let Some(new_state) = check_state_transition(session.last_position.as_ref(), &new_pos, session.last_change_at, now) {
        if session.state != new_state {
            session.state = new_state;
            session.last_change_at = now;
            transition_to = Some(new_state);
        }
    }
    session.last_position = Some(new_pos);

    (transition_to, is_new_session)
}

/// Gestisce l'aggiornamento della posizione del veicolo (ricevuto ogni 30s)
async fn handle_position_update(
    user_id: i64,
    payload: PositionUpdatePayload,
    db: &SharedDb,
    active: &ActiveUsers,
    mqtt_client: &AsyncClient,
) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    if !is_mqtt_token_valid(&payload.token, user_id) {
        return Ok(());
    }

    let now = Utc::now();
    let new_pos = Position { lat: payload.lat, lon: payload.lon, recorded_at: now };
    position_log_dao::insert_position(db, user_id, payload.lat, payload.lon, now)?;

    let (transition_to, is_new_session) = update_session_position(active, user_id, new_pos, now);

    if is_new_session {
        movement_sessions_dao::insert_movement_session(db, user_id, MovementState::Stopped, now)?;
    }

    if let Some(new_state) = transition_to {
        movement_sessions_dao::transition_session(db, user_id, MovementState::from(new_state), now)?;
        outbound::notify_state_change(mqtt_client, user_id, new_state, now).await;
    }

    Ok(())
}

fn check_and_update_rate_limit(active: &ActiveUsers, user_id: i64, now: DateTime<Utc>) -> bool {
    let mut users = active.write().unwrap();
    let session = users.entry(user_id).or_insert_with(|| UserSession {
        last_position: None,
        last_change_at: now,
        last_seen_at: now,
        last_message_at: None,
        state: UserState::Stopped,
    });

    if let Some(last) = session.last_message_at {
        if now.signed_duration_since(last).num_seconds() < MIN_MESSAGE_INTERVAL_SECS {
            return false;
        }
    }
    session.last_message_at = Some(now);
    true
}

//Salva nel DB un messaggio inviato da un utente al server (sender = utente, recipient = NULL).
//Applica un rate limit (1 messaggio/secondo per utente) per evitare che un client in loop riempia la tabella messages senza controllo.
async fn handle_user_message(
    user_id: i64,
    payload: UserMessagePayload,
    db: &SharedDb,
    active: &ActiveUsers,
    mqtt_client: &AsyncClient,
) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    if !is_mqtt_token_valid(&payload.token, user_id) {
        return Ok(());
    }

    let content = match messages::validate_content(&payload.content) {
        Ok(c) => c,
        Err(msg) => {
            tracing::warn!("messaggio MQTT scartato da user {user_id}: {msg}");
            outbound::notify_error(mqtt_client, user_id, msg).await;
            return Ok(());
        }
    };

    let now = Utc::now();
    if !check_and_update_rate_limit(active, user_id, now) {
        tracing::warn!("messaggio MQTT scartato da user {user_id}: rate limit superato");
        outbound::notify_error(mqtt_client, user_id, "Troppi messaggi in un breve intervallo, riprova tra poco").await;
        return Ok(());
    }

    if let Err(e) = messages_dao::insert_message(db, Some(user_id), None, content) {
        tracing::error!("errore salvataggio messaggio da user {user_id}: {e}");
        outbound::notify_error(mqtt_client, user_id, "Impossibile salvare il messaggio").await;
    }

    Ok(())
}

/// Task periodico che controlla l'inattività dei veicoli e forza lo stato "Stopped" dopo 3 minuti
/// dopo un periodo senza alcun segnale (posizione o messaggio) -> Disconnesso
/// (rimozione dalla mappa attiva e chiusura della sessione movement_sessions aperta).
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
                if now.signed_duration_since(session.last_seen_at).num_seconds() >= DISCONNECT_AFTER_SECS {
                    became_disconnected.push(user_id);
                    return false;
                }

                if session.state == UserState::Moving
                    && now.signed_duration_since(session.last_change_at).num_seconds() >= STALE_AFTER_SECS
                {
                    session.state = UserState::Stopped;
                    session.last_change_at = now;
                    became_stopped.push(user_id);
                }

                true
            });
        }

        for user_id in became_stopped {
            if let Err(e) = movement_sessions_dao::transition_session(&db, user_id, MovementState::Stopped, now) {
                tracing::error!("errore transizione stato per user {}: {:?}", user_id, e);
            }
            outbound::notify_state_change(&mqtt_client, user_id, UserState::Stopped, now).await;
        }

        for user_id in became_disconnected {
            if let Err(e) = movement_sessions_dao::close_movement_session_for_user(&db, user_id, now) {
                tracing::error!("errore chiusura sessione per user disconnesso {}: {:?}", user_id, e);
            }
        }
    }
}

/// Loop principale per la ricezione e il routing dei messaggi
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
                        Ok(payload) => {
                            if let Err(e) = handle_position_update(user_id, payload, &db, &active, &mqtt_client).await {
                                tracing::error!("errore gestione posizione per user {user_id}: {e}");
                            }
                        }
                        Err(e) => tracing::warn!("payload posizione non valido da user {user_id}: {e}"),
                    },
                    "message" => match serde_json::from_slice::<UserMessagePayload>(&publish.payload) {
                        Ok(payload) => {
                            if let Err(e) = handle_user_message(user_id, payload, &db, &active, &mqtt_client).await {
                                tracing::error!("errore gestione messaggio per user {user_id}: {e}");
                            }
                        }
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