use axum::{extract::State, response::IntoResponse, Json};
use serde::{Deserialize, Serialize};
use crate::{
    models::UserState,
    database::connection::SharedDb,
    dao::users_dao,
    state::ActiveUsers,
};

#[derive(Serialize, Deserialize)]
pub struct UserStatus {
    pub id: i64,
    pub username: String,
    pub state: UserState, // "Disconnected" | "Stopped" | "Moving"
}

/// GET /api/users
/// Ritorna la lista completa degli utenti registrati.
pub async fn get_users_handler(State(db): State<SharedDb>, active: ActiveUsers) -> impl IntoResponse{

    let users = match users_dao::get_all_users(&db) {
        Ok(u) => u,
        Err(e) => {
            eprintln!("errore get_all_users: {e}");
            return Json::<Vec<UserStatus>>(vec![]).into_response();
        }
    };

    let users_states = active.read().unwrap();

    let result: Vec<UserStatus> = users
        .into_iter()
        .map(|u| UserStatus {
            id: u.id,
            username: u.name,
            state: match users_states.get(&u.id){
                Some(T) => T.state,
                None => UserState::Disconnected
            }
        })
        .collect();

    Json(result).into_response()
}

#[cfg(test)]
mod tests {
    use axum::response::IntoResponse;
    use axum::extract::State;
    use crate::handlers::users::{get_users_handler, UserStatus};
    use crate::dao::users_dao;
    use crate::database::connection::SharedDb;
    use crate::models::UserState;
    use crate::state::ActiveUsers;
    use crate::state::UserSession;

    #[tokio::test]
    async fn users_test() {
        let connection: SharedDb = crate::database::connection::shared_connection().unwrap();

        let id1 = users_dao::insert_user(&connection, &crate::models::NewUser{
            name: "Giacomo".to_string(),
            surname: "".to_string(),
            email: "1".to_string(),
            password_hash: "xyz".to_string(),
            is_admin: false,
        }).unwrap();
        let id2 = users_dao::insert_user(&connection, &crate::models::NewUser{
            name: "Giovanni".to_string(),
            surname: "".to_string(),
            email: "2".to_string(),
            password_hash: "xyz".to_string(),
            is_admin: false,
        }).unwrap();
        users_dao::insert_user(&connection, &crate::models::NewUser{
            name: "Luigi".to_string(),
            surname: "".to_string(),
            email: "3".to_string(),
            password_hash: "xyz".to_string(),
            is_admin: false,
        }).unwrap();

        let active_users = ActiveUsers::default();
        {
            let mut active_guard = active_users.write().unwrap();

            // Assuming the map value has a `.state` field:
            active_guard.insert(id1, UserSession {
                last_position: None,
                last_change_at: Default::default(),
                state: UserState::Moving });
            active_guard.insert(id2, UserSession {
                last_position: None,
                last_change_at: Default::default(),
                state: UserState::Stopped
            });
        }

        let response = get_users_handler( State(connection), active_users).await.into_response();

        assert_eq!(response.status(), axum::http::StatusCode::OK);

        let body_bytes = axum::body::to_bytes(response.into_body(), usize::MAX)
            .await
            .expect("failed to read body");

        let users: Vec<UserStatus> = serde_json::from_slice(&body_bytes)
            .expect("failed to deserialize response body");

        assert_eq!(users.len(), 3);
        assert_eq!(users[0].username, "Giacomo");
    }
}