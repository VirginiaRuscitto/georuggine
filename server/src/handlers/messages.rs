use axum::{
    extract::{Query, State, Extension},
    http::StatusCode,
    response::IntoResponse,
    Json,
};
use rumqttc::QoS;
use serde::{Deserialize, Serialize};

use crate::{
    dao::messages_dao,
    state::AppState,
    auth::Claims,
};

#[derive(Deserialize)]
pub struct DirectMessageRequest {
    pub recipient_id: i64,
    pub content: String,
}

#[derive(Deserialize)]
pub struct MessagesQuery {
    pub with: Option<i64>,
    pub limit: Option<i64>,
}

#[derive(Deserialize)]
pub struct BroadcastRequest {
    pub content: String,
}

#[derive(Serialize)]
struct ApiError {
    error: String,
}

#[derive(Deserialize)]
pub struct ConversationQuery {
    pub with: i64,
    pub limit: Option<i64>,
}

/// GET /api/messages/conversation?with=1&limit=50
pub async fn get_conversation_handler(
    State(state): State<AppState>,
    Extension(claims): Extension<Claims>,
    Query(params): Query<ConversationQuery>,
) -> impl IntoResponse {
    let limit = params.limit.unwrap_or(50);
    
    match messages_dao::get_conversation(&state.db, claims.sub, params.with, limit) {
        Ok(messages) => Json(messages).into_response(),
        Err(e) => {
            eprintln!("errore get_conversation: {e}");
            (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(ApiError {
                    error: "impossibile recuperare i messaggi".into(),
                }),
            ).into_response()
        }
    }
}

pub async fn post_direct_message(
    State(state): State<AppState>,
    Extension(claims): Extension<Claims>,
    Json(body): Json<DirectMessageRequest>,
) -> impl IntoResponse {
    if body.content.trim().is_empty() {
        return (
            StatusCode::BAD_REQUEST,
            Json(ApiError {
                error: "il contenuto del messaggio non può essere vuoto".into(),
            }),
        ).into_response();
    }

    let message_id = match messages_dao::insert_message(
        &state.db,
        Some(claims.sub),
        Some(body.recipient_id),
        &body.content,
    ) {
        Ok(id) => id,
        Err(e) => {
            eprintln!("errore insert_message (direct): {e}");
            return (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(ApiError {
                    error: "impossibile salvare il messaggio".into(),
                }),
            ).into_response();
        }
    };

    // Notifica via MQTT al destinatario
    let topic = format!("georuggine/server/{}/direct", body.recipient_id);
    let payload = serde_json::json!({
        "type": "direct",
        "id": message_id,
        "from": claims.sub,
        "content": body.content,
    }).to_string();

    let delivered = match state
        .mqtt_client
        .publish(topic, QoS::AtLeastOnce, false, payload)
        .await
    {
        Ok(_) => true,
        Err(e) => {
            tracing::warn!("errore invio direct MQTT: {e}");
            false
        }
    };

    Json(serde_json::json!({
        "id": message_id,
        "delivered": delivered,
    })).into_response()
}

/// GET /api/messages?with=<user_id>&limit=<n>
pub async fn get_messages_handler(
    State(state): State<AppState>,
    Query(params): Query<MessagesQuery>,
) -> impl IntoResponse {
    let limit = params.limit.unwrap_or(50);

    match messages_dao::get_messages(&state.db, params.with, limit) {
        Ok(messages) => Json(messages).into_response(),
        Err(e) => {
            eprintln!("errore get_messages: {e}");
            (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(ApiError {
                    error: "impossibile recuperare i messaggi".into(),
                }),
            )
                .into_response()
        }
    }
}

/// POST /api/broadcast   body: { "content": "testo" }
/// Salva nel DB e pubblica su MQTT per tutti i veicoli.
pub async fn post_broadcast_handler(
    State(state): State<AppState>,
    Json(body): Json<BroadcastRequest>,
) -> impl IntoResponse {
    if body.content.trim().is_empty() {
        return (
            StatusCode::BAD_REQUEST,
            Json(ApiError {
                error: "il contenuto del messaggio non può essere vuoto".into(),
            }),
        )
            .into_response();
    }

    let message_id = match messages_dao::insert_message(&state.db, None, None, &body.content) {
        Ok(id) => id,
        Err(e) => {
            eprintln!("errore insert_message (broadcast): {e}");
            return (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(ApiError {
                    error: "impossibile salvare il messaggio".into(),
                }),
            )
                .into_response();
        }
    };

    let payload = serde_json::json!({
        "type": "broadcast",
        "id": message_id,
        "content": body.content,
    })
    .to_string();

    let delivered = match state
        .mqtt_client
        .publish("georuggine/server/broadcast", QoS::AtLeastOnce, false, payload)
        .await
    {
        Ok(_) => true,
        Err(e) => {
            tracing::warn!("errore invio broadcast MQTT: {e}");
            false
        }
    };

    Json(serde_json::json!({
        "id": message_id,
        "delivered": delivered,
    }))
    .into_response()
}