use axum::{
    extract::State,
    http::StatusCode,
    middleware,
    response::{IntoResponse, Response},
    routing::get,
    Json,
    Router,
};
use serde::Serialize;
use crate::{
    auth,
    dao::users_dao,
    errors::error_response,
    models::UserState,
    state::AppState,
};


#[derive(Serialize)]
pub struct UserStatus {
    pub id: i64,
    pub username: String,
    pub state: UserState, // "Disconnected" | "Stopped" | "Moving"
    pub is_admin: bool,
}

/// GET /api/users
/// Ritorna la lista completa degli utenti registrati.
pub async fn get_users_handler(State(state): State<AppState>) -> Response {
    let users = match users_dao::get_all_users(&state.db) {
        Ok(u) => u,
        Err(e) => {
            tracing::error!("errore get_all_users: {e}");
            return error_response(StatusCode::INTERNAL_SERVER_ERROR, "Impossibile recuperare gli utenti");
        }
    };

    let active = state.active_users.read().unwrap();

    let result: Vec<UserStatus> = users
        .into_iter()
        .map(|u| UserStatus {
            id: u.id,
            username: u.name,
            state: active.get(&u.id).map(|s| s.state).unwrap_or(UserState::Disconnected),
            is_admin: u.is_admin,
        })
        .collect();

    Json(result).into_response()
}
pub fn router() -> Router<AppState> {
    Router::new()
        .route("/api/users", get(get_users_handler))
        .layer(middleware::from_fn(auth::jwt_admin_middleware))
}