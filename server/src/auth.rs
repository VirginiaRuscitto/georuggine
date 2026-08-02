use std::env;
use std::sync::OnceLock;
use argon2::password_hash::SaltString;
use rand::rngs::OsRng;
use argon2::{Argon2, PasswordHash, PasswordHasher, PasswordVerifier};
use jsonwebtoken::{decode, encode, DecodingKey, EncodingKey, Header, Validation};
use serde::{Deserialize, Serialize};
use chrono::{Duration, Utc};
use axum::{
    extract::{Request, State},
    http::{HeaderMap, StatusCode},
    middleware::Next,
    response::{IntoResponse, Response},
    routing::post,
    Json,
    Router,
};
use crate::errors::error_response;
use crate::dao::users_dao;
use crate::database::connection::SharedDb;
use crate::models::NewUser;

fn jwt_secret() -> &'static [u8] {
    static SECRET: OnceLock<Vec<u8>> = OnceLock::new();

    SECRET
        .get_or_init(|| {
            let secret = env::var("JWT_SECRET")
                .expect("Errore: impostare la variabile d'ambiente JWT_SECRET prima di avviare il server");
            if secret.is_empty() {
                panic!("JWT_SECRET non può essere vuota.");
            }
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
}

fn generate_jwt(user_id: i64) -> Result<String, jsonwebtoken::errors::Error> {
    let expiration = Utc::now() + Duration::hours(24);
    let claims = Claims {
        sub: user_id,
        exp: expiration.timestamp() as usize,
    };
    encode(
        &Header::default(),
        &claims,
        &EncodingKey::from_secret(jwt_secret()),
    )
}

fn verify_jwt(token: &str) -> Result<Claims, jsonwebtoken::errors::Error> {
    let data = decode::<Claims>(
        token,
        &DecodingKey::from_secret(jwt_secret()),
        &Validation::default(),
    )?;
    Ok(data.claims)
}


pub async fn jwt_auth_middleware(headers: HeaderMap, mut req: Request, next: Next) -> Result<Response, StatusCode> {
    let token = headers
        .get("Authorization")
        .and_then(|v| v.to_str().ok())
        .and_then(|h| h.strip_prefix("Bearer "))
        .ok_or(StatusCode::UNAUTHORIZED)?;

    let claims = verify_jwt(token).map_err(|_| StatusCode::UNAUTHORIZED)?;
    req.extensions_mut().insert(claims);
    Ok(next.run(req).await)
}



#[derive(Debug, Deserialize)]
struct RegisterRequest {
    name: String,
    surname: String,
    email: String,
    password: String,
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
    if body.name.trim().is_empty()
        || body.surname.trim().is_empty()
        || body.email.trim().is_empty()
        || body.password.is_empty()
    {
        return error_response(StatusCode::BAD_REQUEST, "Non sono stati inseriti tutti i dati richiesti");
    }

    if !validate_password(&body.password) {
        return error_response(
            StatusCode::BAD_REQUEST,
            "La password deve contenere almeno 12 caratteri, una lettera maiuscola e un simbolo",
        );
    }

    match users_dao::email_exists(&db, &body.email) {
        Ok(true) => return error_response(StatusCode::CONFLICT, "Questa email è già registrata"),
        Ok(false) => {}
        Err(e) => {
            tracing::error!("errore db in register: {e}");
            return error_response(StatusCode::INTERNAL_SERVER_ERROR, "Impossibile completare la registrazione. Riprova più tardi");
        }
    }

    let password_hash = match hash_password(&body.password) {
        Ok(h) => h,
        Err(e) => {
            tracing::error!("errore hashing password: {e}");
            return error_response(StatusCode::INTERNAL_SERVER_ERROR, "Impossibile completare la registrazione. Riprova più tardi");
        }
    };

    let new_user = NewUser {
        name: body.name,
        surname: body.surname,
        email: body.email,
        password_hash,
    };

    let user_id = match users_dao::insert_user(&db, &new_user) {
        Ok(id) => id,
        Err(e) => {
            tracing::error!("errore inserimento utente: {e}");
            return error_response(StatusCode::INTERNAL_SERVER_ERROR, "Impossibile completare la registrazione. Riprova più tardi");
        }
    };

    match generate_jwt(user_id) {
        Ok(token) => (StatusCode::CREATED, Json(AuthResponse { token })).into_response(),
        Err(e) => {
            tracing::error!("errore generazione jwt: {e}");
            error_response(StatusCode::INTERNAL_SERVER_ERROR, "L'account è stato creato, ma è impossibile completare l'accesso. Riprova più tardi")
        }
    }
}

async fn login_handler(State(db): State<SharedDb>, Json(body): Json<LoginRequest>) -> Response {
    let (user_id, password_hash) = match users_dao::get_credentials_by_email(&db, &body.email) {
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

    match generate_jwt(user_id) {
        Ok(token) => (StatusCode::OK, Json(AuthResponse { token })).into_response(),
        Err(e) => {
            tracing::error!("errore generazione jwt: {e}");
            error_response(StatusCode::INTERNAL_SERVER_ERROR, "Impossibile completare il login. Riprova più tardi")
        }
    }
}


pub fn router() -> Router<SharedDb> {
    Router::new()
        .route("/api/register", post(register_handler))
        .route("/api/login", post(login_handler))
}