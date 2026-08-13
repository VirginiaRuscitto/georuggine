# Manuale dello sviluppatore

## 1.Introduzione

### 1.1 Panoramica del progetto

Il progetto è organizzato in due parti: client e server. Il client rappresenta l'applicazione utilizzata dagli utenti e dagli amministratori, mentre il server si occupa di coordinare la comunicazione tra i client, gestire gli utenti, le posizioni, gli stati, i messaggi e la memorizzazione dei dati. Gli amministratori, pur utilizzando client distinti, operano tutti in veste di server e rappresentano quindi un'unica entità nei confronti degli utenti.

La comunicazione tra client e server utilizza protocolli diversi in base al tipo di operazione e di client. Le operazioni come l'autenticazione, la registrazione e la consultazione dello storico dei messaggi utilizzano HTTP REST per entrambi i client. L'amministratore utilizza HTTP per attività come la gestione degli utenti, la generazione dei report e l'invio di messaggi diretti o broadcast. Trattandosi di un client utilizzato da una postazione stabile, il modello richiesta-risposta di HTTP si adatta bene alle interazioni con il server. Il client degli utenti utilizza invece MQTT, dovendo inviare periodicamente al server la propria posizione.La scelta è legata alla natura IoT del client, che può trovarsi in presenza di una connessione meno stabile. MQTT permette di gestire questo tipo di comunicazione senza dover effettuare una nuova richiesta HTTP per ogni posizione, offrendo inoltre meccanismi di gestione e ritrasmissione dei messaggi. Siccome MQTT viene già utilizzato per la posizione ed è adatto a questo tipo di client, è stato utilizzato anche per per l'invio e la ricezione dei messaggi e per le notifiche relative ai cambiamenti di stato e agli errori.  TODO cambiare forma a quest'ultima frase che non mi paice

La comunicazione HTTP del client web è inoltre gestita tramite CORS, che permette al client di effettuare richieste al server.

### 1.2 Stack tecnico

| Tecnologia |Utilizzo nel progetto |
|---|---|
| Rust | È il linguaggio richiesto dalle specifiche ed è stato utilizzato per sviluppare l'intero backend. |
| Tokio | Runtime asincrono utilizzato per eseguire il server HTTP e coordinare le attività in background, tra cui il listener MQTT, il controllo dello stato degli utenti e il logging periodico della CPU, permettendo di gestire queste operazioni in concorrenza senza bloccare il server. |
| rusqlite | Libreria utilizzata per l'accesso al database SQLite. Sono state inoltre aggiunte le feature bundled e chrono. |
| Axum | Framework utilizzato per sviluppare il server HTTP e gestire le API REST del backend. |
| tower-http | Fornisce il middleware CORS, utilizzato per gestire le richieste provenienti dal frontend. |
| jsonwebtoken | Generazione e verifica dei token JWT per autenticare gli utenti e proteggere le route che richiedono l'accesso autenticato o i privilegi di amministratore. |
| serde/serde_json | Utilizzati per convertire le strutture dati Rust in JSON e viceversa, sia per i dati delle API REST sia per i messaggi MQTT. |
| rumqttc | Gestisce la comunicazione MQTT con il broker, occupandosi della pubblicazione e della sottoscrizione ai topic utilizzati per la posizione, lo stato e la messaggistica in tempo reale. |
| chrono | Gestione dei timestamp in formato UTC. |
| Argon2 | Protegge le password tramite hashing durante la registrazione e ne verifica la corrispondenza durante il login. |
| rand | Utilizzato per fornire il generatore casuale OsRng, impiegato nella generazione del salt per Argon2. |
| validator | Validazione dei dati ricevuti dalle API, ad esempio del formato dell'email durante la registrazione. |
| tracing/tracing-subscriber | Gestiscono il sistema di logging del server: tracing genera i messaggi di log, mentre tracing-subscriber ne gestisce la raccolta e la configurazione. È stata utilizzata la feature env-filter che permette di configurare il livello dei log tramite la variabile d'ambiente `RUST_LOG`, utilizzando info come valore predefinito. |
| anyhow | Semplifica la gestione degli errori nelle funzioni che possono restituire errori diversi, permettendone la propagazione tramite Result e l'operatore ?. |
| dotenvy | Carica dal file `.env` la variabile di configurazione del server JWT_SECRET. |
| sysinfo | Permette di raccogliere informazioni sull'utilizzo della CPU da parte del processo. |
| futures | Fornisce `catch_unwind`, utilizzato per intercettare eventuali panic nei task eseguiti in background e registrarli nei log invece di lasciarli terminare senza essere segnalati. |

TODO stack del frontend

### 1.3 Struttura del database

- Tabella **users** - (id (PK), username (UNIQUE), name, surname, email (UNIQUE), password, created_at)
- Tabella **position_log** - (id (PK), user_id (FK -> users.id), lat, lon, recorded_at)
- Tabella **movement_sessions** - (id (PK), user_id (FK -> users.id), state (CHECK: 'fermo' | 'in_movimento'), started_at, ended_at)
- Tabella **messages** - (id (PK), sender_id (FK -> users.id, nullable), recipient_id (FK -> users.id, nullable), content, sent_at)

Note:
- Nella tabella `messages`, `sender_id` e `recipient_id` permettono di distinguere i diversi tipi di messaggio. Quando entrambi sono `NULL`, il messaggio viene inviato in brodcast dal server a tutti gli utenti. Se invece `sender_id` contiene l'ID di un utente e `recipient_id` è `NULL`, il messaggio è stato inviato da quell'utente al server. Al contrario, quando `sender_id` è `NULL` e `recipient_id` contiene l'ID di un utente, il messaggio è inviato dal server direttamente a quell'utente.
- `movement_sessions` è una tabella derivata che raccoglie le sessioni di movimento a partire dagli eventi di cambio stato. In questo modo, per generare i report non è necessario rielaborare ogni volta l'intero `position_log`. La tabella è inoltre indicizzata su (`user_id`, `started_at`) per velocizzare la ricerca delle sessioni di uno specifico utente che si sovrappongono all'intervallo richiesto.
- Lo stato "disconnesso" non è mai persistito: è rappresentato implicitamente dall'assenza dell'utente dalla mappa delle connessioni attive mantenuta in memoria dal server.

## 2. Aspetti trasversali del server

### 2.1 Variabili d'ambiente

All'avvio dell'applicazione, `main.rs` carica tramite `dotenvy` il file `.env`. L'unica variabile d'ambiente presente è `JWT_SECRET`, utilizzato dal modulo di autenticazione per la firma e la verifica dei token JWT.

### 2.2 Modelli dei dati (models.rs)

Il modulo `models.rs` definisce le principali strutture e enumerazioni utilizzate dal server per rappresentare i dati dell'applicazione, tra cui utenti, posizioni, messaggi, sessioni di movimento e report. Le strutture utilizzate nelle API implementano `Serialize` e `Deserialize`, permettendo lo scambio dei dati in formato JSON. Gli stati sono rappresentati tramite enumerazioni tipizzate. In particolare, `UserState` indica se un utente è `Disconnected`, `Stopped` o `Moving`, mentre `MovementState` viene utilizzato per registrare nel database solo i periodi in cui l'utente è `Stopped` o `Moving`; come già specificato sopra, lo stato `Disconnected` non viene persistito. Per quest'ultima enumerazione è stata implementata la conversione `ToSql`/`FromSql`, che permette a `rusqlite` di salvare e ricostruire direttamente il valore.

### 2.3 Stato condiviso (state.rs)

Lo stato condiviso dell'applicazione è raccolto nella struttura `AppState`, che contiene la connessione al database, il client MQTT e `ActiveUsers`, utilizzata per mantenere in memoria le informazioni sugli utenti attualmente connessi. `ActiveUsers` è definita come `Arc<RwLock<HashMap<i64, UserSession>>>`: `Arc` permette di condividere la struttura tra i diversi task asincroni, mentre `RwLock` ne consente l'accesso concorrente. La mappa utilizza l'identificativo dell'utente come chiave e associa a ciascuno una `UserSession` che contiene l'ultima posizione ricevuta, lo stato corrente e gli istanti dell'ultimo aggiornamento, dell'ultimo cambio di stato e dell'ultimo messaggio accettato. Si sottolinea che lo stato `Disconnected` non viene memorizzato nel database, ma è rappresentato dall'assenza dell'utente da `ActiveUsers`. In questo modo le informazioni necessarie alla gestione in tempo reale rimangono in memoria, mentre nel database vengono persistiti solamente i dati che devono essere conservati.

### 2.4 Comunicazione HTTP e MQTT

La comunicazione HTTP è organizzata tramite route separate nei moduli della cartella `handlers`. Le route vengono poi raccolte nel `main.rs` tramite `Router::merge`. Le risorse protette utilizzano inoltre i middleware di autenticazione trattati nel capitolo sull'autenticazione. Il server HTTP è esposto sulla porta `3001` e utilizza CORS per consentire le richieste provenienti dal client.

La comunicazione MQTT è organizzata nel modulo `outbound.rs`, che gestisce le comunicazioni dal server verso i client attraverso topic distinti per messaggi diretti, broadcast, cambiamenti di stato ed errori. La funzione `publish_json` centralizza la serializzazione dei payload in JSON e la loro pubblicazione con `QoS::AtLeastOnce`. Per i dati inviati dagli utenti invece, il server verifica il JWT tramite `is_mqtt_token_valid`, controllando che l'identificativo presente nei `Claims` corrisponda a quello dell'utente associato alla comunicazione. In questo modo un utente autenticato non può inviare dati a nome di un altro utente.

Il broker utilizzato (`broker.emqx.io:1883`) non utilizza attualmente TLS né autenticazione a livello di broker. Di conseguenza, il traffico MQTT non è cifrato e può essere letto da chiunque abbia accesso al broker. Questa costituisce una limitazione nota dell'implementazione attuale.

TODO: sicurezza https e tls???

### 2.5 Gestione degli errori e logging (error.rs, logging.rs)

Per la gestione degli errori viene utilizzato `anyhow`, che permette di incapsulare e propagare errori di tipo diverso. Le funzioni che possono fallire restituiscono un `Result`, in questo modo l'errore può essere propagato con l'operatore `?` fino al punto in cui viene gestito. Gli errori comunicati dalle API vengono gestiti tramite `ErrorPayload`, che contiene il messaggio nel campo `error` ed è definito nel modulo `errors.rs`. In HTTP viene restituito insieme allo `StatusCode` appropriato, mentre in MQTT viene inviato sul topic dedicato agli errori dell'utente. TODO "errori dell'utente o solo errori?" e poi anyhow lo uso altrove oltre che nel main?

Il sistema di logging è centralizzato nel modulo `logging.rs` e viene inizializzato in `main.rs` all'avvio del server tramite `tracing` e `tracing-subscriber`. Il livello di dettaglio può essere configurato tramite `RUST_LOG`, con `info` utilizzato come valore predefinito. Il logging viene utilizzato sia per segnalare eventi ed errori durante l'esecuzione, sia per monitorare il processo: un task in background registra ogni due minuti l'utilizzo della CPU, il tempo di esecuzione e la memoria occupata dal processo nel file `cpu_usage.log`. Anche eventuali `panic` nei task in background vengono intercettati e registrati.

## N. DAO TODO lo faccio o no???

## N. Autenticazione (auth.rs)

Il modulo di autenticazione gestisce il processo di registrazione e accesso degli utenti, oltre al controllo delle autorizzazioni per le risorse protette.

### N.1 Hashing delle password

Per la gestione delle password viene utilizzata la libreria `Argon2`, che permette di effettuare un hashing progettato specificamente per la protezione delle credenziali. La funzione `hash_password` genera prima un salt casuale tramite `SaltString::generate` utilizzando `OsRng`, un generatore di numeri casuali fornito dal sistema operativo, e successivamente calcola l’hash attraverso `Argon2::default().hash_password`. Il risultato viene memorizzato nel database sotto forma di stringa e contiene già le informazioni necessarie alla successiva verifica, incluso il salt. In fase di autenticazione, `verify_password` ricostruisce l’hash tramite `PasswordHash::new` e utilizza `Argon2::default().verify_password` per verificare la corrispondenza con la password fornita. La password non viene mai memorizzata in chiaro, ma esclusivamente nella sua forma hashata; inoltre, `Argon2` rende ogni operazione di hashing e verifica volutamente onerosa in termini di tempo di calcolo e memoria, aumentando il costo di eventuali attacchi di brute force.

### N.2 Registrazione e creazione degli utenti

La registrazione degli utenti viene gestita da `register_handler`, che permette ad un utente di creare autonomamente il proprio account, mentre `register_by_admin_handler` consente ad un amministratore di registrare un nuovo utente specificandone anche il ruolo, che può essere a sua volta amministratore o meno. Entrambi gli handler delegano la logica alla funzione `create_user`, che centralizza la validazione e la creazione dell’account. La funzione verifica il formato dell’email tramite `validator`, la presenza dei campi obbligatori e i requisiti della password attraverso `validate_password`. Una violazione del vincolo di unicità sull’email viene invece gestita direttamente a livello di database e intercettata come `ConstraintViolation`: questa scelta evita di effettuare un controllo preventivo sull’esistenza dell’email, che non garantirebbe l’atomicità dell’operazione in quanto tra la verifica e il successivo inserimento potrebbero intervenire richieste concorrenti. 

La promozione o la revoca del ruolo di amministratore viene gestita da `update_user_admin_handler`, che modifica il campo `is_admin`. L’eliminazione degli utenti viene invece gestita da `delete_user_handler`. In entrambi i casi, un amministratore non può modificare o eliminare il proprio account.

### N.3 Token JWT e claims

Una volta completata con successo la registrazione o il login, `generate_jwt` genera il token che verrà utilizzato dal client per autenticarsi nelle chiamate successive. Il token viene firmato tramite una chiave segreta recuperata dalla variabile d’ambiente `JWT_SECRET`; la funzione `jwt_secret` ne garantisce inoltre l’inizializzazione una sola volta tramite `OnceLock` e verifica che tale chiave sia presente e non vuota. el token vengono inseriti i `Claims`, composti dall’identificativo dell’utente (`sub`), dal relativo ruolo (`is_admin`) e dalla scadenza (`exp`), impostata a 24 ore dalla generazione. In questo modo, il JWT contiene tutte le informazioni necessarie per identificare e autorizzare l’utente nelle richieste successive, senza richiedere la gestione di una sessione lato server.

### N.4 Autenticazione e autorizzazione tramite middleware

Ad ogni chiamata verso una risorsa protetta, la funzione `authenticate` recupera il token dall’header HTTP `Authorization`, verificando che sia presente nel formato `Bearer <token>`. Il token viene successivamente verificato tramite `verify_jwt`, che utilizza la stessa chiave segreta impiegata durante la generazione e, tramite `Validation::default()`, controlla la validità del token e la relativa scadenza. Questa logica viene utilizzata dai middleware `jwt_auth_middleware` e `jwt_admin_middleware`. Il primo verifica esclusivamente l’autenticazione dell’utente, mentre il secondo controlla anche il valore `is_admin` presente nei `Claims`. La presenza del ruolo direttamente nel token evita quindi di interrogare il database ad ogni chiamata per verificare i privilegi, rendendo il controllo più rapido; il compromesso è che una modifica dei privilegi non invalida automaticamente i token già emessi, che rimangono validi fino alla loro scadenza. Una volta superato il controllo, i `Claims` vengono inseriti nelle `extensions` della `Request` e possono essere recuperati dagli handler tramite `Extension<Claims>`. In questo modo, ad esempio, `me_handler` accede a `claims.sub` per ottenere l’identificativo dell’utente e utilizzarlo nella ricerca sul database, senza dover nuovamente estrarre e verificare il token.

### N.5 Routing

Il router del modulo separa le risorse pubbliche da quelle protette, utilizzando router distinti per i diversi livelli di accesso. `router` raccoglie le rotte pubbliche, a cui unisce `protected_router` e `admin_router`: il primo verifica che l’utente sia autenticato, mentre il secondo controlla che disponga anche dei privilegi di amministratore.

### N.6 Logout e gestione dello stato

Il logout viene gestito lato client, eliminando il JWT memorizzato. Non è presente una blacklist server-side dei token: una volta emesso, il JWT rimane valido fino alla scadenza definita nel campo `exp`. Questa scelta mantiene l’autenticazione stateless, evitando di dover mantenere sul server lo stato delle sessioni o dei token revocati.

## N. Messaggistica

## N. Utilizzo della concorrenza

## N. Frontend

### N.1

## N. Spiegazione di alcune scelte 