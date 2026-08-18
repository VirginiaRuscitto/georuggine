use chrono::{DateTime, Utc};
use csv::Reader;
use rand::seq::IndexedRandom;
use rand::rng;
use std::time::Duration;
use reqwest::{Client, Error};
use serde::{Deserialize, Serialize};
use crate::mqtt::{initialize_mqtt_client, send_message, send_position};
use crate::osrm::SimulatoreVeicolo;
use futures::future::join_all;


mod osrm;
mod mqtt;

#[derive(Deserialize, Debug)]
#[derive(Clone)]
pub struct User {
    pub id: i64,
    pub name: String,
    pub surname: String,
    pub email: String,
    pub created_at: DateTime<Utc>,
    pub is_admin: bool,
    pub state: UserState,
}

#[derive(Clone)]
pub struct UserInfo{
    pub lat: f64,
    pub lon: f64,
    pub lat_dest: f64,
    pub lon_dest: f64,
    pub token: String,
}

#[derive(Serialize, Deserialize, Debug, Clone, Copy, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum UserState {
    Disconnected,
    Stopped,
    Moving,
}

// 1. Define your target structure
#[derive(Deserialize, Debug)]
struct TokenResponse {
    token: String
}
#[derive(Serialize, Debug)]
struct UsersOption{
    is_admin: bool,
}

/// Client HTTP condiviso, configurato per accettare il certificato TLS
/// self-signed locale del backend (generato con mkcert per lo sviluppo).
///
/// NOTA per il report: `danger_accept_invalid_certs(true)` disabilita la
/// verifica del certificato del server. Va bene per sviluppo/demo su
/// localhost con un certificato self-signed che reqwest non riconosce
/// nativamente, ma NON andrebbe mai usato contro un server pubblico reale:
/// in quel caso il certificato deve essere valido (es. Let's Encrypt) e va
/// lasciata attiva la verifica di default.
fn build_http_client() -> Client {
    Client::builder()
        .danger_accept_invalid_certs(true)
        .build()
        .expect("impossibile costruire il client HTTP")
}

async fn get_users(  ) -> Result< Vec<User>, Error > {
    let client = build_http_client();

    let credentials = ("admin@example.com","Password123!");

    let token = client.post("https://127.0.0.1:3001/api/login").json(&credentials)
        .send().await?.json::<TokenResponse>().await?.token;

    let users_option = UsersOption{is_admin: false};

    let users = client.get("https://127.0.0.1:3001/api/users")
        .header("Authorization", format!("Bearer {}", token))
        .query(&users_option)
        .send()
        .await?
        .json::<Vec<User>>()
        .await?;

    Ok(users)
}

/// Simula il movimento di un utente verso un kebab a caso: ogni 30s calcola
/// la posizione successiva lungo il percorso reale e la pubblica via MQTT.
/// Quando il veicolo raggiunge la destinazione, ne sceglie una nuova e
/// ricomincia il ciclo. Termina solo se la lista kebab è vuota.
async fn simula_movimento_utente(
    user: User,
    mut origine: (f64, f64),   // (lat, lon)
    mut destinazione: (f64, f64), // (lat, lon)
    token: String,
    kebab_shops: Vec<KebabShop>
) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    loop {
        // ottieni_percorso/SimulatoreVeicolo vogliono (lon, lat), non (lat, lon):
        let origine_lon_lat = (origine.1, origine.0);
        let destinazione_lon_lat = (destinazione.1, destinazione.0);

        let mut simulatore = SimulatoreVeicolo::nuovo(
            &user.id.to_string(),
            origine_lon_lat,
            destinazione_lon_lat
        ).await?;

        let client_mqtt = initialize_mqtt_client(
            simulatore.id_veicolo(),
            "broker.emqx.io",
            crate::mqtt::MQTT_TLS_PORT
        ).await?;
        tokio::time::sleep(Duration::from_millis(500)).await;

        // Fase di movimento
        loop {
            tokio::time::sleep(Duration::from_secs(30)).await;

            match simulatore.prossima_posizione(30.0) {
                Some(pos) => {
                    send_position(&client_mqtt, user.id, &token, pos.lat, pos.lon).await?;
                    println!("[{}] {:?}", simulatore.id_veicolo(), pos);
                }
                None => {
                    send_message(&client_mqtt, user.id, &token, "destinazione raggiunta").await?;
                    break; // esci dal loop interno e passa a scegliere nuova destinazione
                }
            }
        }

        // Scegli la prossima destinazione
        if let Some(kebab) = scegli_kebab_casuale(&kebab_shops) {
            origine = destinazione;
            destinazione = (kebab.lat, kebab.lon);
            // il loop esterno ricomincia con il nuovo percorso
        } else {
            eprintln!(
                "nessun kebab disponibile in kebab_shops: impossibile scegliere la prossima destinazione per user {}",
                user.id
            );
            break;
        }
    }

    Ok(())
}

/// Sceglie un kebab a caso dalla lista, restituendo `None` invece di panicare
/// se la lista è vuota (es. CSV mancante o senza righe).
fn scegli_kebab_casuale(kebab_shops: &[KebabShop]) -> Option<&KebabShop> {
    let mut rng = rng();
    kebab_shops.choose(&mut rng)
}

#[derive(Debug, Deserialize, Clone)]
struct KebabShop{
    name: String,
    lat: f64,
    lon: f64,
}

fn read_csv() -> Result<Vec<KebabShop>,Box<dyn std::error::Error + Send+Sync>> {
    let mut rdr = Reader::from_path("kebab_torino_google.csv")?;

    let mut shops: Vec<KebabShop> = Vec::new();

    for result in rdr.deserialize() {
        let shop: KebabShop = result?;
        shops.push(shop);
    }

    Ok(shops)
}

#[tokio::main]
async fn main() -> Result<(),Box<dyn std::error::Error + Send + Sync>> {
    let kebab_shops = read_csv()?;

    if kebab_shops.is_empty() {
        return Err(format!(
            "kebab_torino_google.csv è stato letto correttamente ma non contiene righe di dati. \
             Verifica che il file esista nella cartella da cui lanci `cargo run` (stessa cartella di Cargo.toml) \
             e che contenga almeno una riga oltre all'intestazione."
        ).into());
    }

    let users = get_users().await?;

    if users.is_empty() {
        return Err("nessun utente restituito da GET /api/users: crea prima degli utenti (es. con create_users.rs) prima di avviare la simulazione".into());
    }

    let mut handles = Vec::new();

    for user in users.clone().into_iter() {
        let mut rng = rng();

        // .choose() qui è sicuro perché abbiamo già verificato che kebab_shops non sia vuoto
        let kebab = kebab_shops.choose(&mut rng)
            .expect("kebab_shops non dovrebbe essere vuoto a questo punto, controllato sopra");

        let client = build_http_client();

        let token = client
            .post("https://127.0.0.1:3001/api/login")
            .json(&(user.email.clone(), "Password123!"))
            .send()
            .await?
            .json::<TokenResponse>()
            .await?
            .token;

        let dest = kebab_shops.choose(&mut rng)
            .expect("kebab_shops non dovrebbe essere vuoto a questo punto, controllato sopra");

        let handle = tokio::spawn(simula_movimento_utente(
            user,
            (kebab.lat, kebab.lon),
            (dest.lat, dest.lon),
            token.clone(),
            kebab_shops.clone()
        ));
        handles.push(handle);
    }

    let results = join_all(handles).await;
    for r in results {
        if let Err(e) = r {
            eprintln!("task fallito: {:?}", e);
        }
    }

    Ok(())
}