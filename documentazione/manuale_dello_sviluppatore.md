# Manuale dello sviluppatore

## 1.Introduzione

### 1.1 Panoramica del progetto

Il progetto è organizzato in due parti: client e server. Il client rappresenta l'applicazione utilizzata dagli utenti e dagli amministratori, mentre il server si occupa di coordinare la comunicazione tra i client, gestire gli utenti, le posizioni, gli stati, i messaggi e la memorizzazione dei dati. Gli amministratori, pur utilizzando client distinti, operano tutti in veste di server e rappresentano quindi un'unica entità nei confronti degli utenti. La comunicazione HTTPS del client web è inoltre gestita tramite CORS, che permette al client di effettuare richieste al server.

La comunicazione tra client e server utilizza protocolli diversi in base al tipo di operazione e di client. Le operazioni come l'autenticazione, la registrazione e la consultazione dello storico dei messaggi utilizzano HTTPS REST per entrambi i client. L'amministratore utilizza HTTPS per attività come la gestione degli utenti, la generazione dei report e l'invio di messaggi diretti o broadcast. Trattandosi di un client utilizzato da una postazione stabile, il modello richiesta-risposta di HTTPS si adatta bene alle interazioni con il server. Il client degli utenti utilizza invece MQTT, dovendo inviare periodicamente al server la propria posizione. La scelta è legata alla natura IoT del client, che può trovarsi in presenza di una connessione meno stabile. MQTT permette di gestire questo tipo di comunicazione senza dover effettuare una nuova richiesta HTTPS per ogni posizione e offrendo inoltre meccanismi di gestione e ritrasmissione dei messaggi. L'utilizzo di MQTT viene esteso anche alle altre comunicazioni del client utente, quali l'invio e la ricezione dei messaggi e la gestione delle notifiche relative ai cambiamenti di stato e agli errori. Questa scelta consente di mantenere un unico meccanismo di comunicazione, evitando di introdurre ulteriori protocolli e sfruttando un approccio coerente con la natura IoT del client.

Il progetto è compatibile con le piattaforme Windows e Linux.

### 1.2 Stack tecnico

| Tecnologia |Utilizzo nel progetto |
|---|---|
| Rust | È il linguaggio richiesto dalle specifiche ed è stato utilizzato per sviluppare l'intero backend. |
| Tokio | Runtime asincrono utilizzato per eseguire il server HTTPS e coordinare le attività in background, tra cui il listener MQTT, il controllo dello stato degli utenti e il logging periodico della CPU, permettendo di gestire queste operazioni in concorrenza senza bloccare il server. |
| rusqlite | Libreria utilizzata per l'accesso al database SQLite. Sono state inoltre aggiunte le feature bundled e chrono. |
| Axum | Framework utilizzato per sviluppare il server HTTPS e gestire le API REST del backend. |
| tower-HTTPS | Fornisce il middleware CORS, utilizzato per gestire le richieste provenienti dal frontend. |
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
| Vite + React + TypeScript | Frontend SPA, vedi §8.2 per il dettaglio completo. |

### 1.3 Struttura del database

- Tabella **users** - (id (PK), name, surname, email (UNIQUE), is_admin (CHECK: 0 | 1), password_hash, created_at)
- Tabella **position_log** - (id (PK), user_id (FK -> users.id), lat (CHECK: -90 <= lat <= 90), lon (CHECK: -180 <= lon <= 180), recorded_at) - idx_position_log_user_time (user_id, recorded_at)
- Tabella **movement_sessions** - (id (PK), user_id (FK -> users.id), state (CHECK: stopped | moving), started_at, ended_at (CHECK: ended_at IS NULL OR ended_at >= started_at)) - idx_movement_sessions_user_time (user_id, started_at)
- Tabella **messages** - (id (PK), sender_id (FK -> users.id, nullable), recipient_id (FK -> users.id, nullable), content, sent_at)

Note:
- Nella tabella `messages`, `sender_id` e `recipient_id` permettono di distinguere i diversi tipi di messaggio. Quando entrambi sono `NULL`, il messaggio viene inviato in brodcast dal server a tutti gli utenti. Se invece `sender_id` contiene l'ID di un utente e `recipient_id` è `NULL`, il messaggio è stato inviato da quell'utente al server. Al contrario, quando `sender_id` è `NULL` e `recipient_id` contiene l'ID di un utente, il messaggio è inviato dal server direttamente a quell'utente.
- `movement_sessions` è una tabella derivata che raccoglie le sessioni di movimento a partire dagli eventi di cambio stato. In questo modo, per generare i report non è necessario rielaborare ogni volta l'intero `position_log`. La tabella è inoltre indicizzata su (`user_id`, `started_at`) per velocizzare la ricerca delle sessioni di uno specifico utente che si sovrappongono all'intervallo richiesto.
- Lo stato "disconnesso" non è mai persistito: è rappresentato implicitamente dall'assenza dell'utente dalla mappa delle connessioni attive mantenuta in memoria dal server.

### 1.4 Avvio dell'applicazione e dimensione dell'eseguibile

Per compilare ed eseguire il progetto sono necessari Rust e Cargo per il backend e Node.js con npm per il frontend. È inoltre necessario disporre di un broker MQTT per la comunicazione tra il server e i client.

Per avviare l'applicazione in ambiente di sviluppo è necessario avviare separatamente il backend e il frontend.
- **Backend**
  ```bash
  cd server
  cargo run --bin georuggine
  ```
- **Frontend**
  ```bash
  cd client
  npm install
  npm run dev
  ```

- **Demo**

  ```bash
  # 0. una tantum: avvia via Docker l'istanza locale di OSRM su localhost:5000
  ./setup_osrm.sh          # oppure, su Windows: .\setup_osrm.ps1
  
  # 1. una tantum: genera l'elenco delle destinazioni (kebab di Torino)
  MAPS_API_KEY="your_actual_api_key" cargo run --bin find_kebabs
  
  # 2. crea 20 utenti di test (password Password123! per tutti)
  cargo run --bin create_users
  
  # 3. genera i tragitti simulati per 60 minuti (richiede OSRM avviato al passo 0)
  cargo run --bin bake_simulation 60
  
  # 4. riproduce la simulazione via MQTT a velocità normale (1x)
  cargo run --bin replay
  ```

  Al termine, il database del server risulterà popolato con posizioni, sessioni di movimento e messaggi realistici per tutti gli utenti di test, utilizzabili per verificare manualmente report, mappe e messaggistica lato admin.

  > Attenzione: se bake_simulation è già stato runnato e quindi i file "position.csv" e "messages.csv" sono già stati popolati basterà runnare ``` cargo run --bin replay ```

TODO dimensione applicazione 

## 2. Aspetti trasversali del server

### 2.1 Variabili d'ambiente

All'avvio dell'applicazione, `main.rs` carica tramite `dotenvy` il file `.env`. Le variabili d'ambiente presenti sono `JWT_SECRET`, utilizzata dal modulo di autenticazione per la firma e la verifica dei token JWT, e le soglie `STALE_AFTER_SECS` e  `DISCONNECT_AFTER_SECS`, che determinano dopo quanti secondi senza variazioni delle coordinate un utente viene considerato rispettivamente fermo e disconnesso.

### 2.2 Modelli dei dati (models.rs)

Il modulo `models.rs` definisce le principali strutture e enumerazioni utilizzate dal server per rappresentare i dati dell'applicazione, tra cui utenti, posizioni, messaggi, sessioni di movimento e report. Le strutture utilizzate nelle API implementano `Serialize` e `Deserialize`, permettendo lo scambio dei dati in formato JSON. Gli stati sono rappresentati tramite enumerazioni tipizzate. In particolare, `UserState` indica se un utente è `Disconnected`, `Stopped` o `Moving`, mentre `MovementState` viene utilizzato per registrare nel database solo i periodi in cui l'utente è `Stopped` o `Moving`; come già specificato sopra, lo stato `Disconnected` non viene persistito. Per quest'ultima enumerazione è stata implementata la conversione `ToSql`/`FromSql`, che permette a `rusqlite` di salvare e ricostruire direttamente il valore.

### 2.3 Stato condiviso (state.rs)

Lo stato condiviso dell'applicazione è raccolto nella struttura `AppState`, che contiene la connessione al database, il client MQTT e `ActiveUsers`, utilizzata per mantenere in memoria le informazioni sugli utenti attualmente connessi. `ActiveUsers` è definita come `Arc<RwLock<HashMap<i64, UserSession>>>`: `Arc` permette di condividere la struttura tra i diversi task asincroni, mentre `RwLock` ne consente l'accesso concorrente. La mappa utilizza l'identificativo dell'utente come chiave e associa a ciascuno una `UserSession` che contiene l'ultima posizione ricevuta, lo stato corrente e gli istanti dell'ultimo aggiornamento, dell'ultimo cambio di stato, dell'ultima variazione di coordinate e dell'ultimo messaggio accettato. Si sottolinea che lo stato `Disconnected` non viene memorizzato nel database, ma è rappresentato dall'assenza dell'utente da `ActiveUsers`. In questo modo le informazioni necessarie alla gestione in tempo reale rimangono in memoria, mentre nel database vengono persistiti solamente i dati che devono essere conservati.
### 2.4 Accesso al database e DAO (cartella dao, cartella database)

L'accesso al database è organizzato tramite i moduli DAO presenti nella cartella `dao`, che espongono le funzioni utilizzate dal resto dell'applicazione per eseguire le operazioni di lettura e scrittura. I vari DAO seguono tutti lo stesso schema: ricevono la connessione condivisa al database e gli eventuali parametri, eseguono le query tramite `rusqlite` e restituiscono il risultato o un errore. 

La connessione al database è gestita nella cartella `database` dal modulo `connection.rs`, che definisce il tipo `pub type SharedDb = Arc<Mutex<Connection>>`. `Arc` consente di condividere la connessione, mentre `Mutex` ne controlla l'accesso evitando operazioni concorrenti sulla stessa `Connection`. La funzione `get_connection` apre il database e abilita le chiavi esterne, mentre `shared_connection` crea lo `SharedDb`, che viene inizializzato in `main.rs` e inserito nello stato condiviso dell'applicazione

Le operazioni `rusqlite` utilizzate dai DAO sono sincrone e vengono eseguite direttamente dagli handler asincroni e dai task MQTT. In caso di aumento significativo del carico, le operazioni bloccanti potrebbero essere isolate tramite `tokio::task::spawn_blocking`, evitando di occupare i worker di Tokio durante l'esecuzione delle query.

### 2.5 Comunicazione HTTPSS e MQTT (cartella handlers, cartella mqtt, cartella certs, tls.rs)

La comunicazione HTTPS è organizzata tramite route separate nei moduli della cartella `handlers`. Le route vengono poi raccolte nel `main.rs` tramite `Router::merge`. Le risorse protette utilizzano inoltre i middleware di autenticazione trattati nel capitolo sull'autenticazione. Il server HTTPS è esposto sulla porta `3001` e utilizza CORS per consentire le richieste provenienti dal client.

La comunicazione MQTT dal server verso i client è gestita dal modulo `outbound.rs`, attraverso topic distinti per messaggi diretti, broadcast, cambiamenti di stato ed errori. La funzione `publish_json` centralizza la serializzazione dei payload in JSON e la loro pubblicazione con `QoS::AtLeastOnce`. Per la comunicazione dai client verso il server, `start_mqtt_listener`, definita in `handler.rs`, si iscrive ai topic dedicati alla posizione e ai messaggi e gestisce la ricezione e l'instradamento dei dati alle relative funzioni. Viene avviata in `main` come task asincrono in background, in modo da gestire continuamente i messaggi MQTT in parallelo al server HTTPS. Prima di elaborare i dati, il server verifica il JWT tramite `is_mqtt_token_valid`, controllando che l'identificativo presente nei `Claims` corrisponda a quello dell'utente associato alla comunicazione. In questo modo un utente autenticato non può inviare dati a nome di un altro utente. La connessione al broker `broker.emqx.io` viene effettuata sulla porta `8883`.

#### 2.5.1 Sicurezza delle comunicazioni

Per proteggere le comunicazioni dell'applicazione sono stati utilizzati protocolli basati su TLS.

La configurazione HTTPS è gestita dal modulo `tls.rs`, che carica il certificato del server e la relativa chiave privata dalla cartella `certs`. La configurazione TLS del server viene attivata nel `main.rs`. In ambiente di sviluppo vengono utilizzati un certificato self-signed e la relativa chiave, generati tramite `mkcert`; ciò permette di utilizzare HTTPS anche localmente, tuttavia il browser può mostrare un avviso relativo all'attendibilità del certificato. Per un eventuale deployment pubblico è sufficiente sostituire questi certificati con quelli emessi da una Certificate Authority riconosciuta.

Per MQTT viene invece utilizzato il certificato CA del broker, salvato sempre nella cartella `certs`, che permette al backend di verificarne l'identità durante la connessione TLS.

### 2.6 Gestione degli errori e logging (error.rs, logging.rs)

Per la gestione degli errori viene utilizzato `anyhow` esclusivamente nel punto di ingresso dell'applicazione, nella funzione `main()`, in modo da gestire errori di tipo diverso. Negli altri livelli dell'applicazione, le funzioni che possono fallire restituiscono un `Result`. Gli errori comunicati dalle API vengono gestiti tramite `ErrorPayload`, che contiene il messaggio nel campo `error` ed è definito nel modulo `errors.rs`. In HTTPS viene restituito insieme allo `StatusCode` appropriato, mentre in MQTT viene inviato sul topic dedicato agli errori dell'utente.

Il sistema di logging è centralizzato nel modulo `logging.rs` e viene inizializzato in `main.rs` all'avvio del server tramite `tracing` e `tracing-subscriber`. Il livello di dettaglio può essere configurato tramite `RUST_LOG`, con `info` utilizzato come valore predefinito. Il logging viene utilizzato sia per segnalare eventi ed errori durante l'esecuzione, sia per monitorare il processo: un task in background registra ogni due minuti l'utilizzo della CPU, il tempo di esecuzione e la memoria occupata dal processo nel file `cpu_usage.log`. Anche eventuali `panic` nei task in background vengono intercettati e registrati.

## 3. Autenticazione (auth.rs)

Il modulo di autenticazione gestisce il processo di registrazione e accesso degli utenti, oltre al controllo delle autorizzazioni per le risorse protette.

### 3.1 Hashing delle password

Per la gestione delle password viene utilizzata la libreria `Argon2`, che permette di effettuare un hashing progettato specificamente per la protezione delle credenziali. La funzione `hash_password` genera prima un salt casuale tramite `SaltString::generate` utilizzando `OsRng`, un generatore di numeri casuali fornito dal sistema operativo, e successivamente calcola l’hash attraverso `Argon2::default().hash_password`. Il risultato viene memorizzato nel database sotto forma di stringa e contiene già le informazioni necessarie alla successiva verifica, incluso il salt. In fase di autenticazione, `verify_password` ricostruisce l’hash tramite `PasswordHash::new` e utilizza `Argon2::default().verify_password` per verificare la corrispondenza con la password fornita. La password non viene mai memorizzata in chiaro, ma esclusivamente nella sua forma hashata; inoltre, `Argon2` rende ogni operazione di hashing e verifica volutamente onerosa in termini di tempo di calcolo e memoria, aumentando il costo di eventuali attacchi di brute force.

### 3.2 Registrazione e creazione degli utenti

La registrazione degli utenti viene gestita da `register_handler`, che permette ad un utente di creare autonomamente il proprio account, mentre `register_by_admin_handler` consente ad un amministratore di registrare un nuovo utente specificandone anche il ruolo, che può essere a sua volta amministratore o meno. Entrambi gli handler delegano la logica alla funzione `create_user`, che centralizza la validazione e la creazione dell’account. La funzione verifica il formato dell’email tramite `validator`, la presenza dei campi obbligatori e i requisiti della password (minimo 12 caratteri, di cui almeno una lettera maiuscola e almeno un carattere non alfanumerico) attraverso `validate_password`. Una violazione del vincolo di unicità sull’email viene invece gestita direttamente a livello di database e intercettata come `ConstraintViolation`: questa scelta evita di effettuare un controllo preventivo sull’esistenza dell’email, che non garantirebbe l’atomicità dell’operazione in quanto tra la verifica e il successivo inserimento potrebbero intervenire richieste concorrenti. 

### 3.3 Token JWT e claims

Una volta completata con successo la registrazione o il login, `generate_jwt` genera il token che verrà utilizzato dal client per autenticarsi nelle chiamate successive. Il token viene firmato tramite una chiave segreta recuperata dalla variabile d’ambiente `JWT_SECRET`; la funzione `jwt_secret` ne garantisce inoltre l’inizializzazione una sola volta tramite `OnceLock` e verifica che tale chiave sia presente e non vuota. el token vengono inseriti i `Claims`, composti dall’identificativo dell’utente (`sub`), dal relativo ruolo (`is_admin`) e dalla scadenza (`exp`), impostata a 24 ore dalla generazione. In questo modo, il JWT contiene tutte le informazioni necessarie per identificare e autorizzare l’utente nelle richieste successive, senza richiedere la gestione di una sessione lato server.

### 3.4 Autenticazione e autorizzazione tramite middleware

Ad ogni chiamata verso una risorsa protetta, la funzione `authenticate` recupera il token dall’header HTTPS `Authorization`, verificando che sia presente nel formato `Bearer <token>`. Il token viene successivamente verificato tramite `verify_jwt`, che utilizza la stessa chiave segreta impiegata durante la generazione e, tramite `Validation::default()`, controlla la validità del token e la relativa scadenza. Questa logica viene utilizzata dai middleware `jwt_auth_middleware` e `jwt_admin_middleware`. Il primo verifica esclusivamente l’autenticazione dell’utente, mentre il secondo controlla anche il valore `is_admin` presente nei `Claims`. La presenza del ruolo direttamente nel token evita quindi di interrogare il database ad ogni chiamata per verificare i privilegi, rendendo il controllo più rapido; il compromesso è che una modifica dei privilegi non invalida automaticamente i token già emessi, che rimangono validi fino alla loro scadenza. Una volta superato il controllo, i `Claims` vengono inseriti nelle `extensions` della `Request` e possono essere recuperati dagli handler tramite `Extension<Claims>`. In questo modo, ad esempio, `me_handler` accede a `claims.sub` per ottenere l’identificativo dell’utente e utilizzarlo nella ricerca sul database, senza dover nuovamente estrarre e verificare il token.

### 3.5 Logout e gestione dello stato

Il logout viene gestito lato client, eliminando il JWT memorizzato. Non è presente una blacklist server-side dei token: una volta emesso, il JWT rimane valido fino alla scadenza definita nel campo `exp`. Questa scelta mantiene l’autenticazione stateless, evitando di dover mantenere sul server lo stato delle sessioni o dei token revocati.

## 4. Gestione degli utenti (handlers/users.rs)

Il modulo `users.rs` gestisce il recupero e le principali operazioni amministrative sugli utenti.

La funzione `get_users_handler` permette agli amministratori di recuperare l'elenco degli utenti registrati, applicando criteri di ricerca, ordinamento e filtraggio per ruolo amministrativo. La funzione supporta inoltre la paginazione attraverso i parametri `limit`, che determina il numero massimo di utenti restituiti, e `offset`, che indica quanti risultati saltare. Il valore predefinito di `limit` è 10 e viene comunque limitato a un massimo di 100 utenti per richiesta. Le informazioni recuperate dal database vengono completate con lo stato corrente dell'utente. Se il suo identificativo è presente nella struttura `ActiveUsers`, viene utilizzato lo stato memorizzato nella struttura; in caso contrario, l'utente viene considerato `Disconnected`.

La funzione `me_handler` permette invece a un generico utente autenticato di recuperare le proprie informazioni, utilizzando l'identificativo contenuto nei `Claims` del JWT.

La promozione o la revoca del ruolo di amministratore viene gestita da `update_user_admin_handler`, che modifica il campo `is_admin`. L'eliminazione degli utenti viene invece gestita da `delete_user_handler`. In entrambi i casi, un amministratore non può modificare o eliminare il proprio account. Inoltre, dopo l'eliminazione, l'utente viene rimosso anche dalla struttura `ActiveUsers`.

## 5. Posizione e movimento (mqtt/handlers.rs)

La gestione della posizione e la gestione dello stato di movimento sono strettamente collegate: le posizioni ricevute periodicamente dal client permettono infatti al server di determinare se un utente è fermo o in movimento e di costruire, nel tempo, le sessioni necessarie alla successiva generazione dei report.

### 5.1 Ricezione e gestione della posizione

La ricezione delle posizioni viene gestita da `start_mqtt_listener`, che sottoscrive il client MQTT del server al topic `georuggine/client/+/position`. Quando arriva un aggiornamento da parte dell'utente, la funzione deserializza il payload in un `PositionUpdatePayload` e delega la gestione della posizione a `handle_position_update`. 

`handle_position_update` utilizza `validate_coordinates` per controllare che latitudine e longitudine siano valori finiti e rientrino nei rispettivi intervalli geografici validi. In caso di errore, la posizione viene scartata e viene inviata al client una notifica tramite MQTT. Inoltre se non c'è una sessione attiva in memoria (e quindi l'utente viene osservato per la prima volta), viene inizializzata una `UserSession` nello stato `Stopped` e viene creata nel database la relativa sessione iniziale. Lo stato `Stopped` viene utilizzato come stato iniziale, in quanto alla prima posizione non è ancora possibile determinare un eventuale movimento, non essendoci una posizione precedente con cui confrontare le coordinate. Superati i controlli, `handle_position_update` associa alla posizione l'istante corrente in UTC e la salva nella tabella `position_log` nel database. Successivamente passa la nuova `Position` a `update_session_position`, che aggiorna nella struttura `ActiveUsers` l'istante dell'ultimo segnale ricevuto dall'utente. La funzione `update_session_position` confronta inoltre la nuova posizione con quella precedentemente memorizzata: le coordinate vengono considerate cambiate se la differenza assoluta tra latitudini oppure tra longitudini supera la soglia `COORD_EPSILON`. In caso di variazione, viene aggiornato l'istante dell'ultima variazione delle coordinate.

### 5.2 Gestione dello stato di movimento

Sulla base del cambiamento delle coordinate rilevato da `update_session_position`, il server determina l'eventuale transizione dello stato di movimento tramite `check_state_transition`. La funzione considera due casi: se le coordinate sono cambiate, restituisce lo stato `Moving` associato all'istante corrente; se invece non sono cambiate, verifica il tempo trascorso dall'ultima variazione e quando questo raggiunge i 3 minuti definiti dalla costante `STALE_AFTER_SECS`, restituisce lo stato `Stopped`. In questo caso l'istante della transizione viene impostato a `last_coord_change_at`, cioè al momento dell'ultima variazione effettiva delle coordinate, e non all'istante in cui il server rileva il superamento dei tre minuti.

Il risultato restituito da `check_state_transition` viene quindi elaborato nuovamente da `update_session_position`, che confronta l'eventuale nuovo stato determinato con quello attualmente memorizzato nella `UserSession`. Solo quando i due stati sono differenti, la funzione aggiorna lo stato della sessione e l'istante associato alla variazione, restituendo a `handle_position_update` l'eventuale cambiamento rilevato.

Quando `update_session_position` restituisce una variazione dello stato, `handle_position_update` utilizza `transition_session` per chiudere la sessione precedentemente aperta e crearne una nuova con il nuovo `MovementState`, utilizzando come istante di transizione quello determinato dalla logica precedente. Le due operazioni vengono eseguite all'interno di una singola transazione, in modo da mantenere coerente la sequenza delle sessioni.

La gestione dello stato di movimento non dipende però esclusivamente dalla ricezione di una nuova posizione. Per gestire il caso in cui non arrivino nuovi aggiornamenti, il server esegue in background `stale_state_watcher`, che controlla periodicamente gli utenti presenti nella struttura `ActiveUsers`. Il watcher distingue tra l'assenza di cambiamenti nelle coordinate e l'assenza completa di aggiornamenti. Se l'utente continua a inviare posizioni, ma le coordinate rimangono invariate per almeno 3 minuti, viene considerato `Stopped`; `stale_state_watcher` aggiorna quindi lo stato della relativa `UserSession` e utilizza `last_coord_change_at` come istante della variazione. Diversamente, se non viene ricevuto alcun aggiornamento dall'utente, né una posizione né un messaggio MQTT, per almeno 2 minuti, il problema non riguarda più il movimento ma la comunicazione con il client. In questo caso l'utente viene rimosso dalla struttura `ActiveUsers` e l'eventuale sessione di movimento ancora aperta viene chiusa tramite `close_movement_session_for_user`. Non viene aperta una nuova sessione, poiché, come già scritto sopra, lo stato `Disconnected` non viene registrato nella tabella `movement_sessions`.

Quando viene rilevata una variazione dello stato, sia `handle_position_update` che `stale_state_watcher` utilizzano `notify_state_change` per comunicare il nuovo stato al client tramite MQTT. La notifica contiene l'identificativo dell'utente, lo stato risultante e l'istante associato alla transizione.

## 6. Reportistica (handlers/report.rs)

La reportistica permette di analizzare il movimento di un utente su un intervallo temporale definito, ricostruendo i tragitti percorsi e calcolandone le principali informazioni.

La generazione del report viene gestita da `get_report_handler`, che riceve l'identificativo dell'utente per il quale si vuole effettuare l'analisi e il periodo da considerare e delega la costruzione del risultato a `build_report`. La funzione `get_start_end_from_report_period` determina quindi l'intervallo temporale corrispondente al periodo richiesto: per il giorno considera la giornata corrente, per la settimana considera la settimana corrente a partire da lunedì e per il mese considera il mese corrente. L'intervallo termina sempre all'istante in cui viene richiesto il report. Dopo aver verificato che l'utente esista, `build_report` recupera le posizioni registrate per quell'utente nell'intervallo tramite `get_positions_in_range` e le sessioni di movimento e di pausa tramite `get_sessions_in_range`.

A partire dalle sessioni e dalle posizioni recuperate, `build_segments` raggruppa le posizioni all'interno delle rispettive sessioni `Moving`, escludendo le sessioni `Stopped`. In questo modo il tragitto viene rappresentato come una sequenza di segmenti distinti e non viene collegata artificialmente la fine di una sessione con l'inizio di quella successiva. La velocità media viene quindi calcolata da `compute_avg_speed_kmh` considerando solo le coppie consecutive appartenenti allo stesso segmento. Per ogni coppia la distanza geografica viene calcolata tramite `haversine_distance_km`; vengono escluse le coppie con un intervallo superiore a 90 secondi e quelle con una distanza inferiore a 5 metri, considerate rispettivamente non contigue e rumore GPS. La velocità ottenuta viene rapportata alla durata complessiva delle sessioni `Moving`. Le durate del movimento e delle pause vengono invece calcolate da `compute_durations` a partire dalle `MovementSession`, sommando separatamente la durata delle sessioni `Moving` e `Stopped` e considerando solo la parte di ciascuna sessione compresa nell'intervallo richiesto.

Inoltre, la funzione `get_own_positions_handler` permette agli utenti autenticati di recuperare le proprie posizioni relative alla sessione di movimento attualmente aperta, considerando un intervallo che parte da 60 secondi prima dell’inizio della sessione e termina al momento della richiesta.

## 7. Messaggistica (handlers/messages.rs, mqtt/handlers.rs)

Il sistema di messaggistica gestisce l'invio e la ricezione di messaggi diretti e broadcast. HTTPS viene utilizzato dall'amministratore per l'invio dei messaggi e da entrambi i client per la consultazione dello storico. MQTT viene invece utilizzato per lo scambio dei messaggi tra gli utenti e il server.

### 7.1 Invio e recupero dei messaggi tramite HTTPS

Il modulo `messages.rs` espone le route dedicate alla messaggistica. `get_messages_handler` distingue innanzitutto il tipo di richiesta in base al ruolo dell'utente e al parametro `with`. Per un amministratore, `with` identifica l'utente con cui visualizzare la conversazione diretta; se non viene specificato, vengono invece recuperati i soli messaggi broadcast. Per un utente normale non è necessario specificare `with`, perché vengono recuperati automaticamente i messaggi diretti che lo riguardano insieme ai broadcast. Il parametro `limit` stabilisce il numero massimo di messaggi restituiti: se non viene specificato viene utilizzato il valore predefinito di 50. `clamp(1, MAX_LIMIT)` limita comunque il valore tra 1 e 200. La differenza nella gestione delle conversazioni rispecchia le esigenze delle due interfacce: mentre l'amministratore ha le chat con tutti gli utenti, l'utente normale ha solo una chat con l'amministratore. 

L'invio tramite HTTPS è invece riservato agli amministratori: `post_direct_message` verifica l'esistenza del destinatario, salva il messaggio nel database e ne notifica la ricezione tramite MQTT, mentre `post_broadcast_handler` salva e pubblica un messaggio destinato a tutti gli utenti. Entrambe le funzioni utilizzano `validate_content` per verificare che il messaggio non sia vuoto e non superi i 1000 caratteri. I messaggi vengono quindi prima persistiti nel database e solo successivamente notificati tramite MQTT, mantenendo lo storico disponibile anche nel caso in cui la pubblicazione MQTT non vada a buon fine.

### 7.2 Ricezione e invio dei messaggi tramite MQTT

I messaggi inviati dal server vengono ricevuti dagli utenti tramite topic MQTT dedicati. I messaggi diretti vengono pubblicati sul topic associato al singolo utente, mentre i broadcast utilizzano un topic comune a tutti gli utenti.

Quando un utente invia un messaggio tramite MQTT, questo viene invece ricevuto dal server sul canale dedicato all'utente e passato a `handle_user_message` per la gestione e il salvataggio. Prima di procedere, viene verificato che l'utente non abbia già inviato un altro messaggio nell'ultimo secondo, applicando un rate limit per evitare un invio eccessivo di messaggi, sia per limitare il carico sul server e sul database sia per ridurre il rischio di attacchi basati sull'invio massivo di richieste. Se il controllo viene superato, il messaggio viene salvato nel database associandolo all'utente come mittente. In questo modo i messaggi ricevuti tramite MQTT vengono persistiti nello stesso storico utilizzato dai messaggi inviati tramite HTTPS. Si sottolinea che anche la ricezione di un messaggio costituisce un segnale di vita dell'utente e aggiorna quindi `last_seen_at`; in questo modo si evita di considerare l'utente come disconnesso finché continuano ad arrivare messaggi.

## 8. Frontend

### 8.1 Panoramica

Il frontend è un'applicazione web realizzata con **React 18**, **TypeScript** e **Vite**. L'interfaccia è organizzata in due aree distinte, con un design system basato su **Tailwind CSS** e componenti in stile *glassmorphism*.

### 8.2 Stack tecnico

| Tecnologia | Utilizzo nel progetto |
|---|---|
| React 18 | Framework UI per la costruzione dell'interfaccia utente tramite componenti funzionali e hook. |
| TypeScript | Tipizzazione statica di tutto il codice sorgente, inclusi modelli dati, props dei componenti e risposte API. |
| Vite | Build tool e dev server; fornisce HMR rapido e bundling ottimizzato per la produzione (`npm run build`). |
| React Router DOM | Gestione del routing lato client; le route protette verificano autenticazione e ruolo admin. |
| Tailwind CSS | Utility-first CSS framework; il tema è personalizzato tramite direttive `@theme` con palette scura e variabili per il design glassmorphism. |
| Framer Motion | Animazioni di entrata, transizioni tra pagine e micro-interazioni (hover, tap, scroll). |
| Leaflet + React-Leaflet | Visualizzazione delle mappe interattive per il tracciamento della posizione singola e della flotta. |
| Axios | Client HTTP per le chiamate REST al backend; configurato con interceptor per il JWT e gestione centralizzata degli errori. |
| MQTT.js | Client MQTT che opera su WebSocket (`wss://broker.emqx.io:8084/mqtt`) per la pubblicazione di messaggi e posizioni in tempo reale. |
| Lucide React | Libreria di icone utilizzata in tutta l'applicazione per garantire coerenza visiva. |

### 8.3 Struttura del progetto

Il codice sorgente è organizzato nella cartella `src/` secondo il seguente schema:

```
src/
├── components/
│   ├── auth/           # Form di login e registrazione
│   ├── dashboard/      # Mappe (singola utente e flotta), sidebar utente
│   ├── layout/         # Navbar con navigazione condizionale admin/user
│   ├── messages/       # Sidebar conversazioni e finestra chat
│   └── ui/             # Componenti riutilizzabili (GlassCard, FormInput, AnimatedBackground, ...)
├── context/
│   └── AuthContext.tsx # Gestione globale dello stato di autenticazione (JWT, ruolo, userId)
├── hooks/
│   └── useMqttClient.ts# Hook per la connessione e pubblicazione MQTT
├── lib/
│   └── api.ts          # Istanza Axios configurata con base URL, interceptor JWT e gestione errori
├── pages/
│   ├── AuthPage.tsx           # Pagina di accesso (login + registrazione affiancati)
│   ├── DashboardPage.tsx      # Dashboard utente con mappa personale e stato
│   ├── MessagesPage.tsx       # Messaggistica utente (admin e broadcast)
│   └── admin/
│       ├── AdminDashboardPage.tsx  # Panoramica flotta, stats e mappa multi-utente
│       ├── AdminUsersPage.tsx      # Gestione, filtro, promozione e eliminazione utenti
│       ├── AdminReportsPage.tsx    # Generazione report per singolo utente con mappa e metriche
│       └── AdminMessagesPage.tsx   # Messaggistica admin (diretta e broadcast)
├── types/              # Tipi TypeScript condivisi (User, messaggi, coordinate, ...)
├── App.tsx             # Router principale, route protette e banner errori globali
├── main.tsx            # Entry point con StrictMode
└── index.css           # Tailwind + design system custom (glass, input, bottoni, griglia)
```

### 8.4 Design system e UI

L'interfaccia utilizza un tema scuro uniforme basato su una palette di grigi profondi (`#050505` background, `#111` surface) con accenti bianchi e colori di stato (emerald per *moving*, amber per *stopped*, neutral per *disconnected*).

I componenti fondamentali del design system sono:

- **`GlassCard`**: contenitore con sfondo semi-trasparente, `backdrop-filter: blur`, bordo sottile e ombre stratificate. Supporta varianti (`default`, `hover`, `interactive`, `subtle`) per adattarsi a contesti diversi (card cliccabili, sidebar, chat).
- **`AnimatedBackground`**: sfondo fisso con gradienti radiali animati (Framer Motion) e griglia sottile, applicato a tutte le pagine per dare profondità senza distrarre.
- **`FormInput`** e **`glass-input`**: campi di input con icona, stile glass e stati focus con bordo luminoso.
- **`btn-primary` / `btn-secondary`**: bottoni con stile pieno (bianco su nero) o outlined, usati rispettivamente per azioni principali e secondarie.

### 8.5 Autenticazione e routing

L'autenticazione è gestita interamente lato client tramite **JWT** memorizzato in `localStorage`. 

L' `AuthContext`:
- All'avvio legge il token, ne decodifica il payload (campi `sub`/`user_id`, `is_admin`) e inizializza lo stato globale.
- Fornisce le funzioni `login(token)` e `logout()`.
- Reindirizza automaticamente gli admin alla route `/admin` se tentano di accedere alla root `/`.

Il routing in `App.tsx` protegge le route tramite il componente `ProtectedRoute`, che verifica `isAuthenticated` e, per le sezioni admin, il flag `isAdmin`. Le chiamate API che ricevono HTTP 401 attivano un interceptor che cancella il token e reindirizza al login.

### 8.6 Comunicazione con il backend

#### 8.6.1 API REST (`lib/api.ts`)

Il modulo `api.ts` crea un'istanza Axios con:
- `baseURL` letto dalla variabile d'ambiente `VITE_API_URL` (default: `https://127.0.0.1:3001`).
- **Request interceptor**: aggiunge l'header `Authorization: Bearer <token>` se presente in `localStorage`.
- **Response interceptor**: in caso di 401 effettua il logout automatico; per altri errori emette un evento globale `app-error` che viene visualizzato dal banner in `App.tsx`.

#### 8.6.2 MQTT (`hooks/useMqttClient.ts`)

L'hook `useMqttClient` gestisce una singola connessione MQTT over WebSocket verso `wss://broker.emqx.io:8084/mqtt`. Al mount crea un client con `clean: true`, riconnessione automatica ogni 5 secondi e keepalive di 60 secondi. Espone:

- `connected`: stato della connessione.
- `publish(topic, payload)`: serializza il payload in JSON e pubblica con QoS 1, restituendo una Promise booleana.

L'hook viene utilizzato in `MessagesPage.tsx` per permettere agli utenti di inviare messaggi al server tramite il topic `georuggine/client/:user_id/message`, includendo nel payload il JWT per la verifica lato server.

### 8.7 Pagine principali

#### 8.7.1 Autenticazione (`AuthPage`)

Pagina di ingresso non protetta. Presenta affiancati il form di login e quello di registrazione, separati da un divisore diagonale animato. Entrambi i form utilizzano `FormInput` con icone Lucide e validazione lato server; al successo del login il token viene salvato e l'utente reindirizzato alla dashboard appropriata.

#### 8.7.2 Dashboard utente (`DashboardPage`)

Layout a due colonne: sidebar sinistra (`UserSidebar`) con dati profilo, stato di movimento e coordinate; area destra (`MapView`) con mappa Leaflet in tema scuro (tile CARTO dark) che mostra la posizione corrente e la traiettoria della sessione aperta.

Il polling avviene ogni 30 secondi: una chiamata a `/api/me` aggiorna lo stato, mentre `/api/me/positions` recupera le posizioni della sessione corrente. Quando l'utente passa da *disconnected* a online, la traiettoria precedente viene azzerata per ricominciare il tracciamento dalla nuova sessione.

#### 8.7.3 Messaggistica utente (`MessagesPage`)

Interfaccia chat con sidebar a sinistra (due voci fisse: *Admin* e *Broadcast*) e finestra conversazione a destra (`ChatWindow`). I messaggi vengono recuperati da `/api/messages` con polling ogni 5 secondi. L'invio verso l'admin utilizza MQTT (topic `georuggine/client/:user_id/message`); il canale broadcast è in sola lettura.

#### 8.7.4 Dashboard admin (`AdminDashboardPage`)

Panoramica della flotta in tempo reale. In alto sono visualizzate quattro card riassuntive (utenti totali, in movimento, attivi, link ai report) che navigano alle rispettive sezioni. L'area principale ospita `FleetMapView`, una mappa multi-utente che traccia fino a 4 veicoli selezionabili da un dropdown. Per ogni utente selezionato viene chiamato `/api/report?period=day` e le sessioni di movimento (`segments`) vengono appiattite in un'unica traiettoria colorata. Il refresh è configurabile (default 10 secondi).

#### 8.7.5 Gestione utenti (`AdminUsersPage`)

Pagina divisa in due pannelli: a sinistra il form per registrare nuovi utenti (anche admin) tramite `POST /api/admin/register`; a destra la lista utenti con ricerca testuale, filtri per ruolo e stato, toggle admin e eliminazione. Le azioni su sé stessi sono disabilitate. La lista è virtualmente scrollabile e mostra badge di ruolo, indicatore di stato e pulsanti azione.

#### 8.7.6 Report (`AdminReportsPage`)

Strumento di analisi per singolo utente. L'admin seleziona un utente da un dropdown con ricerca, sceglie la granularità (giorno/settimana/mese) e genera il report. Il risultato mostra:
- **Metriche**: velocità media, tempo in movimento, tempo in pausa.
- **Mappa**: una Polyline per ogni sessione di movimento (colori diversi per sessione), marker di partenza (verde) e ultima posizione (ambra).
- **Legenda**: spiegazione dei colori e conteggio delle sessioni.

#### 8.7.7 Messaggistica admin (`AdminMessagesPage`)

Simile alla pagina utente ma con funzionalità estese: la sidebar mostra tutti gli utenti non-admin con indicatore di stato; l'admin può selezionare un utente per conversazione diretta (via `POST /api/messages/direct`) o il canale broadcast (via `POST /api/broadcast`). Lo storico viene aggiornato con polling ogni 3 secondi.

### 8.8 Build e avvio

Per l'ambiente di sviluppo:
```bash
cd client
npm install
npm run dev
```

Per la build di produzione:
```bash
npm run build
```

L'output viene generato nella cartella `dist/` e può essere servito da qualsiasi web server statico. Il backend HTTPS deve essere raggiungibile all'indirizzo configurato in `VITE_API_URL`. TODO vedere che fare


## 9. API

### 9.1 HTTPS

- **POST `/api/register`**
  - Request body:
    ```json
    {
      "name": "Marco",
      "surname": "Rossi",
      "email": "mrossi@example.com",
      "password": "Marco01!password",
      "is_admin": false
    }
    ```
  - Response 201 Created:
    ```json
    {
      "token": "JWT_TOKEN"
    }
    ```
  - Response 400 Bad Request: `{"error": "Dati non validi: ..."}` oppure `{"error": "Non sono stati inseriti tutti i dati richiesti"}` oppure `{"error": "La password deve contenere almeno 12 caratteri, una lettera maiuscola e un simbolo"}`
  - Response 409 Conflict: `{"error": "Questa email è già registrata"}`
  - Response 500 Internal Server Error: `{"error": "Impossibile completare la registrazione. Riprova più tardi"}` oppure `{"error": "L'account è stato creato, ma è impossibile completare l'accesso. Riprova più tardi"}`
- **POST `/api/login`**
  - Request body:
    ```json
    {
      "email": "mrossi@example.com",
      "password": "Marco01!password"
    }
    ```
  - Response 200 OK:
    ```json
    {
      "token": "JWT_TOKEN"
    }
    ```
  - Response 401 Unauthorized: `{"error": "Credenziali non valide"}`
  - Response 500 Internal Server Error: `{"error": "Impossibile completare il login. Riprova più tardi"}`
- **POST `/api/admin/register`**
  - Request body:
    ```json
    {
      "name": "Luigi",
      "surname": "Bianchi",
      "email": "lbianchi@example.com",
      "password": "Luigi01!password",
      "is_admin": true
    }
    ```
  - Response 201 Created:
    ```json
    {
      "id": 2,
      "name": "Luigi",
      "surname": "Bianchi",
      "email": "lbianchi@example.com",
      "is_admin": true,
      "created_at": "2026-06-20T14:30:45Z"
    }
    ```
  - Response 400 Bad Request: `{"error": "Dati non validi: ..."}` oppure `{"error": "Non sono stati inseriti tutti i dati richiesti"}` oppure `{"error": "La password deve contenere almeno 12 caratteri, una lettera maiuscola e un simbolo"}`
  - Response 401 Unauthorized: `{"error": "L'utente non ha effettuato l'accesso"}`
  - Response 403 Forbidden: `{"error": "Accesso riservato agli amministratori"}`
  - Response 409 Conflict: `{"error": "Questa email è già registrata"}`
  - Response 500 Internal Server Error: `{"error": "Impossibile completare la registrazione. Riprova più tardi"}`
- **GET `/api/me`**
  - Response 200 OK: 
    ```json
    {
      "id": 1,
      "name": "Marco",
      "surname": "Rossi",
      "email": "mrossi@example.com",
      "is_admin": false,
      "created_at": "2026-06-20T14:30:45Z"
    }
    ```
  - Response 401 Unauthorized: `{"error": "L'utente non ha effettuato l'accesso"}`
  - Response 404 Not Found: `{"error": "Utente non trovato"}`
  - Response 500 Internal Server Error: `{"error": "Errore del server"}`
- **GET `/api/users`**
  - Query parameters:
    - `order_by_field`
    - `order_by_dir`
    - `search`
    - `is_admin`
    - `limit`
    - `offset`
  - Response 200 OK:
    ```json
    [
      {
        "id": 1,
        "name": "Marco",
        "surname": "Rossi",
        "email": "mrossi@example.com",
        "created_at": "2026-06-20T14:30:45Z",
        "state": "moving",
        "is_admin": false
      }
    ]
    ```
  - Response 401 Unauthorized: `{"error": "L'utente non ha effettuato l'accesso"}`
  - Response 403 Forbidden: `{"error": "Accesso riservato agli amministratori"}`
  - Response 500 Internal Server Error: `{"error": "Impossibile recuperare gli utenti"}`
- **DELETE `/api/admin/users/:user_id`**
  - Response 204 No Content: No response body
  - Response 400 Bad Request: `{"error": "Non puoi eliminare il tuo stesso account"}`
  - Response 401 Unauthorized: `{"error": "L'utente non ha effettuato l'accesso"}`
  - Response 403 Forbidden: `{"error": "Accesso riservato agli amministratori"}`
  - Response 404 Not Found: `{"error": "Utente non trovato"}`
  - Response 500 Internal Server Error: `{"error": "Impossibile eliminare l'utente. Riprova più tardi"}`
- **PUT `/api/admin/users/:user_id/admin`**
  - Request body:
    ```json
    {
      "is_admin": true
    }
    ```
  - Response 204 No Content: No response body
  - Response 400 Bad Request: `{"error": "Non puoi modificare il tuo stesso account"}`
  - Response 401 Unauthorized: `{"error": "L'utente non ha effettuato l'accesso"}`
  - Response 403 Forbidden: `{"error": "Accesso riservato agli amministratori"}`
  - Response 404 Not Found: `{"error": "Utente non trovato"}`
  - Response 500 Internal Server Error: `{"error": "Impossibile aggiornare l'utente. Riprova più tardi"}`
- **GET `/api/messages`**
  - Query parameters:
    - `with`
    - `limit`
    - `offset`
  - Response 200 OK:
    ```json
    [
      {
        "id": 15,
        "sender_id": 1,
        "recipient_id": 2,
        "content": "Messaggio per il camionista",
        "sent_at":: "2026-06-20T14:30:45Z"
      },
      {
        "id": 16,
        "sender_id": null,
        "recipient_id": null,
        "content": "Messaggio per tutti i camionisti",
        "sent_at": "2026-06-20T14:35:12Z"
      }
    ]
    ```
  - Response 401 Unauthorized: `{"error": "L'utente non ha effettuato l'accesso"}`
  - Response 500 Internal Server Error: `{"error": "Impossibile recuperare i messaggi"}`
- **POST `/api/messages/direct`**
  - Request body:
    ```json
    {
      "recipient_id": 2,
      "content": "Messaggio per il camionista"
    }
    ```
  - Response 200 OK:
    ```json
    {
      "id": 15,
      "queued": true
    }
    ```
  - Response 400 Bad Request: `{"error": "Il contenuto del messaggio non può essere vuoto"}` oppure `{"error": "Messaggio troppo lungo (max 1000 caratteri)"}`
  - Response 401 Unauthorized: `{"error": "L'utente non ha effettuato l'accesso"}`
  - Response 403 Forbidden: `{"error": "Accesso riservato agli amministratori"}`
  - Response 404 Not Found: `{"error": "Destinatario non trovato"}`
  - Response 500 Internal Server Error: `{"error": "Impossibile inviare il messaggio"}` oppure `{"error": "Impossibile salvare il messaggio"}`
- **POST `/api/broadcast`**
  - Request body:
    ```json
    {
      "content": "Messaggio per tutti i camionisti"
    }
    ```
  - Response 200 OK:
    ```json
    {
      "id": 16,
      "queued": true
    }
    ```
  - Response 400 Bad Request: `{"error": "Il contenuto del messaggio non può essere vuoto"}` oppure `{"error": "Messaggio troppo lungo (max 1000 caratteri)"}`
  - Response 401 Unauthorized: `{"error": "L'utente non ha effettuato l'accesso"}`
  - Response 403 Forbidden: `{"error": "Accesso riservato agli amministratori"}`
  - Response 500 Internal Server Error: `{"error": "Impossibile salvare il messaggio"}`
- **GET `/api/report`**
  - Query parameters:
    - `user_id`
    - `period`
  - Response 200 OK:
    ```json
    {
      "user_id": 1,
      "period": "day",
      "segments": [
        {
          "lat": 45.0703,
          "lon": 7.6869,
          "recorded_at": "2026-06-20T14:30:45Z"
        },
        {
          "lat": 45.0721,
          "lon": 7.6895,
          "recorded_at": "2026-06-20T14:31:20Z"
        },
      ],
      "avg_speed_kmh": 42.5,
      "movement_duration_secs": 3600,
      "pause_duration_secs": 600
    }
    ```
  - Response 401 Unauthorized: `{"error": "L'utente non ha effettuato l'accesso"}`
  - Response 403 Forbidden: `{"error": "Accesso riservato agli amministratori"}`
  - Response 404 Not Found: `{"error": "Utente non trovato"}`
  - Response 500 Internal Server Error: `{"error": "Errore del server"}` oppure `{"error": "Impossibile calcolare il tragitto"}` oppure `{"error": "Impossibile calcolare le durate del movimento e delle pause"}`
- **GET `/api/me/positions`**
  - Response 200 OK:
    ```json
    [
      {
        "lat": 45.0703,
        "lon": 7.6869,
        "recorded_at": "2026-06-20T14:30:45Z"
      },
      {
        "lat": 45.0721,
        "lon": 7.6895,
        "recorded_at": "2026-06-20T14:31:20Z"
      }
    ]
    ```
  - Response 401 Unauthorized: `{"error": "L'utente non ha effettuato l'accesso"}`
  - Response 500 Internal Server Error: `{"error": "Impossibile recuperare la sessione"}` oppure `{"error": "Impossibile recuperare le posizioni"}`

### 9.2 MQTT

- **Publish `georuggine/client/:user_id/position`**
  - QoS: `AtMostOnce (0)`
  - Payload:
    ```json
    {
      "token": "JWT_TOKEN",
      "lat": 45.0703,
      "lon": 7.6869
    }
    ```
- **Publish `georuggine/client/:user_id/message`**
  - QoS: `AtLeastOnce (1)`
  - Payload:
    ```json
    {
      "token": "JWT_TOKEN",
      "content": "Messaggio inviato al server"
    }
    ```
- **Publish `georuggine/server/:user_id/direct`**
  - QoS: `AtLeastOnce (1)`
  - Payload:
    ```json
    {
      "type": "direct",
      "id": 15,
      "from": "server",
      "content": "Messaggio per il camionista",
      "timestamp": "2026-06-20T14:30:45Z"
    }
    ```
- **Publish `georuggine/server/broadcast`**
  - QoS: `AtLeastOnce (1)`
  - Payload:
    ```json
    {
      "type": "broadcast",
      "id": 16,
      "from": "server",
      "content": "Messaggio per tutti i camionisti",
      "timestamp": "2026-06-20T14:30:45Z"
    }
    ```
- **Publish `georuggine/server/:user_id/state`**
  - QoS: `AtLeastOnce (1)`
  - Payload:
    ```json
    {
      "user_id": 1,
      "state": "moving",
      "timestamp": "2026-06-20T14:30:45Z"
    }
    ```
- **Publish `georuggine/server/:user_id/error`**
  - QoS: `AtLeastOnce (1)`
  - Payload:
    ```json
    {
      "error": "Coordinate non valide"
    }
    ```
## 10. Demo

### 10.0 Elenco dei file
Per popolare il sistema con dati realistici e verificarne il funzionamento end-to-end (registrazione utenti, invio posizioni via MQTT, invio messaggi, generazione dei report) è disponibile un piccolo progetto Rust separato, organizzato come una serie di binari (`src/bin/*.rs`) più due moduli di libreria condivisi (`mqtt.rs`, `osrm.rs`). Questi script non fanno parte del server, ma agiscono da **client di simulazione**: creano utenti reali tramite le API REST del server, generano tragitti realistici su rete stradale e riproducono via MQTT il traffico che normalmente verrebbe generato da veicoli reali.

> **Importante:** tutti gli utenti creati dagli script di simulazione (compreso l'admin usato per autenticarsi) hanno la password `Password123!`. È necessario che questa password coincida con quella già presente nel database del server (o che l'utente admin venga creato con questa password), altrimenti le chiamate a `/api/login` effettuate dagli script falliscono.

| File | A cosa serve |
| --- | --- |
| `find_kebabs.rs` | Interroga Google Places e genera `kebab_torino_google.csv`, l'elenco delle destinazioni usate nella simulazione. |
| `create_users.rs` | Crea 20 utenti di test tramite `POST /api/register`, tutti con password `Password123!`. |
| `bake_simulation.rs` | Precalcola i tragitti di tutti gli utenti (via OSRM) e li salva in `positions.csv` e `messages.csv`. |
| `replay.rs` | Legge `positions.csv` e `messages.csv` e reinvia gli eventi al server via MQTT rispettando la timeline originale. |
| `osrm.rs` | Modulo condiviso che interroga OSRM per calcolare tragitti reali su strada e ne simula l'avanzamento nel tempo. |
| `mqtt.rs` | Modulo condiviso che gestisce la connessione TLS al broker MQTT e la pubblicazione di posizioni/messaggi. |
| `setup_osrm.sh` | Script Bash (Linux/macOS/WSL) che avvia via Docker l'istanza locale di OSRM necessaria a `bake_simulation.rs`. |
| `setup_osrm.ps1` | Equivalente PowerShell nativo dello script precedente, per Windows senza WSL. |

Questi quattro script sono pensati per essere eseguiti in sequenza:

```
find_kebabs   -->  create_users  -->  bake_simulation  -->  replay
(dataset)          (utenti)           (genera i CSV)        (invia via MQTT)
```

### 10.1 `find_kebabs.rs` — generazione del dataset di destinazioni

Script una tantum che interroga le **Google Places API** (endpoint `nearbysearch`) cercando locali con la parola chiave "kebab" in un raggio di 10 km dal centro di Torino (coordinate `45.0703, 7.6869`). Gestisce la paginazione dei risultati tramite `next_page_token` (con la pausa di 2 secondi richiesta da Google prima di poter riutilizzare il token) e scrive il risultato in `kebab_torino_google.csv`, con colonne `name`, `lat`, `lon`.

Questo file rappresenta l'insieme dei punti di interesse che gli utenti simulati raggiungeranno a turno durante la simulazione (funge quindi da elenco di "destinazioni plausibili" sparse sulla città, non da funzionalità del prodotto). Va eseguito una sola volta: il CSV prodotto viene poi riutilizzato da `bake_simulation`. Richiede una API key di Google Maps valida.

### 10.2 `create_users.rs` — creazione degli utenti di test

Effettua il login come amministratore (`admin@example.com` / `Password123!`) su `POST /api/login`, quindi chiama `POST /api/register` per creare 20 utenti di test con nomi e cognomi italiani predefiniti (es. `marco.rossi@example.com`). Tutti vengono creati con `is_admin: false` e password `Password123!`, la stessa richiesta da tutti gli altri script della demo.

Usa `danger_accept_invalid_certs(true)` sul client HTTPS perché in ambiente di sviluppo il server espone un certificato self-signed generato con `mkcert` (coerentemente con quanto descritto nel §2.5.1); questa opzione non deve mai essere usata verso un server pubblico con certificato valido.

### 10.3 `bake_simulation.rs` — generazione dei tragitti simulati

È lo script più corposo: **non invia nulla in tempo reale**, ma pre-calcola ("bake", da cui il nome) un'intera simulazione e la salva su disco in due file CSV, che verranno poi effettivamente inviati al server da `replay.rs`. Si esegue con:

```bash
cargo run --bin bake_simulation <MINUTI>
```

dove `<MINUTI>` è la durata (simulata, non reale) della simulazione, di default 60 minuti se omesso.

Funzionamento:

1. Effettua il login come admin e recupera tramite `GET /api/users` l'elenco di tutti gli utenti non amministratori presenti sul server (quelli creati da `create_users.rs`).
2. Legge `kebab_torino_google.csv` come elenco di destinazioni possibili.
3. Per ciascun utente, avvia un task asincrono indipendente (`tokio::spawn`) che simula un percorso: origine e destinazione iniziali vengono scelte casualmente tra i kebab del CSV, e il tragitto reale tra i due punti viene calcolato interrogando un'istanza locale di **OSRM** (`http://localhost:5000`, vedi §10.5) tramite il modulo `osrm.rs`.
4. Il movimento viene campionato ogni `TICK_SECONDS` (30 secondi simulati) e ogni posizione intermedia viene scritta come riga in `positions.csv`.
5. Quando un utente raggiunge la destinazione, lo script sceglie casualmente (1 possibilità su 15) tra tre comportamenti: registrare l'arrivo con un messaggio "destinazione raggiunta" in `messages.csv` e ripartire verso un nuovo kebab scelto a caso; oppure fermarsi per una pausa di 120 secondi (simulati) registrando un messaggio "pausa di 120 secondi"; oppure restare fermo nella posizione corrente per un altro tick. Questo produce un mix di soste e spostamenti più realistico di un semplice tragitto continuo.
6. La simulazione per ogni utente termina quando il tempo simulato trascorso raggiunge i minuti richiesti da riga di comando.

Entrambi i CSV (`positions.csv`, `messages.csv`) vengono azzerati (`init_csv_files`) all'avvio di ogni run, in modo da non mescolare dati di esecuzioni diverse: `replay.rs` raggruppa e ordina gli eventi solo per `user_id` e offset temporale, quindi righe residue di un run precedente causerebbero "teletrasporti" dell'utente da un capo all'altro della città. Per lo stesso motivo, tutte le scritture sui due file passano da un unico lock globale (`csv_write_lock`), necessario perché più utenti vengono simulati in parallelo e la scrittura dell'header CSV non è altrimenti atomica.

Ogni riga dei due CSV include, oltre ai dati di posizione/messaggio, anche `user_id` **ed `email`**, così da permettere a `replay.rs` di autenticarsi direttamente senza bisogno di consultare di nuovo il server, e i due campi `elapsed_from_start_ms` / `elapsed_from_last_ms`, usati rispettivamente per ricostruire la timeline assoluta e per calcolare gli intervalli tra un evento e il successivo.

### 10.4 `replay.rs` — invio della simulazione via MQTT

Legge `positions.csv` e `messages.csv` (di default nella cartella corrente, oppure percorsi passati da riga di comando) e reinvia tutti gli eventi al server rispettando, per ciascun utente, la stessa sequenza temporale con cui sono stati generati da `bake_simulation`:

```bash
cargo run --bin replay                              # velocità normale (1x, consigliata)
cargo run --bin replay -- 15                         # 15x più veloce, solo per debug
cargo run --bin replay -- 15 positions.csv messages.csv
```

Il primo argomento opzionale è uno `speed_factor`: gli offset temporali letti dal CSV vengono divisi per questo valore, permettendo di comprimere una simulazione di ore in pochi minuti reali.

> **Importante:** lo `speed_factor` serve solo per il debug, ad esempio per verificare rapidamente che un'intera simulazione venga riprodotta correttamente senza dover attendere il tempo reale corrispondente. Una simulazione pensata per essere effettivamente utilizzata (report, demo, verifica del comportamento del server con un carico realistico) va invece eseguita a velocità normale (`speed_factor = 1`, cioè senza passare l'argomento). Velocità più alte comprimono gli intervalli tra gli eventi al di sotto di quanto previsto dal comportamento reale di un utente (ad es. il rate limit di 1 messaggio/secondo lato server, vedi §10.6), quindi possono produrre messaggi scartati o non rispettare il vincolo di una position log ogni 30 secondi.

Per ogni utente presente nei CSV, lo script:

1. Raggruppa posizioni e messaggi in un'unica lista di eventi ordinata per `elapsed_from_start_ms` (i due tipi di file vengono quindi fusi e non più trattati separatamente).
2. Effettua il login (`POST /api/login`) usando l'email presente nella riga CSV e la password `Password123!`, ottenendo un JWT fresco (i token non vengono quindi salvati nei CSV, solo l'email).
3. Apre una connessione MQTT dedicata verso `broker.emqx.io:8883` tramite `initialize_mqtt_client` (modulo `mqtt.rs`, §10.6).
4. Attende il tempo necessario a rispettare l'offset del prossimo evento rispetto a un cronometro locale (`Instant`), quindi pubblica l'evento (`send_position` o `send_message`) con il token appena ottenuto. Se l'invio accumula più di una soglia di ritardo, lo stampa a log come avviso ("Behind schedule").

Ogni utente viene gestito da un task `tokio::spawn` indipendente, quindi tutti gli utenti vengono "riprodotti" in parallelo, esattamente come erano stati generati.

### 10.5 Dipendenza da OSRM

`osrm.rs` (usato solo da `bake_simulation`) non è un binario a sé ma un modulo condiviso che genera tragitti realistici su strada invece di semplici linee rette tra due coordinate. Richiede un'istanza locale del progetto **OSRM** (Open Source Routing Machine) raggiungibile su `http://localhost:5000`, con il profilo di routing per auto e i dati OSM dell'area di Torino già caricati. Per ogni coppia origine/destinazione, `get_route` interroga l'endpoint `/route/v1/driving/...` con `annotations=speed`, ottenendo sia la geometria del percorso sia la velocità stimata segmento per segmento (in mancanza di un'annotazione valida viene usata una velocità di fallback di 35 km/h). `VehicleSimulator` mantiene poi lo stato di avanzamento lungo questi segmenti e restituisce una posizione interpolata ogni volta che `next_position(dt_sec)` viene chiamato, finché il tragitto non è esaurito.

Questa istanza locale non viene avviata dagli script Rust: va predisposta a parte tramite Docker, come descritto nel paragrafo seguente.

### 10.5.1 `setup_osrm.sh` / `setup_osrm.ps1` — avvio dell'istanza OSRM locale

Sono due script di infrastruttura (uno per Linux/macOS/WSL in Bash, uno equivalente per Windows in PowerShell nativo) che preparano ed avviano, tramite **Docker**, l'istanza OSRM richiesta da `bake_simulation.rs`. Non fanno parte della pipeline Rust e vanno eseguiti manualmente **una sola volta**, prima di lanciare `bake_simulation`, dalla cartella in cui si vuole conservare l'estratto della mappa:

```bash
# Linux / macOS / WSL / Git Bash
./setup_osrm.sh
```

```powershell
# Windows, PowerShell nativo (non richiede WSL)
.\setup_osrm.ps1
```

Eseguono la stessa sequenza di passi, usando l'immagine ufficiale `ghcr.io/project-osrm/osrm-backend`:

1. **Verifica Docker** — controllano che il Docker daemon sia in esecuzione (`docker info`), interrompendosi con un errore chiaro in caso contrario.
2. **Download dell'estratto OSM** — scaricano da BBBike (`download.bbbike.org`) l'estratto `Turin.osm.pbf`, cioè la sola rete stradale del comune di Torino (circa 13 MB), evitando così di scaricare l'estratto regionale Geofabrik "nord-ovest" molto più pesante (400+ MB). Se il file è già presente non viene riscaricato; viene inoltre verificato che la dimensione superi una soglia minima (5 MB), per accorgersi se al posto del `.pbf` è stata scaricata per errore una pagina di errore HTML.
3. **`osrm-extract`** — costruisce il grafo della rete stradale a partire dal `.pbf`, usando il profilo `car.lua` (routing per auto).
4. **`osrm-partition`** — partiziona il grafo secondo l'algoritmo **MLD** (Multi-Level Dijkstra), l'algoritmo di routing raccomandato di default da OSRM.
5. **`osrm-customize`** — completa la preparazione dei dati per MLD.
6. **Avvio del server di routing** — lanciano un container Docker persistente (`--restart unless-stopped`, nome `osrm`) che espone `osrm-routed --algorithm mld` sulla porta `5000`, la stessa interrogata da `osrm.rs` (`http://localhost:5000/route/v1/driving/...`).

Al termine, entrambi gli script stampano un comando di verifica rapida (una chiamata di test all'endpoint `/route/v1/driving`) e i comandi Docker utili per la gestione successiva del container (`docker logs osrm`, `docker stop osrm`, `docker start osrm`); una volta processati i dati con `extract`/`partition`/`customize`, riavviare il container con `docker start osrm` non richiede di rieseguire l'intera pipeline.

> **Nota:** trattandosi di un estratto limitato al solo comune di Torino, qualsiasi tragitto richiesto a OSRM che esca dal relativo bounding box (ad es. verso comuni limitrofi) fallisce con un errore `NoRoute`. Per coprire un'area più ampia è sufficiente sostituire l'URL dell'estratto (`PBF_URL` / `$PbfUrl`) con quello di un estratto regionale Geofabrik (es. "nord-ovest"), tenendo presente che le fasi di `extract`/`partition`/`customize` richiederanno più tempo e spazio su disco.

### 10.6 Modulo `mqtt.rs`

Modulo condiviso da `replay.rs` (e riutilizzabile da eventuali altri publisher esterni) che incapsula la connessione MQTT via TLS al broker pubblico `broker.emqx.io` sulla porta `8883`, usando il certificato CA del broker incluso a compile-time nel binario. Espone tre funzioni:

- `initialize_mqtt_client`: crea l'`AsyncClient` e avvia in background un task che effettua il polling continuo dell'eventloop (richiede quindi di essere chiamata da un contesto già dentro un runtime Tokio).
- `send_position`: pubblica su `georuggine/client/{user_id}/position` con `QoS::AtMostOnce`, coerentemente con quanto documentato in §9.2 per questo topic.
- `send_message`: pubblica su `georuggine/client/{user_id}/message` con `QoS::AtLeastOnce`.

In entrambi i casi il payload include il JWT dell'utente (claim `sub` corrispondente a `user_id`): il server scarta silenziosamente i messaggi in cui i due valori non coincidono, per impedire che un utente autenticato invii dati a nome di un altro (comportamento descritto anche in §2.5).

### 10.7 Riepilogo: esecuzione completa della demo

```bash
# 0. una tantum: avvia via Docker l'istanza locale di OSRM su localhost:5000
./setup_osrm.sh          # oppure, su Windows: .\setup_osrm.ps1

# 1. una tantum: genera l'elenco delle destinazioni (kebab di Torino)
MAPS_API_KEY="your_actual_api_key" cargo run --bin find_kebabs

# 2. crea 20 utenti di test (password Password123! per tutti)
cargo run --bin create_users

# 3. genera i tragitti simulati per 60 minuti (richiede OSRM avviato al passo 0)
cargo run --bin bake_simulation 60

# 4. riproduce la simulazione via MQTT a velocità normale (1x)
cargo run --bin replay
```

Al termine, il database del server risulterà popolato con posizioni, sessioni di movimento e messaggi realistici per tutti gli utenti di test, utilizzabili per verificare manualmente report, mappe e messaggistica lato admin.