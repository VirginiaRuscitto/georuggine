mod auth;
mod dao;
mod database;
mod errors;
mod handlers;
mod logging;
mod models;
mod mqtt;
mod state;

use tower_http::cors::{Any, CorsLayer};
use axum::http::{Method, header};
use axum::routing::{get, post};
use axum::Router;
use rumqttc::{AsyncClient, MqttOptions};
use std::collections::HashMap;
use std::sync::{Arc, RwLock};

use database::connection::SharedDb;
use state::{ActiveUsers, AppState};

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    dotenvy::dotenv().ok();
    logging::init_tracing();

    let db: SharedDb = database::connection::shared_connection()?;
    let active_users: ActiveUsers = Arc::new(RwLock::new(HashMap::new()));

    // --- Connessione al broker MQTT ---
    let mut mqttoptions = MqttOptions::new("georuggine_server", "broker.emqx.io", 1883);
    mqttoptions.set_keep_alive(std::time::Duration::from_secs(5));
    let (mqtt_client, eventloop) = AsyncClient::new(mqttoptions, 10);

    let app_state = AppState {
        db: db.clone(),
        active_users: active_users.clone(),
        mqtt_client: mqtt_client.clone(),
    };

    let cors = CorsLayer::new()
    .allow_origin(Any)
    .allow_methods([
        Method::GET,
        Method::POST,
        Method::PUT,
        Method::DELETE,
        Method::OPTIONS,
    ])
    .allow_headers([
        header::AUTHORIZATION,
        header::CONTENT_TYPE,
        header::ACCEPT,
    ]);

    let app = Router::new()
        .merge(auth::router())
        .merge(auth::admin_router())
        .merge(auth::protected_router())
        .route("/api/messages", get(handlers::messages::get_messages_handler))
        .route("/api/broadcast", post(handlers::messages::post_broadcast_handler))
        .layer(cors)
        .with_state(app_state);

    let listener = tokio::net::TcpListener::bind("0.0.0.0:3001").await?;
    tracing::info!("Server HTTP in ascolto su 0.0.0.0:3001");

    // --- Task di background ---
    tokio::spawn(logging::cpu_logging_task());
    tokio::spawn(mqtt::handler::start_mqtt_listener(eventloop, db, active_users, mqtt_client));

    axum::serve(listener, app).await?;
    Ok(())
}