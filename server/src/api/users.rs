use crate::db::SharedDb;
use crate::models::UserState;
use crate::state::ActiveUsers;

/// GET /api/users
/// Ritorna la lista completa degli utenti registrati.
pub async fn get_users_handler(State(db): State<SharedDb>, State(active): State<ActiveUsers>) -> impl IntoResponse{

    let users = match crate::db::fetch_all_users(&db) {
        Ok(u) => u,
        Err(e) => {
            tracing::error!("errore fetch_all_users: {e}");
            return Json::<Vec<UserListItem>>(vec![]).into_response();
        }
    };

    let result: Vec<UserStatus> = users
        .into_iter()
        .map(|u| UserStatus {
            id: u.id,
            username: u.username,
            state: "Disconnected".to_string(),
        })
        .collect();

    Json(result).into_response()
}