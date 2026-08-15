// Crea 20 utenti chiamando POST /api/register.
// Modifica BASE_URL sotto se il server gira su un indirizzo diverso.

use reqwest::Client;
use serde::Deserialize;
use serde_json::json;

#[derive(Deserialize, Debug)]
struct TokenResponse {
    token: String
}

const BASE_URL: &str = "http://127.0.0.1:3001";
const PASSWORD: &str = "Password123!";

const USERS: [(&str, &str); 20] = [
    ("Marco", "Rossi"),
    ("Giulia", "Bianchi"),
    ("Luca", "Ferrari"),
    ("Chiara", "Romano"),
    ("Andrea", "Colombo"),
    ("Francesca", "Ricci"),
    ("Matteo", "Marino"),
    ("Sara", "Greco"),
    ("Davide", "Bruno"),
    ("Elisa", "Gallo"),
    ("Simone", "Conti"),
    ("Valentina", "De Luca"),
    ("Alessandro", "Costa"),
    ("Martina", "Giordano"),
    ("Federico", "Mancini"),
    ("Ilaria", "Rizzo"),
    ("Riccardo", "Lombardi"),
    ("Sofia", "Moretti"),
    ("Gabriele", "Barbieri"),
    ("Alessia", "Fontana"),
];

#[tokio::main]
async fn main() -> Result<(),Box<dyn std::error::Error>>{
    let client = Client::new();

    let credentials = ("admin@example.com","Password123!");

    let token = client.post("http://127.0.0.1:3001/api/login").json(&credentials)
        .send().await?.json::<TokenResponse>().await?.token;

    println!("{token}");

    for (name, surname) in USERS {
        let email = format!("{}.{}@example.com", name.to_lowercase(), surname.to_lowercase());

        let res = client
            .post(format!("{BASE_URL}/api/register"))
            .header("Authorization", format!("Bearer {}", token))
            .json(&json!({
                "name": name,
                "surname": surname,
                "email": email,
                "password": PASSWORD,
                "is_admin": false
            }))
            .send()
            .await;

        match res {
            Ok(r) => println!("{} {} <{}> -> {}", name, surname, email, r.status()),
            Err(e) => println!("{} {} <{}> -> errore: {}", name, surname, email, e),
        }
    }

    Ok(())
}