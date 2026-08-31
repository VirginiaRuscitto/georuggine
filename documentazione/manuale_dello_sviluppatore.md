# Manuale dello sviluppatore

## 1.Introduzione

### 1.1 Panoramica del progetto

Il progetto è organizzato in due parti: client e server. Il client rappresenta l'applicazione utilizzata dagli utenti e dagli amministratori, mentre il server si occupa di coordinare la comunicazione tra i client, gestire gli utenti, le posizioni, gli stati, i messaggi e la memorizzazione dei dati. Gli amministratori, pur utilizzando client distinti, operano tutti in veste di server e rappresentano quindi un'unica entità nei confronti degli utenti. La comunicazione HTTPS del client web è inoltre gestita tramite CORS, che permette al client di effettuare richieste al server.

La comunicazione tra client e server utilizza protocolli diversi in base al tipo di operazione e di client. Le operazioni come l'autenticazione, la registrazione e la consultazione dello storico dei messaggi utilizzano HTTPS REST per entrambi i client. L'amministratore utilizza HTTPS per attività come la gestione degli utenti, la generazione dei report e l'invio di messaggi diretti o broadcast. Trattandosi di un client utilizzato da una postazione stabile, il modello richiesta-risposta di HTTPS si adatta bene alle interazioni con il server. Il client degli utenti utilizza invece MQTT, dovendo inviare periodicamente al server la propria posizione. La scelta è legata alla natura IoT del client, che può trovarsi in presenza di una connessione meno stabile. MQTT permette di gestire questo tipo di comunicazione senza dover effettuare una nuova richiesta HTTPS per ogni posizione e offrendo inoltre meccanismi di gestione e ritrasmissione dei messaggi. L'utilizzo di MQTT viene esteso anche alle altre comunicazioni del client utente, quali l'invio e la ricezione dei messaggi e la gestione delle notifiche relative ai cambiamenti di stato e agli errori. Questa scelta consente di mantenere un unico meccanismo di comunicazione, evitando di introdurre ulteriori protocolli e sfruttando un approccio coerente con la natura IoT del client.

TODO compatibilità

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

TODO stack del frontend e della demo

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

Per avviare l'applicazione in ambiente di sviluppo è necessario avviare separatamente il backend e il frontend.
- **Backend**
  ```bash
  cd server
  cargo run
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

Il modulo `messages.rs` espone le route dedicate alla messaggistica. `get_messages_handler` distingue innanzitutto il tipo di richiesta in base al ruolo dell'utente e al parametro `with`. Per un amministratore, `with` identifica l'utente con cui visualizzare la conversazione diretta; se non viene specificato, vengono invece recuperati i soli messaggi broadcast. Per un utente normale non è necessario specificare `with`, perché vengono recuperati automaticamente i messaggi diretti che lo riguardano insieme ai broadcast. Il parametro `limit` stabilisce il numero massimo di messaggi restituiti: se non viene specificato viene utilizzato il valore predefinito di 50. `clamp(1, MAX_LIMIT)` limita comunque il valore tra 1 e 200. La differenza nella gestione delle conversazioni rispecchia le esigenze delle due interfacce: . TODO chiedere a enzo il funzionamento per completare

L'invio tramite HTTPS è invece riservato agli amministratori: `post_direct_message` verifica l'esistenza del destinatario, salva il messaggio nel database e ne notifica la ricezione tramite MQTT, mentre `post_broadcast_handler` salva e pubblica un messaggio destinato a tutti gli utenti. Entrambe le funzioni utilizzano `validate_content` per verificare che il messaggio non sia vuoto e non superi i 1000 caratteri. I messaggi vengono quindi prima persistiti nel database e solo successivamente notificati tramite MQTT, mantenendo lo storico disponibile anche nel caso in cui la pubblicazione MQTT non vada a buon fine.

### 7.2 Ricezione e invio dei messaggi tramite MQTT

I messaggi inviati dal server vengono ricevuti dagli utenti tramite topic MQTT dedicati. I messaggi diretti vengono pubblicati sul topic associato al singolo utente, mentre i broadcast utilizzano un topic comune a tutti gli utenti.

Quando un utente invia un messaggio tramite MQTT, questo viene invece ricevuto dal server sul canale dedicato all'utente e passato a `handle_user_message` per la gestione e il salvataggio. Prima di procedere, viene verificato che l'utente non abbia già inviato un altro messaggio nell'ultimo secondo, applicando un rate limit per evitare un invio eccessivo di messaggi, sia per limitare il carico sul server e sul database sia per ridurre il rischio di attacchi basati sull'invio massivo di richieste. Se il controllo viene superato, il messaggio viene salvato nel database associandolo all'utente come mittente. In questo modo i messaggi ricevuti tramite MQTT vengono persistiti nello stesso storico utilizzato dai messaggi inviati tramite HTTPS. Si sottolinea che anche la ricezione di un messaggio costituisce un segnale di vita dell'utente e aggiorna quindi `last_seen_at`; in questo modo si evita di considerare l'utente come disconnesso finché continuano ad arrivare messaggi.

## 8. Frontend

### 8.1

Se ti interessa io nel corso di applicazioni web avevo fatto "componenti principali" con una breve spiegazione e "pagine"

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

### 10.1 Funzionamento complessivo

Il core della simulazione avviene dentro `bake_simulation.rs` che interroga i dati generati da `find_kebabs.rs` (locali con la parola chiave "kebab" in un raggio di 10 km dal centro di Torino), e fa muovere gli utenti da un punto A ad un punto B, scelti casualmente;
Lo spostamento viene simulato grazie ad `osrm.rs` che gestisce il calcolo dei tragitti reali e calcola delle posizioni realistiche del veicolo lungo il tragitto, tenendo conto anche della velocità media lungo il percorso.
I dati generati da `bake_simulation.rs` vengono poi usati da `replay.rs` per inviare al server i dati relativi agli utenti secondo la timeline precalcolata. 

### 10.2 `find_kebabs.rs` — generazione del dataset di destinazioni

Script una tantum che interroga le **Google Places API** (endpoint `nearbysearch`) cercando locali con la parola chiave "kebab" in un raggio di 10 km dal centro di Torino (coordinate `45.0703, 7.6869`). Gestisce la paginazione dei risultati tramite `next_page_token` (con la pausa di 2 secondi richiesta da Google prima di poter riutilizzare il token) e scrive il risultato in `kebab_torino_google.csv`, con colonne `name`, `lat`, `lon`.



### 10.3 `replay.rs` — invio della simulazione via MQTT

Legge `positions.csv` e `messages.csv` (di default nella cartella corrente, oppure percorsi passati da riga di comando) e reinvia tutti gli eventi al server rispettando, per ciascun utente, la stessa sequenza temporale con cui sono stati generati da `bake_simulation`:

```bash
cargo run --bin replay                              # velocità normale (1x, consigliata)
cargo run --bin replay -- 15                         # 15x più veloce, solo per debug
cargo run --bin replay -- 15 positions.csv messages.csv
```

Il primo argomento opzionale è uno `speed_factor`: gli offset temporali letti dal CSV vengono divisi per questo valore, permettendo di comprimere una simulazione di ore in pochi minuti reali.

> **Importante:** lo `speed_factor` serve solo per il debug, ad esempio per verificare rapidamente che un'intera simulazione venga riprodotta correttamente senza dover attendere il tempo reale corrispondente. Una simulazione pensata per essere effettivamente utilizzata (report, demo, verifica del comportamento del server con un carico realistico) va invece eseguita a velocità normale (`speed_factor = 1`, cioè senza passare l'argomento). Velocità più alte comprimono gli intervalli tra gli eventi al di sotto di quanto previsto dal comportamento reale di un utente (ad es. il rate limit di 1 messaggio/secondo lato server, vedi §10.6), quindi possono produrre messaggi scartati o non rispettare il vincolo di una position log ogni 30 secondi.

### 10.4 Dipendenza da OSRM

`osrm.rs` (usato solo da `bake_simulation`) non è un binario a sé ma un modulo condiviso che genera tragitti realistici su strada invece di semplici linee rette tra due coordinate. Richiede un'istanza locale del progetto **OSRM** (Open Source Routing Machine) raggiungibile su `http://localhost:5000`.

Questa istanza locale non viene avviata dagli script Rust: va predisposta a parte tramite Docker, come descritto nel paragrafo seguente.

### 10.4.1 `setup_osrm.sh` / `setup_osrm.ps1` — avvio dell'istanza OSRM locale

Sono due script di infrastruttura (uno per Linux/macOS/WSL in Bash, uno equivalente per Windows in PowerShell nativo) che preparano ed avviano, tramite **Docker**, l'istanza OSRM richiesta da `bake_simulation.rs`. Non fanno parte della pipeline Rust e vanno eseguiti manualmente **una sola volta**, prima di lanciare `bake_simulation`, dalla cartella in cui si vuole conservare l'estratto della mappa:

```bash
# Linux / macOS / WSL / Git Bash
./setup_osrm.sh
```

```powershell
# Windows, PowerShell nativo (non richiede WSL)
.\setup_osrm.ps1
```
