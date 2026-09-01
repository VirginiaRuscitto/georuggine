//! Genera uno storico realistico delle posizioni degli utenti del seed.
//! Esempio:
//!
//! cargo run --bin bake_history 30
//!
//! Lo storico copre gli ultimi N giorni rispetto a Utc::now().
//! Gli utenti utilizzati sono quelli del seed con ID 3..9.

#[path = "../osrm.rs"]
mod osrm;

use chrono::{DateTime, Duration, NaiveDate, NaiveTime, Utc};
use csv::{Reader, WriterBuilder};
use rand::{rng, RngExt};
use serde::{Deserialize, Serialize};
use std::env;
use std::fs::{File, OpenOptions};

use osrm::VehicleSimulator;

const POSITIONS_CSV: &str = "history_positions.csv";

/// Una posizione ogni 30 secondi durante il movimento.
const TICK_SECONDS: u64 = 30;

//Utenti del seed da utilizzare nello storico.
const HISTORY_USERS: [HistoryUser; 7] = [
    HistoryUser {
        id: 3,
        email: "luca.ferrari@example.com",
    },
    HistoryUser {
        id: 4,
        email: "chiara.romano@example.com",
    },
    HistoryUser {
        id: 5,
        email: "andrea.colombo@example.com",
    },
    HistoryUser {
        id: 6,
        email: "francesca.ricci@example.com",
    },
    HistoryUser {
        id: 7,
        email: "matteo.marino@example.com",
    },
    HistoryUser {
        id: 8,
        email: "sara.greco@example.com",
    },
    HistoryUser {
        id: 9,
        email: "davide.bruno@example.com",
    },
];

/// Utente utilizzato per la generazione dello storico.
///
/// Gli ID e le email devono corrispondere a quelli presenti nel seed.
#[derive(Debug, Clone, Copy)]
struct HistoryUser {
    id: i64,
    email: &'static str,
}

/// Destinazione letta dal CSV dei locali.
#[derive(Debug, Deserialize, Clone)]
struct KebabShop {
    name: String,
    lat: f64,
    lon: f64,
}

/// Una riga del CSV finale.
#[derive(Debug, Serialize)]
struct PositionRecord {
    user_id: i64,
    email: String,
    lat: f64,
    lon: f64,
    elapsed_from_start_ms: u64,
    elapsed_from_last_ms: u64,
}

/// Legge le destinazioni dal CSV.
fn read_kebab_shops() -> Result<Vec<KebabShop>, Box<dyn std::error::Error + Send + Sync>> {
    let file = File::open("kebab_torino_google.csv")?;

    let mut reader = Reader::from_reader(file);

    let mut shops = Vec::new();

    for result in reader.deserialize() {
        let shop: KebabShop = result?;
        shops.push(shop);
    }

    if shops.len() < 2 {
        return Err(
            "kebab_torino_google.csv deve contenere almeno 2 destinazioni".into()
        );
    }

    Ok(shops)
}

/// Svuota il CSV precedente.
fn truncate_csv(path: &str) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    File::create(path)?;
    Ok(())
}

/// Scrive una posizione nel CSV.
///
/// `elapsed_ms` è il tempo trascorso dall'inizio dello storico.
///
/// `last_elapsed_ms` contiene l'ultimo timestamp scritto per quella sessione.
fn append_position(
    writer: &mut csv::Writer<std::fs::File>,
    user: &HistoryUser,
    lat: f64,
    lon: f64,
    elapsed_ms: u64,
    last_elapsed_ms: &mut u64,
) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    let interval = elapsed_ms.saturating_sub(*last_elapsed_ms);

    writer.serialize(PositionRecord {
        user_id: user.id,
        email: user.email.to_string(),
        lat,
        lon,
        elapsed_from_start_ms: elapsed_ms,
        elapsed_from_last_ms: interval,
    })?;

    *last_elapsed_ms = elapsed_ms;

    Ok(())
}

/// Genera una singola sessione di movimento.
///
/// La sessione:
/// - parte da una destinazione casuale;
/// - arriva verso una seconda destinazione casuale;
/// - produce una posizione ogni 30 secondi;
/// - dopo il movimento genera uno STOP con coordinate identiche.
async fn generate_session(
    writer: &mut csv::Writer<std::fs::File>,
    user: &HistoryUser,
    session_start: DateTime<Utc>,
    session_duration_minutes: u64,
    history_start: DateTime<Utc>,
    shops: &[KebabShop],
) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    let mut rng = rng();

    let max_duration_ms = session_duration_minutes * 60_000;
    let mut simulated_ms = 0u64;

    // Timestamp dell'ultima posizione scritta per questa sessione.
    let mut last_elapsed_ms = 0u64;

    // Ultima posizione raggiunta.
    let mut last_position: Option<(f64, f64)> = None;

    // ------------------------------------------------------------
    // SCEGLIAMO LA PRIMA ORIGINE
    // ------------------------------------------------------------

    let mut origin_index = rng.random_range(0..shops.len());

    println!(
        "  User {} | sessione {} min | {}",
        user.id,
        session_duration_minutes,
        session_start
    );

    // ------------------------------------------------------------
    // CICLO DEL CORRIERE
    // ------------------------------------------------------------
    //
    // Una sessione può contenere più consegne:
    //
    // partenza -> consegna -> pausa -> nuova destinazione
    //            -> consegna -> pausa -> nuova destinazione...
    //

    while simulated_ms < max_duration_ms {
        // Scegliamo una destinazione diversa dall'origine.
        let mut destination_index = rng.random_range(0..shops.len());

        while destination_index == origin_index {
            destination_index = rng.random_range(0..shops.len());
        }

        let origin = &shops[origin_index];
        let destination = &shops[destination_index];

        println!(
            "    User {} | {} -> {}",
            user.id,
            origin.name,
            destination.name
        );

        let origin_lon_lat = (origin.lon, origin.lat);
        let destination_lon_lat = (destination.lon, destination.lat);

        let vehicle_id = user.id.to_string();

        let mut simulator = VehicleSimulator::new(
            &vehicle_id,
            origin_lon_lat,
            destination_lon_lat,
        )
        .await?;

        // --------------------------------------------------------
        // MOVIMENTO VERSO LA CONSEGNA
        // --------------------------------------------------------

        let mut reached_destination = false;

        while simulated_ms < max_duration_ms {
            simulated_ms += TICK_SECONDS * 1_000;

            let Some(pos) = simulator.next_position(TICK_SECONDS as f64) else {
                break;
            };

            last_position = Some((pos.lat, pos.lon));

            let absolute_time =
                session_start + Duration::milliseconds(simulated_ms as i64);

            // Non generiamo mai dati futuri.
            if absolute_time > Utc::now() {
                break;
            }

            let elapsed_from_history_start = absolute_time
                .signed_duration_since(history_start)
                .num_milliseconds();

            if elapsed_from_history_start < 0 {
                continue;
            }

            append_position(
                writer,
                user,
                pos.lat,
                pos.lon,
                elapsed_from_history_start as u64,
                &mut last_elapsed_ms,
            )?;

            if simulator.is_finished() {
                reached_destination = true;
                break;
            }
        }

        // Se non siamo arrivati alla destinazione, la sessione è finita.
        if !reached_destination {
            break;
        }

        // --------------------------------------------------------
        // STOP / CONSEGNA
        // --------------------------------------------------------
        //
        // Il corriere è arrivato dal cliente.
        // Rimane fermo qualche minuto per effettuare la consegna.
        //

        let Some((lat, lon)) = last_position else {
            break;
        };

        let stop_minutes = rng.random_range(2..=8);

        for minute in 1..=stop_minutes {
            simulated_ms += 60_000;

            if simulated_ms > max_duration_ms {
                break;
            }

            let absolute_time =
                session_start + Duration::milliseconds(simulated_ms as i64);

            // Mai nel futuro.
            if absolute_time > Utc::now() {
                break;
            }

            let elapsed_from_history_start = absolute_time
                .signed_duration_since(history_start)
                .num_milliseconds();

            if elapsed_from_history_start < 0 {
                continue;
            }

            append_position(
                writer,
                user,
                lat,
                lon,
                elapsed_from_history_start as u64,
                &mut last_elapsed_ms,
            )?;
        }

        // --------------------------------------------------------
        // RIPARTENZA
        // --------------------------------------------------------
        //
        // La destinazione appena raggiunta diventa la nuova origine.
        //

        origin_index = destination_index;
    }

    Ok(())
}

/// Genera un orario casuale realistico per l'inizio di una sessione.
///
/// Fascia utilizzata: 07:00 - 18:59.
fn random_session_start(
    rng: &mut impl RngExt,
    day: NaiveDate,
) -> DateTime<Utc> {
    let hour = rng.random_range(7..=18);
    let minute = rng.random_range(0..60);

    let time = NaiveTime::from_hms_opt(hour, minute, 0)
        .expect("orario non valido");

    DateTime::<Utc>::from_naive_utc_and_offset(
        day.and_time(time),
        Utc,
    )
}

/// Decide se un utente lavora in un determinato giorno.
///
/// La probabilità varia in base al giorno e all'utente,
/// così gli utenti non seguono tutti lo stesso calendario.
///
/// In media ogni utente si muove alcuni giorni alla settimana,
/// ma non tutti i giorni.
fn user_active_that_day(
    rng: &mut impl RngExt,
    user_index: usize,
    day_index: i64,
) -> bool {
    // Pattern diverso per ogni utente.
    let pattern = (day_index + (user_index as i64 * 3)).rem_euclid(7);

    let chance = match pattern {
        0 | 1 => 25,
        2 | 3 | 4 => 55,
        _ => 35,
    };

    rng.random_range(0..100) < chance
}

/// Genera un numero realistico di sessioni per il giorno.
///
/// Normalmente 1 sessione.
/// In alcuni casi 2 sessioni.
fn session_count(rng: &mut impl RngExt) -> u32 {
    if rng.random_range(0..100) < 20 {
        2
    } else {
        1
    }
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    println!("=== GENERAZIONE STORICO FLEET ===");

    // ------------------------------------------------------------
    // ARGOMENTI
    // ------------------------------------------------------------

    let args: Vec<String> = env::args().collect();

    if args.len() != 2 {
        eprintln!("Uso:");
        eprintln!("  cargo run --bin bake_history <giorni_indietro>");
        eprintln!();
        eprintln!("Esempio:");
        eprintln!("  cargo run --bin bake_history 30");
        std::process::exit(1);
    }

    let history_days: i64 = args[1]
        .parse()
        .expect("Il numero di giorni deve essere un intero positivo");

    if history_days <= 0 {
        eprintln!("Errore: il numero di giorni deve essere maggiore di 0.");
        std::process::exit(1);
    }

    // ------------------------------------------------------------
    // INTERVALLO TEMPORALE
    // ------------------------------------------------------------

    let now = Utc::now();

    let history_start = now - Duration::days(history_days);

    println!("Ora corrente:   {}", now);
    println!("Storico da:     {}", history_start);
    println!("Storico a:      {}", now);

    // ------------------------------------------------------------
    // DESTINAZIONI
    // ------------------------------------------------------------

    let shops = read_kebab_shops()?;

    println!("Destinazioni disponibili: {}", shops.len());

    // ------------------------------------------------------------
    // UTENTI
    // ------------------------------------------------------------

    println!("\nUtenti storici:");

    for user in HISTORY_USERS.iter() {
        println!("  {} -> {}", user.id, user.email);
    }

    // ------------------------------------------------------------
    // CSV
    // ------------------------------------------------------------

    truncate_csv(POSITIONS_CSV)?;

    let file = OpenOptions::new()
        .create(true)
        .append(true)
        .open(POSITIONS_CSV)?;

    let mut writer = WriterBuilder::new()
        .has_headers(true)
        .from_writer(file);

    // ------------------------------------------------------------
    // GENERAZIONE
    // ------------------------------------------------------------

    let mut rng = rng();

    let mut generated_sessions = 0usize;

    for day_index in 0..history_days {
        let day = (history_start + Duration::days(day_index))
            .date_naive();

        for (user_index, user) in HISTORY_USERS.iter().enumerate() {
            // L'utente non lavora tutti i giorni.
            if !user_active_that_day(
                &mut rng,
                user_index,
                day_index,
            ) {
                continue;
            }

            let number_of_sessions = session_count(&mut rng);

            for session_index in 0..number_of_sessions {
                // Orario casuale.
                let mut session_start =
                    random_session_start(&mut rng, day);

                // Se è la seconda sessione, la spostiamo più avanti.
                if session_index == 1 {
                    session_start += Duration::hours(3);
                }

                // La sessione deve iniziare nel passato.
                if session_start >= now {
                    continue;
                }

                // Durata casuale della sessione.
                let requested_duration_minutes =
                    rng.random_range(35..=100);

                // Quanto tempo è effettivamente disponibile
                // prima di arrivare a "now".
                let remaining_minutes = now
                    .signed_duration_since(session_start)
                    .num_minutes();

                if remaining_minutes < 5 {
                    continue;
                }

                let actual_duration_minutes =
                    requested_duration_minutes
                        .min(remaining_minutes as u64);

                println!(
                    "\nGiorno {} - sessione {}",
                    day,
                    session_index + 1
                );

                generate_session(
                    &mut writer,
                    user,
                    session_start,
                    actual_duration_minutes,
                    history_start,
                    &shops,
                )
                .await?;

                generated_sessions += 1;
            }
        }
    }

    // ------------------------------------------------------------
    // FINE
    // ------------------------------------------------------------

    writer.flush()?;

    println!();
    println!("======================================");
    println!("Storico generato correttamente.");
    Ok(())
}