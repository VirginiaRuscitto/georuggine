use std::env;
use std::sync::OnceLock;
use argon2::password_hash::SaltString;
use rand::rngs::OsRng;
use argon2::{Argon2, PasswordHash, PasswordHasher, PasswordVerifier};
use jsonwebtoken::{decode, encode, DecodingKey, EncodingKey, Header, Validation};
use serde::{Deserialize, Serialize};
use chrono::{Duration, Utc};
use axum::{
    extract::{Extension, Request, State, Path},
    http::{HeaderMap, StatusCode},
    middleware::{self, Next},
    response::{IntoResponse, Response},
    routing::{post, delete, get, put},
    Json,
    Router,
};
use validator::Validate;
use crate::errors::error_response;
use crate::dao::users_dao;
use crate::database::connection::SharedDb;
use crate::models::{NewUser, User};
use crate::state::AppState;

fn jwt_secret() -> &'static [u8] {
    static SECRET: OnceLock<Vec<u8>> = OnceLock::new();

    SECRET
        .get_or_init(|| {
            let secret = env::var("JWT_SECRET")
                .expect("Errore: impostare la variabile d'ambiente JWT_SECRET prima di avviare il server");
            assert!(
                !secret.is_empty(),
                "JWT_SECRET non può essere vuota"
            );
            secret.into_bytes()
        })
        .as_slice()
}

fn hash_password(password: &str) -> Result<String, argon2::password_hash::Error> {
    let salt = SaltString::generate(&mut OsRng);
    let hash = Argon2::default().hash_password(password.as_bytes(), &salt)?;
    Ok(hash.to_string())
}

fn verify_password(password: &str, hash: &str) -> bool {
    match PasswordHash::new(hash) {
        Ok(parsed) => Argon2::default()
            .verify_password(password.as_bytes(), &parsed)
            .is_ok(),
        Err(_) => false,
    }
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct Claims {
    pub sub: i64,
    pub exp: usize,
    pub is_admin: bool, 
}

fn generate_jwt(user_id: i64, is_admin: bool) -> Result<String, jsonwebtoken::errors::Error> {
    let expiration = Utc::now() + Duration::hours(24);
    let claims = Claims {
        sub: user_id,
        is_admin,
        exp: expiration.timestamp() as usize,
    };
    encode(&Header::default(), &claims, &EncodingKey::from_secret(jwt_secret()))
}

fn verify_jwt(token: &str) -> Result<Claims, jsonwebtoken::errors::Error> {
    let data = decode::<Claims>(
        token,
        &DecodingKey::from_secret(jwt_secret()),
        &Validation::default(),
    )?;
    Ok(data.claims)
}

fn authenticate(headers: &HeaderMap) -> Result<Claims, StatusCode> {
    let token = headers
        .get("Authorization")
        .and_then(|v| v.to_str().ok())
        .and_then(|h| h.strip_prefix("Bearer "))
        .ok_or(StatusCode::UNAUTHORIZED)?;

    verify_jwt(token).map_err(|_| StatusCode::UNAUTHORIZED)
}

pub async fn jwt_auth_middleware(headers: HeaderMap, mut req: Request, next: Next) -> Result<Response, StatusCode> {
    let claims = authenticate(&headers)?;
    req.extensions_mut().insert(claims);
    Ok(next.run(req).await)
}

pub async fn jwt_admin_middleware(headers: HeaderMap, mut req: Request, next: Next) -> Result<Response, StatusCode> {
    let claims = authenticate(&headers)?;
    if !claims.is_admin {
        return Err(StatusCode::FORBIDDEN);
    }
    req.extensions_mut().insert(claims);
    Ok(next.run(req).await)
}

#[derive(Debug, Deserialize, Validate)]
struct RegisterRequest {
    name: String,
    surname: String,
    #[validate(email(message = "Email non valida"))]
    email: String,
    password: String,
    #[serde(default)]
    is_admin: bool,
}

#[derive(Debug, Deserialize)]
struct LoginRequest {
    email: String,
    password: String,
}

#[derive(Debug, Serialize)]
struct AuthResponse {
    token: String,
}

fn validate_password(password: &str) -> bool {
    password.len() >= 12
        && password.chars().any(|c| c.is_uppercase())
        && password.chars().any(|c| !c.is_alphanumeric())
}

async fn register_handler(State(db): State<SharedDb>, Json(body): Json<RegisterRequest>) -> Response {
    let user = match create_user(&db, body, false) {
        Ok(u) => u,
        Err(resp) => return resp,
    };

    match generate_jwt(user.id, user.is_admin) {
        Ok(token) => (StatusCode::CREATED, Json(AuthResponse { token })).into_response(),
        Err(e) => {
            tracing::error!("errore generazione jwt: {e}");
            error_response(StatusCode::INTERNAL_SERVER_ERROR, "L'account è stato creato, ma è impossibile completare l'accesso. Riprova più tardi")
        }
    }
}

async fn register_by_admin_handler(State(db): State<SharedDb>, Json(body): Json<RegisterRequest>) -> Response {
    match create_user(&db, body, true) {
        Ok(user) => (StatusCode::CREATED, Json(user)).into_response(),
        Err(resp) => resp,
    }
}

fn create_user(db: &SharedDb, body: RegisterRequest, allow_admin: bool) -> Result<User, Response> {
    if let Err(errors) = body.validate() {
        return Err(error_response(StatusCode::BAD_REQUEST, &format!("Dati non validi: {errors}")));
    }

    if body.name.trim().is_empty()
        || body.surname.trim().is_empty()
        || body.password.is_empty()
    {
        return Err(error_response(StatusCode::BAD_REQUEST, "Non sono stati inseriti tutti i dati richiesti"));
    }

    if !validate_password(&body.password) {
        return Err(error_response(
            StatusCode::BAD_REQUEST,
            "La password deve contenere almeno 12 caratteri, una lettera maiuscola e un simbolo",
        ));
    }

    let password_hash = match hash_password(&body.password) {
        Ok(h) => h,
        Err(e) => {
            tracing::error!("errore hashing password: {e}");
            return Err(error_response(StatusCode::INTERNAL_SERVER_ERROR, "Impossibile completare la registrazione. Riprova più tardi"));
        }
    };

    let is_admin = allow_admin && body.is_admin;
    let new_user = NewUser {
        name: body.name,
        surname: body.surname,
        email: body.email,
        password_hash,
        is_admin,
    };

    let user_id = match users_dao::insert_user(db, &new_user) {
        Ok(id) => id,

        Err(rusqlite::Error::SqliteFailure(e, _))
            if e.code == rusqlite::ErrorCode::ConstraintViolation =>
        {
            return Err(error_response(
                StatusCode::CONFLICT,
                "Questa email è già registrata",
            ));
        }

        Err(e) => {
            tracing::error!("errore inserimento utente: {e}");
            return Err(error_response(
                StatusCode::INTERNAL_SERVER_ERROR,
                "Impossibile completare la registrazione. Riprova più tardi",
            ));
        }
    };

    Ok(User {
        id: user_id,
        name: new_user.name,
        surname: new_user.surname,
        email: new_user.email,
        is_admin: new_user.is_admin,
        created_at: Utc::now(),
    })
}

async fn login_handler(State(db): State<SharedDb>, Json(body): Json<LoginRequest>) -> Response {
    let (user_id, password_hash, is_admin) = match users_dao::get_credentials_by_email(&db, &body.email) {
        Ok(Some(c)) => c,
        Ok(None) => return error_response(StatusCode::UNAUTHORIZED, "Credenziali non valide"),
        Err(e) => {
            tracing::error!("errore db in login: {e}");
            return error_response(StatusCode::INTERNAL_SERVER_ERROR, "Impossibile completare il login. Riprova più tardi");
        }
    };

    if !verify_password(&body.password, &password_hash) {
        return error_response(StatusCode::UNAUTHORIZED, "Credenziali non valide");
    }

    match generate_jwt(user_id, is_admin) {
        Ok(token) => (StatusCode::OK, Json(AuthResponse { token })).into_response(),
        Err(e) => {
            tracing::error!("errore generazione jwt: {e}");
            error_response(StatusCode::INTERNAL_SERVER_ERROR, "Impossibile completare il login. Riprova più tardi")
        }
    }
}

async fn me_handler(
    Extension(claims): Extension<Claims>,
    State(db): State<SharedDb>,
) -> Response {
    match users_dao::get_user_by_id(&db, claims.sub) {
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
    State(db): State<SharedDb>,
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
    match users_dao::set_user_admin(&db, user_id, body.is_admin) {
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
        .route("/api/register", post(register_handler))
        .route("/api/login", post(login_handler))
        .merge(protected_router())
        .merge(admin_router())
}

fn protected_router() -> Router<AppState> {
    Router::new()
        .route("/api/me", get(me_handler))
        .layer(middleware::from_fn(jwt_auth_middleware))
}

fn admin_router() -> Router<AppState> {
    Router::new()
        .route("/api/admin/register", post(register_by_admin_handler))
        .route("/api/admin/users/:user_id", delete(delete_user_handler))
        .route("/api/admin/users/:user_id/admin", put(update_user_admin_handler))
        .layer(middleware::from_fn(jwt_admin_middleware))
}