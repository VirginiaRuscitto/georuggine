use serde::Deserialize;
use std::fs::File;
use std::thread::sleep;
use std::time::Duration;

// Strutture per deserializzare la risposta JSON di Google Places API
#[derive(Debug, Deserialize)]
struct PlacesResponse {
    results: Vec<Place>,
    next_page_token: Option<String>,
    status: String,
}

#[derive(Debug, Deserialize)]
struct Place {
    name: String,
    geometry: Geometry,
}

#[derive(Debug, Deserialize)]
struct Geometry {
    location: Location,
}

#[derive(Debug, Deserialize)]
struct Location {
    lat: f64,
    lng: f64,
}

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let api_key = std::env::var("MAPS_API_KEY")
    .expect("MAPS_API_KEY environment variable not set");

    let client = reqwest::blocking::Client::new();
    let mut all_places: Vec<(String, f64, f64)> = Vec::new();
    let mut next_token: Option<String> = None;

    println!("Inizio ricerca kebabbari a Torino...");

    loop {
        // Costruzione dell'URL in base alla presenza o meno del paginazione token
        let url = match &next_token {
            Some(token) => format!(
                "https://maps.googleapis.com/maps/api/place/nearbysearch/json?pagetoken={}&key={}",
                token, api_key
            ),
            None => format!(
                "https://maps.googleapis.com/maps/api/place/nearbysearch/json?location=45.0703,7.6869&radius=10000&keyword=kebab&key={}",
                api_key
            ),
        };

        let res: PlacesResponse = client.get(&url).send()?.json()?;

        if res.status != "OK" && res.status != "ZERO_RESULTS" {
            eprintln!("Errore dalle API di Google: {}", res.status);
            break;
        }

        // Estrazione dei dati di ciascun locale
        for place in res.results {

            all_places.push((
                place.name,
                place.geometry.location.lat,
                place.geometry.location.lng,
            ));
        }

        // Verifica se esistono altre pagine di risultati (Google ne invia max 20 alla volta)
        next_token = res.next_page_token;
        if next_token.is_none() {
            break;
        }

        // Google richiede un breve ritardo prima di poter usare il next_page_token
        sleep(Duration::from_secs(2));
    }

    // Scrittura dei dati nel file CSV
    let file = File::create("kebab_torino_google.csv")?;
    let mut wtr = csv::Writer::from_writer(file);

    // Intestazione delle colonne
    wtr.write_record(&["name", "lat", "lon"])?;

    for place in &all_places {
        wtr.write_record(&[
            &place.0,
            &place.1.to_string(),
            &place.2.to_string(),
        ])?;
    }

    wtr.flush()?;
    println!(
        "Completato! Salvati {} locali nel file 'kebab_torino_google.csv'.",
        all_places.len()
    );

    Ok(())
}