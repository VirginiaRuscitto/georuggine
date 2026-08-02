mod api;
mod auth;
mod dao;
mod database;
mod models;
mod state;
mod errors;

use axum::http::{HeaderValue, Method};
use axum::Router;
use tower_http::cors::CorsLayer;

use database::connection::shared_connection;

#[tokio::main]
async fn main() {
    dotenvy::dotenv().ok();

    tracing_subscriber::fmt::init();

    let db = shared_connection().expect("impossibile connettersi al database");

    let cors = CorsLayer::new()
        .allow_origin("http://localhost:5173".parse::<HeaderValue>().unwrap()) // vite
        .allow_methods([Method::GET, Method::POST, Method::PUT, Method::DELETE])
        .allow_headers(tower_http::cors::Any);

    let app = Router::new()
        .merge(auth::router())
        .layer(cors)
        .with_state(db);

    let listener = tokio::net::TcpListener::bind("0.0.0.0:3001")
        .await
        .expect("impossibile aprire la porta 3001");

    tracing::info!("Server HTTP in ascolto su http://0.0.0.0:3001");
    axum::serve(listener, app).await.unwrap();
}