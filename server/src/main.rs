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
use axum::Router;
use rumqttc::{AsyncClient, MqttOptions};
use std::collections::HashMap;
use std::sync::{Arc, RwLock};
use futures::FutureExt;

use database::connection::SharedDb;
use state::{ActiveUsers, AppState};

fn spawn_supervised<F>(name: &'static str, fut: F)
where F: std::future::Future<Output = ()> + Send + 'static{
    tokio::spawn(async move {
        //AssertUnwindSafe perché i task non condividono stato mutabile instabile col resto
        let result = std::panic::AssertUnwindSafe(fut).catch_unwind().await;
        if let Err(e) = result {
            tracing::error!("task '{name}' terminato per panic: {e:?}");
        } else {
            tracing::warn!("task '{name}' terminato inaspettatamente (senza panic)");
        }
    });
}

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
    .allow_origin(Any) //TODO togliere any e mettere le robe giuste da accettare
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
        .merge(handlers::messages::router())
        .merge(handlers::users::router())
        .merge(handlers::report::router())
        .layer(cors)
        .with_state(app_state);

    let listener = tokio::net::TcpListener::bind("0.0.0.0:3001").await?;
    tracing::info!("Server HTTP in ascolto su 0.0.0.0:3001");

    // --- Task di background ---
    spawn_supervised("cpu_logging", logging::cpu_logging_task());
    spawn_supervised("mqtt_listener", mqtt::handler::start_mqtt_listener(
        eventloop, db.clone(), active_users.clone(), mqtt_client.clone(),
    ));
    spawn_supervised("stale_state_watcher", mqtt::handler::stale_state_watcher(
        active_users, db, mqtt_client,
    ));
    
    axum::serve(listener, app).await?;
    Ok(())
}