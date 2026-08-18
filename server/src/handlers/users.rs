use axum::{
    extract::{Path, Query, Extension, State},
    http::StatusCode,
    middleware,
    response::{IntoResponse, Response},
    routing::{get, delete, put},
    Json,
    Router,
};
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use crate::{
    dao::users_dao,
    dao::users_dao::{OrderByField, OrderDirection},
    errors::error_response,
    models::UserState,
    state::AppState,
    auth,
    auth::Claims,
};

#[derive(Serialize)]
pub struct UserStatus {
    pub id: i64,
    pub name: String,
    pub surname: String,
    pub email: String,
    pub created_at: DateTime<Utc>,
    pub state: UserState,
    pub is_admin: bool,
}

#[derive(Deserialize)]
pub struct UsersQuery {
    pub order_by_field: Option<OrderByField>,
    pub order_by_dir: Option<OrderDirection>,
    pub search: Option<String>,
    pub is_admin: Option<bool>,
    pub limit: Option<u32>,
    pub offset: Option<u32>
}

/// GET /api/users
/// Ritorna la lista completa degli utenti registrati.
pub async fn get_users_handler(State(state): State<AppState>, Query(params): Query<UsersQuery>) -> Response {
    let users = match users_dao::get_all_users(
        &state.db,
        params.search,
        params.order_by_field,
        params.order_by_dir,
        params.is_admin,
        params.limit,
        params.offset,
    ) {
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
            name: u.name,
            surname: u.surname,
            email: u.email,
            created_at: u.created_at,
            state: active.get(&u.id).map(|s| s.state).unwrap_or(UserState::Disconnected),
            is_admin: u.is_admin,
        })
        .collect();

    Json(result).into_response()
}

async fn me_handler(
    Extension(claims): Extension<Claims>,
    State(state): State<AppState>,
) -> Response {
    match users_dao::get_user_by_id(&state.db, claims.sub) {
        Ok(Some(user)) => (StatusCode::OK, Json(user)).into_response(),
        Ok(None) => error_response(StatusCode::NOT_FOUND, "Utente non trovato"),
        Err(e) => {
            tracing::error!("errore get_user_by_id: {e}");
            error_response(StatusCode::INTERNAL_SERVER_ERROR, "Errore del server")
        }
    }
}

async fn delete_user_handler(State(state): State<AppState>, Extension(claims): Extension<Claims>, Path(user_id): Path<i64>) -> Response {
    if claims.sub == user_id {
        return error_response(StatusCode::BAD_REQUEST, "Non puoi eliminare il tuo stesso account");
    }
    match users_dao::delete_user(&state.db, user_id) {
        Ok(true) => {
            state.active_users.write().unwrap().remove(&user_id);
            StatusCode::NO_CONTENT.into_response()
        }
        Ok(false) => error_response(StatusCode::NOT_FOUND, "Utente non trovato"),
        Err(e) => {
            tracing::error!("errore eliminazione utente: {e}");
            error_response(StatusCode::INTERNAL_SERVER_ERROR, "Impossibile eliminare l'utente. Riprova più tardi")
        }
    }
}

#[derive(Debug, Deserialize)]
struct UpdateAdminRequest {
    is_admin: bool,
}

async fn update_user_admin_handler(
    State(state): State<AppState>,
    Extension(claims): Extension<Claims>,
    Path(user_id): Path<i64>,
    Json(body): Json<UpdateAdminRequest>,
) -> Response {
    if claims.sub == user_id {
        return error_response(
            StatusCode::BAD_REQUEST,
            "Non puoi modificare il tuo stesso account",
        );
    }
    match users_dao::set_user_admin(&state.db, user_id, body.is_admin) {
        Ok(true) => StatusCode::NO_CONTENT.into_response(),
        Ok(false) => error_response(StatusCode::NOT_FOUND, "Utente non trovato"),
        Err(e) => {
            tracing::error!("errore update admin utente: {e}");
            error_response(
                StatusCode::INTERNAL_SERVER_ERROR,
                "Impossibile aggiornare l'utente. Riprova più tardi",
            )
        }
    }
}

pub fn router() -> Router<AppState> {
    Router::new()
        .merge(protected_router())
        .merge(admin_router())
}

fn protected_router() -> Router<AppState> {
    Router::new()
        .route("/api/me", get(me_handler))
        .layer(middleware::from_fn(auth::jwt_auth_middleware))
}

fn admin_router() -> Router<AppState> {
    Router::new()
        .route("/api/users", get(get_users_handler))
        .route("/api/admin/users/:user_id", delete(delete_user_handler))
        .route("/api/admin/users/:user_id/admin", put(update_user_admin_handler))
        .layer(middleware::from_fn(auth::jwt_admin_middleware))
}