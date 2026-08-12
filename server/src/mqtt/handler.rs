use crate::auth;
use crate::database::connection::SharedDb;
use crate::models::{UserState, Position, MovementState};
use crate::state::{ActiveUsers, UserSession};
use chrono::{DateTime, Utc};
use rumqttc::{AsyncClient, Event, EventLoop, Packet, QoS};
use serde::Deserialize;
use crate::dao::{messages_dao, movement_sessions_dao, position_log_dao};

pub const STALE_AFTER_SECS: i64 = 180;
// intervallo minimo tra due messaggi accettati dallo stesso utente via MQTT
const MIN_MESSAGE_INTERVAL_SECS: i64 = 1;

#[derive(Deserialize)]
pub struct PositionUpdatePayload {
    pub lat: f64,
    pub lon: f64,
}

#[derive(Deserialize)]
pub struct UserMessagePayload {
    // il client deve autenticarsi anche su MQTT: senza questo campo chiunque potrebbe
    // pubblicare sul topic di un altro utente e impersonarlo (vedi 1.1 del report)
    pub token: String,
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
            (prev.lat - new_pos.lat).abs() > 0.0001 || (prev.lon - new_pos.lon).abs() > 0.0001
        }
        None => false,
    };

    if coords_changed {
        Some(UserState::Moving)
    } else if now.signed_duration_since(last_change_at).num_seconds() >= 180 {
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
    {
        let mut users = active.write().unwrap();
        let session = users.entry(user_id).or_insert_with(|| UserSession {
            last_position: None,
            last_change_at: now,
            last_seen_at: now,
            last_message_at: None,
            state: UserState::Stopped,
        });

        // ogni posizione ricevuta conta come "visto ora", a prescindere dal cambio di stato
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
/// Il JWT nel payload deve appartenere allo stesso user_id del topic: se non corrisponde o non è
/// valido, il messaggio viene scartato con un warning invece di essere salvato a nome di qualcun altro.
/// Applica inoltre un rate limit (1 messaggio/secondo per utente) per evitare che un client in
/// loop o malevolo riempia la tabella messages senza controllo.
pub async fn handle_user_message(
    user_id: i64,
    payload: UserMessagePayload,
    db: &SharedDb,
    active: &ActiveUsers,
) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    let claims = match auth::verify_jwt(&payload.token) {
        Ok(c) => c,
        Err(_) => {
            tracing::warn!("MQTT message rifiutato: token non valido per user {user_id}");
            return Ok(());
        }
    };
    if claims.sub != user_id {
        tracing::warn!(
            "MQTT message rifiutato: token di user {} usato sul topic di user {}",
            claims.sub, user_id
        );
        return Ok(());
    }

    let content = match messages_dao::validate_content(&payload.content) {
        Ok(c) => c,
        Err(msg) => {
            tracing::warn!("messaggio MQTT scartato da user {user_id}: {msg}");
            return Ok(());
        }
    };

    let now = Utc::now();
    {
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
                tracing::warn!("messaggio MQTT scartato da user {user_id}: rate limit superato");
                return Ok(());
            }
        }
        session.last_message_at = Some(now);
    }

    if let Err(e) = messages_dao::insert_message(db, Some(user_id), None, content) {
        tracing::error!("errore salvataggio messaggio da user {user_id}: {e}");
    }

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
        let mut stale_users = Vec::new();

        {
            let mut users = active.write().unwrap();
            for (&user_id, session) in users.iter_mut() {
                if session.state == UserState::Moving
                    && now.signed_duration_since(session.last_change_at).num_seconds() >= STALE_AFTER_SECS
                {
                    session.state = UserState::Stopped;
                    session.last_change_at = now;
                    stale_users.push(user_id);
                }
            }
        }

        for user_id in stale_users {
            if let Err(e) = movement_sessions_dao::transition_session(&db, user_id, MovementState::Stopped, now) {
                tracing::error!("errore transizione stato per user {}: {:?}", user_id, e);
            }

            let topic = format!("georuggine/server/{}/state", user_id);
            if let Ok(payload) = serde_json::to_string(&serde_json::json!({
                "user_id": user_id,
                "state": UserState::Stopped,
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
                            let _ = handle_position_update(
                                user_id, payload.lat, payload.lon, &db, &active, &mqtt_client,
                            ).await;
                        }
                        Err(e) => tracing::warn!("payload posizione non valido da user {user_id}: {e}"),
                    },
                    "message" => match serde_json::from_slice::<UserMessagePayload>(&publish.payload) {
                        Ok(payload) => {
                            let _ = handle_user_message(user_id, payload, &db, &active).await;
                        }
                        Err(e) => tracing::warn!("payload messaggio non valido da user {user_id}: {e}"),
                    },
                    other => tracing::warn!("topic MQTT sconosciuto: {other} (user {user_id})"),
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