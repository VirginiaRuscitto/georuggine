use rusqlite::Connection;
use std::error::Error;
use argon2::{ //TODO: rivedere come importare auth per l'hash
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

    let users = [
        ("Admin", "Admin", "admin@georuggine.it", 1),
        ("Mario", "Rossi", "mario.rossi@georuggine.it", 0),
        ("Luca", "Bianchi", "luca.bianchi@georuggine.it", 0),
    ];

    for (name, surname, email, is_admin) in users {
        let password_hash = hash_password("Password123!")
            .map_err(|e| std::io::Error::other(format!("Errore hashing password: {e}")))?;
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

    println!("Database popolato.");

    Ok(())
}