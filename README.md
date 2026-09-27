# GeoRuggine

GeoRuggine è un'applicazione client/server sviluppata in Rust per la geolocalizzazione in tempo reale di una flotta di veicoli. Ogni 
conducente può registrarsi e autenticarsi nell'applicazione e, durante il viaggio, invia periodicamente la propria 
posizione al server. Le informazioni ricevute vengono utilizzate per seguire i veicoli sulla mappa e per determinarne 
lo stato, distinguendo tra veicoli in movimento, fermi o disconnessi. Gli amministratori dispongono di una dashboard dalla 
quale possono monitorare l'intera flotta in tempo reale, visualizzare gli spostamenti dei singoli veicoli e consultare i dati 
raccolti attraverso report che mostrano il percorso effettuato, la velocità media, il tempo trascorso in movimento e le pause. 
Il sistema permette inoltre agli amministratori di comunicare con i conducenti, e viceversa, attraverso messaggi di testo, sia 
individualmente sia tramite comunicazioni broadcast rivolte all'intera flotta di veicoli.

## Funzionalità principali
- Registrazione e autenticazione degli utenti.
- Geolocalizzazione in tempo reale: i conducenti inviano periodicamente la propria posizione al server tramite MQTT.
- Rilevamento dello stato dei veicoli, distinguendo tra veicoli in movimento, fermi e disconnessi.
- Tracciamento degli spostamenti e gestione delle sessioni di movimento.
- Report dei tragitti, con informazioni sul percorso effettuato, sulla velocità media, sulla durata del movimento e sulle pause.
- Messaggistica bidirezionale tra amministratori e conducenti, con possibilità di comunicare individualmente oppure tramite messaggi broadcast rivolti all'intera flotta.
- Due interfacce distinte, con funzionalità e visualizzazioni diverse in base al ruolo
  - Conducente: visualizzazione della propria posizione, stato del veicolo, spostamenti e messaggistica.
  - Amministratore: visualizzazione dei veicoli in movimento sulla mappa, monitoraggio dello stato dei veicoli, consultazione dei report, messaggistica e gestione degli utenti dell'applicazione.
 
## Stack tecnico
| Livello | Tecnologie |
|---|---|
| Backend | `Rust`, `Axum`, `Tokio` |
| Comunicazione | `HTTPS/REST`, `MQTT`, `rumqttc` |
| Database | `SQLite`, `rusqlite` |
| Autenticazione | `JWT`, `Argon2` |
| Frontend | `React`, `TypeScript`, `Vite` |
| Interfaccia | `Tailwind CSS`, `Framer Motion` |
| Mappe | `Leaflet`, `React-Leaflet` |
| Logging | `tracing`, `tracing-subscriber` |

---

## Screenshot

### Dashboard amministratore

### Reportistica di un conducente

### Messaggistica lato amministratore

## Documentazione

- [Manuale utente](./documentazione/manuale_dello_sviluppatore.md)
- [Manuale dello sviluppatore](./documentazione/manuale_utente.md)

## Avvio in locale
Per avviare l'applicazione in ambiente di sviluppo è necessario avviare separatamente il backend e il frontend.
### Backend
```bash
cd server
cargo run --bin georuggine
```
### Frontend
```bash
cd client
npm install
npm run dev
```
### Demo
```bash
cd fleet_simulation
cargo run --bin replay
```

---

Per maggiori informazioni sulla configurazione, sull'utilizzo dell'applicazione e sull'esecuzione della demo, consultare la documentazione.
