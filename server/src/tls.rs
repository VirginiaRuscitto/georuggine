//! Gestione del certificato TLS locale per servire il backend in HTTPS.
//!
//! Per il progetto usiamo un certificato self-signed: va benissimo per
//! dimostrare che il meccanismo funziona (dev/demo/localhost), ma NON è
//! adatto a un deploy pubblico reale. Per quello serve un certificato
//! valido (es. Let's Encrypt) emesso per un dominio pubblico, tipicamente
//! terminato da un reverse proxy (nginx/Caddy) davanti ad Axum — è una
//! scelta di infrastruttura, non qualcosa che si programma qui.
//!
//! Dipendenze aggiuntive (Cargo.toml):
//! axum-server = { version = "0.7", features = ["tls-rustls"] }

use axum_server::tls_rustls::RustlsConfig;
use std::path::Path;

const CERT_PATH: &str = "certs/dev-cert.pem";
const KEY_PATH: &str = "certs/dev-key.pem";

/// Carica il certificato/chiave self-signed da `certs/`. Se non esistono ancora,
/// spiega come generarli (con `mkcert`, consigliato, o con `openssl` come fallback)
/// invece di far fallire l'avvio con un errore criptico.
pub async fn load_or_explain(app_name: &str) -> anyhow::Result<RustlsConfig> {
    if !Path::new(CERT_PATH).exists() || !Path::new(KEY_PATH).exists() {
        anyhow::bail!(
            "\n\nCertificato TLS non trovato in '{CERT_PATH}' / '{KEY_PATH}'.\n\
             Per l'ambiente di sviluppo puoi generarne uno self-signed:\n\n\
             OPZIONE CONSIGLIATA (mkcert, nessun warning nel browser):\n\
             \x20 1. Installa mkcert (https://github.com/FiloSottile/mkcert)\n\
             \x20 2. mkcert -install\n\
             \x20 3. mkdir certs\n\
             \x20 4. mkcert -cert-file certs/dev-cert.pem -key-file certs/dev-key.pem localhost 127.0.0.1 ::1\n\n\
             OPZIONE ALTERNATIVA (openssl, il browser mostrerà un warning da accettare manualmente):\n\
             \x20 mkdir certs\n\
             \x20 openssl req -x509 -newkey rsa:2048 -nodes -days 365 \\\n\
             \x20   -keyout certs/dev-key.pem -out certs/dev-cert.pem \\\n\
             \x20   -subj \"/CN=localhost\"\n\n\
             ({app_name} si aspetta questi due file nella cartella 'certs/' allo stesso livello di Cargo.toml)"
        );
    }

    let config = RustlsConfig::from_pem_file(CERT_PATH, KEY_PATH).await?;
    Ok(config)
}