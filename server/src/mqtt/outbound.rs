use crate::errors::ErrorPayload;
use crate::models::UserState;
use chrono::{DateTime, Utc};
use rumqttc::{AsyncClient, QoS};
use serde::Serialize;

fn topic_direct(user_id: i64) -> String {
    format!("georuggine/server/{}/direct", user_id)
}

fn topic_broadcast() -> &'static str {
    "georuggine/server/broadcast"
}

fn topic_state(user_id: i64) -> String {
    format!("georuggine/server/{}/state", user_id)
}

fn topic_error(user_id: i64) -> String {
    format!("georuggine/server/{}/error", user_id)
}

async fn publish_json(mqtt_client: &AsyncClient, topic: impl Into<String>, payload: impl Serialize) -> bool {
    let topic = topic.into();
    let body = match serde_json::to_string(&payload) {
        Ok(b) => b,
        Err(e) => {
            tracing::error!("errore serializzazione payload MQTT per {topic}: {e}");
            return false;
        }
    };

    match mqtt_client.publish(topic.clone(), QoS::AtLeastOnce, false, body).await {
        Ok(_) => true,
        Err(e) => {
            tracing::error!("errore invio MQTT su {topic}: {e}");
            false
        }
    }
}

//Notifica un messaggio diretto admin -> singolo camionista.
pub async fn notify_direct_message(mqtt_client: &AsyncClient, user_id: i64, message_id: i64, content: &str) -> bool {
    let payload = serde_json::json!({
        "type": "direct",
        "id": message_id,
        "from": "server",
        "content": content,
        "timestamp": Utc::now().to_rfc3339(),
    });
    publish_json(mqtt_client, topic_direct(user_id), payload).await
}

//Notifica un messaggio broadcast admin -> tutti.
pub async fn notify_broadcast_message(mqtt_client: &AsyncClient, message_id: i64, content: &str) -> bool {
    let payload = serde_json::json!({
        "type": "broadcast",
        "id": message_id,
        "from": "server",
        "content": content,
        "timestamp": Utc::now().to_rfc3339(),
    });
    publish_json(mqtt_client, topic_broadcast(), payload).await
}

//Notifica un cambio di stato (fermo/in movimento/disconnesso).
pub async fn notify_state_change(mqtt_client: &AsyncClient, user_id: i64, state: UserState, at: DateTime<Utc>) {
    let payload = serde_json::json!({ "user_id": user_id, "state": state, "timestamp": at.to_rfc3339() });
    publish_json(mqtt_client, topic_state(user_id), payload).await;
}

//Notifica un errore al camionista, stesso payload {"error": "..."} usato dalle risposte HTTP.
pub async fn notify_error(mqtt_client: &AsyncClient, user_id: i64, msg: &str) {
    publish_json(mqtt_client, topic_error(user_id), ErrorPayload::new(msg)).await;
}