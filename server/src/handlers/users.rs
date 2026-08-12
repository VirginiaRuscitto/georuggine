use axum::{
    extract::State,
    http::StatusCode,
    middleware,
    response::{IntoResponse, Response},
    routing::get,
    Json,
    Router,
};
use axum::extract::Query;
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use crate::{
    auth,
    dao::users_dao,
    dao::users_dao::OrderBy,
    errors::error_response,
    models::UserState,
    state::AppState,
    auth::jwt_auth_middleware,
    handlers::report::ReportQuery,
    models::ReportPeriod,
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
    pub order_by: Option<OrderBy>,
    pub search: Option<String>,
    pub is_admin: Option<bool>,
    pub state: Option<UserState>,
    pub limit: Option<u32>,
    pub offset: Option<u32>
}

/// GET /api/users
/// Ritorna la lista completa degli utenti registrati.
pub async fn get_users_handler(State(state): State<AppState>, Query(params): Query<UsersQuery>) -> Response {
    let users = match users_dao::get_all_users(
        &state.db,
        params.search,
        params.order_by,
        params.is_admin,
        params.limit,
        params.offset
    ) {
        Ok(u) => u,
        Err(e) => {
            tracing::error!("errore get_all_users: {e}");
            return error_response(StatusCode::INTERNAL_SERVER_ERROR, "Impossibile recuperare gli utenti");
        }
    };

    let active = state.active_users.read().unwrap();

    if params.state.is_some(){
        let result: Vec<UserStatus> = users
            .into_iter()
            .filter(|u| active.get(&u.id).unwrap().state == params.state.unwrap())
            .map(|u| UserStatus {
                id: u.id,
                name: u.name,
                surname: u.surname,
                email: u.email,
                created_at: u.created_at,
                state: active.get(&u.id).unwrap().state,
                is_admin: u.is_admin,
            })
            .collect();

        return Json(result).into_response()
    }

    //else
    let result: Vec<UserStatus> = users
        .into_iter()
        .map(|u| UserStatus {
            id: u.id,                        // <-- aggiunto
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
pub fn router() -> Router<AppState> {
    Router::new()
        .route("/api/users", get(get_users_handler))
        .layer(middleware::from_fn(jwt_auth_middleware))
}