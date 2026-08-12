use axum::{
    extract::{Extension, Query, State},
    http::StatusCode,
    middleware,
    response::IntoResponse,
    routing::{get, post},
    Json, Router,
};
use rumqttc::{AsyncClient, QoS};
use serde::{Deserialize, Serialize};
use crate::{
    auth::{self, Claims},
    dao::{messages_dao, users_dao},
    errors::error_response,
    state::AppState,
};

const DEFAULT_LIMIT: i64 = 50;
const MAX_LIMIT: i64 = 200;

#[derive(Deserialize)]
pub struct MessagesQuery {
    pub with: Option<i64>,
    pub limit: Option<i64>,
}

#[derive(Deserialize)]
pub struct DirectMessageRequest {
    pub recipient_id: i64,
    pub content: String,
}

#[derive(Deserialize)]
pub struct BroadcastRequest {
    pub content: String,
}

#[derive(Serialize)]
struct SendResult {
    id: i64,
    queued: bool,
}

async fn publish(mqtt_client: &AsyncClient, topic: impl Into<String>, payload: serde_json::Value) -> bool {
    match mqtt_client.publish(topic.into(), QoS::AtLeastOnce, false, payload.to_string()).await {
        Ok(_) => true,
        Err(e) => {
            tracing::error!("errore invio MQTT: {e}");
            false
        }
    }
}

pub async fn get_messages_handler(
    State(state): State<AppState>,
    Extension(claims): Extension<Claims>,
    Query(params): Query<MessagesQuery>,
) -> impl IntoResponse {
    let limit = params.limit.unwrap_or(DEFAULT_LIMIT).clamp(1, MAX_LIMIT);

    let result = if claims.is_admin {
        match params.with {
            Some(user_id) => messages_dao::get_direct_conversation(&state.db, user_id, limit),
            None => messages_dao::get_broadcast_messages(&state.db, limit),
        }
    } else {
        messages_dao::get_conversation_with_broadcasts(&state.db, claims.sub, limit)
    };

    match result {
        Ok(messages) => Json(messages).into_response(),
        Err(e) => {
            tracing::error!("errore recupero messaggi: {e}");
            error_response(StatusCode::INTERNAL_SERVER_ERROR, "Impossibile recuperare i messaggi")
        }
    }
}

pub async fn post_direct_message(
    State(state): State<AppState>,
    Json(body): Json<DirectMessageRequest>,
) -> impl IntoResponse {
    let content = match messages_dao::validate_content(&body.content) {
        Ok(c) => c,
        Err(msg) => return error_response(StatusCode::BAD_REQUEST, msg),
    };

    match users_dao::get_user_by_id(&state.db, body.recipient_id) {
        Ok(Some(_)) => {}
        Ok(None) => return error_response(StatusCode::NOT_FOUND, "Destinatario non trovato"),
        Err(e) => {
            tracing::error!("errore verifica destinatario: {e}");
            return error_response(StatusCode::INTERNAL_SERVER_ERROR, "Impossibile inviare il messaggio");
        }
    }

    let message_id = match messages_dao::insert_message(&state.db, None, Some(body.recipient_id), content) {
        Ok(id) => id,
        Err(e) => {
            tracing::error!("errore insert_message (direct): {e}");
            return error_response(StatusCode::INTERNAL_SERVER_ERROR, "Impossibile salvare il messaggio");
        }
    };

    let topic = format!("georuggine/server/{}/direct", body.recipient_id);
    let payload = serde_json::json!({ "type": "direct", "id": message_id, "from": "server", "content": content });
    let queued = publish(&state.mqtt_client, topic, payload).await;

    Json(SendResult { id: message_id, queued }).into_response()
}

pub async fn post_broadcast_handler(
    State(state): State<AppState>,
    Json(body): Json<BroadcastRequest>,
) -> impl IntoResponse {
    let content = match messages_dao::validate_content(&body.content) {
        Ok(c) => c,
        Err(msg) => return error_response(StatusCode::BAD_REQUEST, msg),
    };

    let message_id = match messages_dao::insert_message(&state.db, None, None, content) {
        Ok(id) => id,
        Err(e) => {
            tracing::error!("errore insert_message (broadcast): {e}");
            return error_response(StatusCode::INTERNAL_SERVER_ERROR, "Impossibile salvare il messaggio");
        }
    };

    let payload = serde_json::json!({ "type": "broadcast", "id": message_id, "content": content });
    let queued = publish(&state.mqtt_client, "georuggine/server/broadcast", payload).await;

    Json(SendResult { id: message_id, queued }).into_response()
}

pub fn router() -> Router<AppState> {
    Router::new()
        .merge(user_router())
        .merge(admin_router())
}

fn user_router() -> Router<AppState> {
    Router::new()
        .route("/api/messages", get(get_messages_handler))
        .layer(middleware::from_fn(auth::jwt_auth_middleware))
}

fn admin_router() -> Router<AppState> {
    Router::new()
        .route("/api/messages/direct", post(post_direct_message))
        .route("/api/broadcast", post(post_broadcast_handler))
        .layer(middleware::from_fn(auth::jwt_admin_middleware))
}