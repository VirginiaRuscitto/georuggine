//! Client MQTT standalone per pubblicare posizioni e messaggi verso il server georuggine.
//!
//! La connessione avviene ora in TLS (mqtts) sulla porta 8883 del broker
//! pubblico broker.emqx.io, usando il certificato CA del broker incluso
//! nel binario a compile-time (vedi `certs/broker.emqx.io-ca.crt`).
//!
//! Espone:
//! - `initialize_mqtt_client` per creare il client e avviare il polling dell'eventloop
//! - `send_position` per pubblicare un aggiornamento di posizione
//! - `send_message` per pubblicare un messaggio utente
//!
//! Dipendenze (Cargo.toml):
//! rumqttc = "0.24"   (il backend TLS "use-rustls" è quello di default)
//! serde_json = "1"
//! tokio = { version = "1", features = ["full"] }

use rumqttc::{AsyncClient, MqttOptions, QoS, TlsConfiguration, Transport};
use serde_json::json;
use std::time::Duration;

/// Errore generico per le operazioni di questo modulo.
pub type MqttResult<T> = Result<T, Box<dyn std::error::Error + Send + Sync>>;

/// Porta MQTT su TLS del broker pubblico EMQX (1883 è la porta in chiaro, non usarla più).
pub const MQTT_TLS_PORT: u16 = 8883;

/// Certificato CA del broker, scaricato da https://assets.emqx.com/data/broker.emqx.io-ca.crt
/// e salvato in certs/broker.emqx.io-ca.crt (stesso livello di Cargo.toml).
/// Se cambi broker in futuro, aggiorna questo file con la CA corretta.
const BROKER_CA_CERT: &[u8] = include_bytes!("../certs/broker.emqx.io-ca.crt");

fn tls_transport() -> Transport {
    Transport::Tls(TlsConfiguration::Simple {
        ca: BROKER_CA_CERT.to_vec(),
        alpn: None,
        client_auth: None,
    })
}

/// Inizializza il client MQTT in TLS e avvia il task che consuma l'eventloop in background.
///
/// `client_id` deve essere univoco sul broker (es. "georuggine-publisher-1").
/// `port` va impostata a `MQTT_TLS_PORT` (8883) per usare TLS.
/// Ritorna solo l'`AsyncClient`: l'eventloop viene pollato internamente in un task spawnato,
/// quindi va chiamata dentro un runtime tokio già avviato.
pub async fn initialize_mqtt_client(
    client_id: &str,
    host: &str,
    port: u16,
) -> MqttResult<AsyncClient> {
    let mut mqttoptions = MqttOptions::new(client_id, host, port);
    mqttoptions.set_keep_alive(Duration::from_secs(5));
    mqttoptions.set_transport(tls_transport());

    let (client, mut eventloop) = AsyncClient::new(mqttoptions, 10);

    tokio::spawn(async move {
        loop {
            match eventloop.poll().await {
                Ok(event) => println!("evento MQTT: {event:?}"),
                Err(e) => {
                    println!("errore nell'eventloop MQTT: {e:?}");
                    tokio::time::sleep(Duration::from_secs(1)).await;
                }
            }
        }
    });

    Ok(client)
}

/// Pubblica un aggiornamento di posizione per `user_id` sul topic
/// `georuggine/client/{user_id}/position`.
///
/// `token` deve essere un JWT valido il cui claim `sub` corrisponde a `user_id`,
/// altrimenti il server scarta silenziosamente il messaggio.
pub async fn send_position(
    client: &AsyncClient,
    user_id: i64,
    token: &str,
    lat: f64,
    lon: f64,
) -> MqttResult<()> {
    let topic = format!("georuggine/client/{user_id}/position");
    let payload = json!({
        "token": token,
        "lat": lat,
        "lon": lon,
    });

    client
        .publish(topic, QoS::AtMostOnce, false, payload.to_string())
        .await?;

    Ok(())
}

/// Pubblica un messaggio utente per `user_id` sul topic
/// `georuggine/client/{user_id}/message`.
///
/// Nota: il server applica un rate limit di 1 messaggio/secondo per utente;
/// messaggi inviati più ravvicinati vengono scartati.
pub async fn send_message(
    client: &AsyncClient,
    user_id: i64,
    token: &str,
    content: &str,
) -> MqttResult<()> {
    let topic = format!("georuggine/client/{user_id}/message");
    let payload = json!({
        "token": token,
        "content": content,
    });

    match client.publish(topic, QoS::AtLeastOnce, false, payload.to_string()).await {
        Ok(_) => println!("publish queued ok"),
        Err(e) => println!("publish failed: {e:?}"),
    }

    Ok(())
}

#[cfg(test)]
mod example_usage {
    use super::*;

    #[allow(dead_code)]
    async fn example() -> MqttResult<()> {
        let client = initialize_mqtt_client("georuggine-publisher-1", "broker.emqx.io", MQTT_TLS_PORT).await?;

        send_position(&client, 42, "eyJhbGciOi...", 41.2233, 17.7139).await?;
        send_message(&client, 42, "eyJhbGciOi...", "ciao dal publisher esterno").await?;

        Ok(())
    }
}