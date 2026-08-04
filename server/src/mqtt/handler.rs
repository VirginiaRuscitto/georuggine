use crate::database::connection::SharedDb;
use crate::models::{UserState, Position, MovementState};
use crate::state::{ActiveUsers, UserSession};

use chrono::{DateTime, Utc};
use rumqttc::{AsyncClient, Event, EventLoop, Packet, QoS};

use serde::Deserialize;

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
    let new_pos = Position {
        lat,
        lon,
        recorded_at: now,
    };

    // 1. Salvataggio della posizione nel DB (position_log)
    {
        let conn = db.lock().unwrap();
        conn.execute(
            "INSERT INTO position_log (user_id, lat, lon, recorded_at) VALUES (?1, ?2, ?3, ?4)",
            rusqlite::params![user_id, lat, lon, now.to_rfc3339()],
        )?;
    }

    // 2. Controllo transizione di stato in memoria
    let mut transition_to: Option<UserState> = None;
    {
        let mut users = active.write().unwrap();
        let session = users.entry(user_id).or_insert_with(|| UserSession {
            last_position: None,
            last_change_at: now,
            state: UserState::Stopped,
        });

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

    // 3. Se lo stato è cambiato, aggiorna la sessione nel DB e notifica via MQTT
    if let Some(new_state) = transition_to {
        // Mappiamo UserState (usato in RAM) su MovementState (usato per il DB)
        let movement_state = match new_state {
            UserState::Disconnected | UserState::Stopped => MovementState::Stopped,
            UserState::Moving => MovementState::Moving,
        };

        // Aggiorna la tabella movement_sessions (sfruttando il tratto ToSql di MovementState)
        {
            let conn = db.lock().unwrap();
            conn.execute(
                "UPDATE movement_sessions SET ended_at = ?1 WHERE user_id = ?2 AND ended_at IS NULL",
                rusqlite::params![now.to_rfc3339(), user_id],
            )?;
            conn.execute(
                "INSERT INTO movement_sessions (user_id, state, started_at) VALUES (?1, ?2, ?3)",
                rusqlite::params![user_id, movement_state, now.to_rfc3339()],
            )?;
        }

        // Pubblica il cambio di stato sul topic dell'utente.
        // Grazie al macro #[serde(rename_all = "snake_case")] new_state diventerà "stopped" o "moving".
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

/// Salva nel DB un messaggio inviato da un utente al server
pub async fn handle_user_message(
    user_id: i64,
    content: String,
    db: &SharedDb,
) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    let now = Utc::now();
    let conn = db.lock().unwrap();
    // Inseriamo user_id come sender_id e NULL come recipient_id
    conn.execute(
        "INSERT INTO messages (sender_id, recipient_id, content, sent_at) VALUES (?1, NULL, ?2, ?3)",
        rusqlite::params![user_id, content, now.to_rfc3339()],
    )?;
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
pub async fn stale_state_watcher(
    active: ActiveUsers,
    db: SharedDb,
    mqtt_client: AsyncClient,
) {
    let mut interval = tokio::time::interval(std::time::Duration::from_secs(10));

    loop {
        interval.tick().await;
        let now = Utc::now();
        let mut stale_users = Vec::new();

        {
            let mut users = active.write().unwrap();
            for (&user_id, session) in users.iter_mut() {
                if session.state == UserState::Moving {
                    if now.signed_duration_since(session.last_change_at).num_seconds() >= 180 {
                        session.state = UserState::Stopped;
                        session.last_change_at = now;
                        stale_users.push(user_id);
                    }
                }
            }
        }

        for user_id in stale_users {
            // Chiude la sessione attiva e ne apre una nuova di tipo "stopped"
            {
                if let Ok(conn) = db.lock() {
                    let _ = conn.execute(
                        "UPDATE movement_sessions SET ended_at = ?1 WHERE user_id = ?2 AND ended_at IS NULL",
                        rusqlite::params![now.to_rfc3339(), user_id],
                    );
                    let _ = conn.execute(
                        "INSERT INTO movement_sessions (user_id, state, started_at) VALUES (?1, ?2, ?3)",
                        rusqlite::params![user_id, MovementState::Stopped, now.to_rfc3339()],
                    );
                }
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