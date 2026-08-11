use crate::database::connection::SharedDb;
use crate::models::{UserState, Position, MovementState};
use crate::state::{ActiveUsers, UserSession};
use chrono::{DateTime, Utc};
use rumqttc::{AsyncClient, Event, EventLoop, Packet, QoS};
use serde::Deserialize;
use crate::dao::{messages_dao, movement_sessions_dao, position_log_dao};

pub const STALE_AFTER_SECS: i64 = 180; //movimento -> fermo senza cambi coordinate
pub const COORD_EPSILON: f64 = 0.0001; //soglia per considerare due coordinate "diverse"
pub const DISCONNECT_AFTER_SECS: i64 = 300; //fermo/movimento -> disconnesso senza posizioni ricevute

#[derive(Deserialize)]
pub struct PositionUpdatePayload {
    pub lat: f64,
    pub lon: f64,
}

#[derive(Deserialize)]
pub struct UserMessagePayload {
    pub content: String,
}

/// Verifica la transizione di stato tra due coordinate o in base al tempo
pub fn check_state_transition(
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

/// Gestisce l'aggiornamento della posizione del veicolo (ricevuto ogni 30s)
pub async fn handle_position_update(
    user_id: i64,
    lat: f64,
    lon: f64,
    db: &SharedDb,
    active: &ActiveUsers,
    mqtt_client: &AsyncClient,
) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    let now = Utc::now();
    let new_pos = Position { lat, lon, recorded_at: now };

    position_log_dao::insert_position(db, user_id, lat, lon, now)?;

    let mut transition_to: Option<UserState> = None;
    let mut is_new_user = false;
    {
        let mut users = active.write().unwrap();
        is_new_user = !users.contains_key(&user_id);

        let session = users.entry(user_id).or_insert_with(|| UserSession {
            last_position: None,
            last_change_at: now,
            last_seen_at: now,
            state: UserState::Stopped,
        });

        session.last_seen_at = now;

        if let Some(new_state) = check_state_transition(
            session.last_position.as_ref(),
            &new_pos,
            session.last_change_at,
            now,
        ) {
            if session.state != new_state {
                session.state = new_state;
                session.last_change_at = now;
                transition_to = Some(new_state);
            }
        }
        session.last_position = Some(new_pos);
    }

    if is_new_user {
        movement_sessions_dao::insert_movement_session(db, user_id, MovementState::Stopped, now)?;
    }

    if let Some(new_state) = transition_to {
        let movement_state = MovementState::from(new_state);
        movement_sessions_dao::transition_session(db, user_id, movement_state, now)?;

        let topic = format!("georuggine/server/{}/state", user_id);
        let payload = serde_json::to_string(&serde_json::json!({
            "user_id": user_id,
            "state": new_state,
            "timestamp": now.to_rfc3339()
        }))?;
        mqtt_client.publish(topic, QoS::AtLeastOnce, false, payload).await?;
    }

    Ok(())
}


/// Salva nel DB un messaggio inviato da un utente al server (sender = utente, recipient = NULL).
pub async fn handle_user_message(
    user_id: i64,
    content: String,
    db: &SharedDb,
) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    messages_dao::insert_message(db, Some(user_id), None, &content)?;
    Ok(())
}

/// Invia un messaggio in broadcast a tutti i client in ascolto
pub async fn broadcast_to_all(
    mqtt_client: &AsyncClient,
    content: &str,
) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    let topic = "georuggine/server/broadcast";
    let payload = serde_json::to_string(&serde_json::json!({
        "from": "server",
        "content": content,
        "timestamp": Utc::now().to_rfc3339()
    }))?;

    mqtt_client.publish(topic, QoS::AtLeastOnce, false, payload).await?;
    Ok(())
}

/// Invia un messaggio diretto a un singolo veicolo
pub async fn send_direct(
    mqtt_client: &AsyncClient,
    user_id: i64,
    content: &str,
) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    let topic = format!("georuggine/server/{}/direct", user_id);
    let payload = serde_json::to_string(&serde_json::json!({
        "from": "server",
        "content": content,
        "timestamp": Utc::now().to_rfc3339()
    }))?;

    mqtt_client.publish(topic, QoS::AtLeastOnce, false, payload).await?;
    Ok(())
}

/// Task periodico che controlla l'inattività dei veicoli e forza lo stato "Stopped" dopo 3 minuti
pub async fn stale_state_watcher(active: ActiveUsers, db: SharedDb, mqtt_client: AsyncClient) {
    let mut interval = tokio::time::interval(std::time::Duration::from_secs(10));

    loop {
        interval.tick().await;
        let now = Utc::now();
        let mut transitions: Vec<(i64, UserState)> = Vec::new();

        {
            let mut users = active.write().unwrap();
            for (&user_id, session) in users.iter_mut() {
                if session.state != UserState::Disconnected
                    && now.signed_duration_since(session.last_seen_at).num_seconds() >= DISCONNECT_AFTER_SECS
                {
                    session.state = UserState::Disconnected;
                    session.last_change_at = now;
                    transitions.push((user_id, UserState::Disconnected));
                } else if session.state == UserState::Moving
                    && now.signed_duration_since(session.last_change_at).num_seconds() >= STALE_AFTER_SECS
                {
                    session.state = UserState::Stopped;
                    session.last_change_at = now;
                    transitions.push((user_id, UserState::Stopped));
                }
            }
        }

        for (user_id, new_state) in transitions {
            // Disconnected non è un valore ammesso in movement_sessions (solo stopped/moving):
            // in quel caso chiudiamo la sessione aperta senza aprirne una nuova.
            let result = if new_state == UserState::Disconnected {
                movement_sessions_dao::close_movement_session_for_user(&db, user_id, now)
            } else {
                movement_sessions_dao::transition_session(&db, user_id, MovementState::from(new_state), now).map(|_| ())
            };

            if let Err(e) = result {
                tracing::error!("errore transizione stato per user {}: {:?}", user_id, e);
            }

            let topic = format!("georuggine/server/{}/state", user_id);
            if let Ok(payload) = serde_json::to_string(&serde_json::json!({
                "user_id": user_id,
                "state": new_state,
                "timestamp": now.to_rfc3339()
            })) {
                let _ = mqtt_client.publish(topic, QoS::AtLeastOnce, false, payload).await;
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

                if parts.len() == 4 && parts[0] == "georuggine" && parts[1] == "client" {
                    if let Ok(user_id) = parts[2].parse::<i64>() {
                        match parts[3] {
                            "position" => {
                                if let Ok(payload) = serde_json::from_slice::<PositionUpdatePayload>(&publish.payload) {
                                    let _ = handle_position_update(
                                        user_id,
                                        payload.lat,
                                        payload.lon,
                                        &db,
                                        &active,
                                        &mqtt_client,
                                    ).await;
                                }
                            }
                            "message" => {
                                if let Ok(payload) = serde_json::from_slice::<UserMessagePayload>(&publish.payload) {
                                    let _ = handle_user_message(user_id, payload.content, &db).await;
                                }
                            }
                            _ => {}
                        }
                    }
                }
            }
            Ok(_) => {}
            Err(e) => {
                tracing::error!("Errore nell'EventLoop MQTT: {:?}", e);
                tokio::time::sleep(std::time::Duration::from_secs(1)).await;
            }
        }
    }
}