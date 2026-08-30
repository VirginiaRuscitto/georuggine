use rusqlite::Connection;
use std::error::Error;
use argon2::{
    password_hash::{
        rand_core::OsRng,
        PasswordHasher,
        SaltString,
    },
    Argon2,
};

fn hash_password(password: &str) -> Result<String, argon2::password_hash::Error> {
    let salt = SaltString::generate(&mut OsRng);
    let hash = Argon2::default().hash_password(password.as_bytes(), &salt)?;
    Ok(hash.to_string())
}

fn main() -> Result<(), Box<dyn Error>> {
    let conn = Connection::open("src/database/georuggine.db")?;

    //UTENTI
    let users = [
        ("Marco", "Rossi", "marco.rossi@example.com", 1),
        ("Giulia", "Bianchi", "giulia.bianchi@example.com", 1),
        ("Luca", "Ferrari", "luca.ferrari@example.com", 0),
        ("Chiara", "Romano", "chiara.romano@example.com", 0),
        ("Andrea", "Colombo", "andrea.colombo@example.com", 0),
        ("Francesca", "Ricci", "francesca.ricci@example.com", 0),
        ("Matteo", "Marino", "matteo.marino@example.com", 0),
        ("Sara", "Greco", "sara.greco@example.com", 0),
        ("Davide", "Bruno", "davide.bruno@example.com", 0),
        ("Elisa", "Gallo", "elisa.gallo@example.com", 0),
        ("Simone", "Conti", "simone.conti@example.com", 0),
        ("Valentina", "De Luca", "valentina.deluca@example.com", 0),
        ("Alessandro", "Costa", "alessandro.costa@example.com", 0),
        ("Martina", "Giordano", "martina.giordano@example.com", 0),
        ("Federico", "Mancini", "federico.mancini@example.com", 0),
        ("Ilaria", "Rizzo", "ilaria.rizzo@example.com", 0),
    ];

    for (name, surname, email, is_admin) in users {
        let password_hash = hash_password("Password123!")
            .map_err(|e| {
                std::io::Error::other(format!("Errore hashing password: {e}"))
            })?;

        conn.execute(
            r#"
            INSERT INTO users
                (name, surname, email, is_admin, password_hash)
            VALUES
                (?1, ?2, ?3, ?4, ?5)
            "#,
            (
                name,
                surname,
                email,
                is_admin,
                &password_hash,
            ),
        )?;
    }

    // MESSAGGI
    // ID utenti:
    // 1 = Marco Rossi (admin)
    // 2 = Giulia Bianchi (admin)
    // 3 = Luca Ferrari
    // 4 = Chiara Romano
    // 5 = Andrea Colombo
    // 6 = Francesca Ricci
    // 7 = Matteo Marino
    // 8 = Sara Greco
    // 9 = Davide Bruno

    let messages = [
        // BROADCAST
        (
            None,
            None,
            "Benvenuti su GeoRuggine! Questo è un messaggio in brodcast.",
        ),

        // LUCA (3)
        (
            Some(3),
            None,
            "Ho fatto un incidente.",
        ),

        // CHIARA (4)
        (
            Some(4),
            None,
            "Buongiorno, l'indirizzo indicato su un pacco non esiste.",
        ),
        (
            None,
            Some(4),
            "Ciao Chiara, sto verificando il problema. Puoi indicarmi l'indirizzo riportato sul pacco?",
        ),
        (
            Some(4),
            None,
            "Via Roma 125, ma sul navigatore la numerazione si ferma prima.",
        ),
        (
            None,
            Some(4),
            "Ok, verifico l'indirizzo e ti faccio sapere come procedere con la consegna.",
        ),


        // ANDREA (5)
        (      
            None,
            Some(5),
            "Sei in ritardo con le consegne?",
        ),

        // FRANCESCA (6)
        (
            Some(6),
            None,
            "Non riesco a visualizzare correttamente la mappa.",
        ),
        (
            None,
            Some(6),
            "Ciao Francesca, grazie della segnalazione. Puoi provare a ricaricare la pagina?",
        ),
        (
            Some(6),
            None,
            "Ho provato e adesso funziona. Grazie!",
        ),

        // MATTEO (7)
        (
            Some(7),
            None,
            "Ho completato il percorso previsto.",
        ),

        // SARA (8)
        (
            None,
            Some(8),
            "Ciao Sara, hai per caso dei problemi di connessione?",
        ),
        (
            None,
            Some(8),
            "Se leggi questo messaggio rispondi per favore.",
        ),

        // DAVIDE (9)
        (
            Some(9),
            None,
            "Buongiorno, si è bucata una gomma",
        ),
        (
            None,
            Some(9),
            "Mando subito un meccanico.",
        ),
    ];

    for (sender_id, recipient_id, content) in messages {
        conn.execute(
            r#"
            INSERT INTO messages
                (sender_id, recipient_id, content)
            VALUES
                (?1, ?2, ?3)
            "#,
            (sender_id, recipient_id, content),
        )?;
    }

    println!("Database popolato");

    Ok(())
}