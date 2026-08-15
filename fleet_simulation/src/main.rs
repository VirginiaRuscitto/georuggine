use chrono::{DateTime, Utc};
use csv::Reader;
use rand::seq::IndexedRandom;
use rand::rng;
use std::time::Duration;
use reqwest::{Client, Error};
use serde::{Deserialize, Serialize};
use crate::mqtt::{initialize_mqtt_client, send_message, send_position};
use crate::osrm::{Posizione, SimulatoreVeicolo};
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

async fn get_users(  ) -> Result< Vec<User>, Error > {
    let client = Client::new();

    let credentials = ("admin@example.com","Password123!");

    let token = client.post("http://127.0.0.1:3001/api/login").json(&credentials)
        .send().await?.json::<TokenResponse>().await?.token;

    let users_option = UsersOption{is_admin: false};

    let users = client.get("http://127.0.0.1:3001/api/users")
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
/// Termina da sola quando il veicolo raggiunge la destinazione.
async fn simula_movimento_utente(
    user: User,
    origine: (f64, f64),   // (lat, lon)
    destinazione: (f64, f64), // (lat, lon)
    token: String,
    kebab_shops: Vec<KebabShop>
) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    // ottieni_percorso/SimulatoreVeicolo vogliono (lon, lat), non (lat, lon):
    // qui invertiamo l'ordine per evitare il bug presente nella versione precedente.
    let origine_lon_lat = (origine.1, origine.0);
    let destinazione_lon_lat = (destinazione.1, destinazione.0);

    let mut simulatore = SimulatoreVeicolo::nuovo(
        &user.id.to_string(),
        origine_lon_lat,
        destinazione_lon_lat
    ).await?;

    let client_mqtt = initialize_mqtt_client(simulatore.id_veicolo(), "broker.emqx.io", 1883).await?;
    tokio::time::sleep(Duration::from_millis(500)).await;

    loop {
        tokio::time::sleep(Duration::from_secs(30)).await;

        match simulatore.prossima_posizione(30.0) {
            Some(pos) => {
                send_position(&client_mqtt, user.id, &token, pos.lat, pos.lon ).await?;
                println!("[{}] {:?}", simulatore.id_veicolo(), pos);
            }
            None => {
                send_message(&client_mqtt, user.id, &token, "destinazione raggiunta").await?;
                let mut rng = rng();
                let kebab = kebab_shops.choose(&mut rng).unwrap();
                simula_movimento_utente(user, destinazione_lon_lat, (kebab.lat, kebab.lon), token.clone(), kebab_shops.clone() );
                break;
            }
        }
    }

    Ok(())
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

    let users = get_users().await?;

    let mut handles = Vec::new();

    for user in users.clone().into_iter() {
        let mut rng = rng();
        let kebab = kebab_shops.choose(&mut rng).unwrap();

        let client = Client::new();

        let token = client
            .post("http://127.0.0.1:3001/api/login")
            .json(&(user.email.clone(), "Password123!"))
            .send()
            .await?
            .json::<TokenResponse>()
            .await?
            .token;

        let dest = kebab_shops.choose(&mut rng).unwrap();

        let handle = tokio::spawn(simula_movimento_utente(user, (kebab.lat, kebab.lon), (dest.lat, dest.lon), token.clone(), kebab_shops.clone() ) );
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
