# Manuale Utente

## 1.Cosa è Georuggine

Georuggine è un'applicazione finalizzata alla gestione della geolocalizzazione e della comunicazione con una flotta di veicoli. Nello specifico, consente il tracciamento in tempo reale dei veicoli e la comunicazione (sia unicast che broadcast) tra amministratori e autisti.

### 1.1 Ruoli utenti

- Amministratore: gli amministratori sono un gruppo di utenti che beneficiano di una vista condivisa, agendo di fatto in veste di server che gestisce e supervisiona gli altri utenti. Possono creare e gestire gli account utente, nominare altri amministratori o rimuovere un utente da questo incarico, scrivere agli altri utenti, scrivere messaggi broadcast, tracciare gli altri utenti in tempo reale e generare report statistici sui loro movimenti.
- Utenti: sono i fruitori principali di questa applicazione, ognuno gode di un profilo personale da cui può comunicare con il profilo amministratore, ricevere messaggi broadcast e visualizzare mappa e stato in tempo reale.

## 2.Requisiti

- Il progetto è compatibile con Windows 10/11 e con le principali distribuzioni Linux (es. Ubuntu 22.04 o successive).
- Non è richiesta ma è fortemente consigliata la configurazione di un broker MQTT locale.

## 3.Avvio app (solo tester e sviluppatori)

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
  cd fleet_simulation
  cargo run --bin replay
  ```
## 4.Primo accesso: registrazione e login

- Registrazione:
  - Vai sulla scheda “Registrati”
  - Inserisci una mail e una password
  - Clicca su "Crea account"
  - Conferma: riceverai una notifica di avvenuta registrazione e potrai effettuare il login
- Login:
  - Vai sulla scheda “Accedi”
  - Inserisci mail e password
  - Clicca su "Login"
  - Accedi: vedrai la tua dashboard se sei un utente normale o la visuale da server se sei un amministratore

## 5.Interfaccia utente

### 5.1 Aree principali (Home)

- Header: schermate disponibili (Home/Messaggi, di default su Home), pulsante Logout
- Sidebar (sinistra): nome e mail, stato connessione (Fermo/In movimento/Offline)
- Mappa: mappa della città che mostra lo spostamento dell'utente in tempo reale

### 5.2 Aree principali (Messaggi)

- Header: schermate disponibili (Home/Messaggi), pulsante Logout
- Sidebar (sinistra): chat disponibili (Broadcast e Admin)

## 6.Interfaccia amministratore

### 6.1 Aree principali (Dashboard)

- Header: schermate disponibili (Dashboard/Utenti/Messaggi/Report, di default su Dashboard), pulsante Logout
- Header principale: Numero di utenti tracciati al momento, frequenza di aggiornamento delle posizioni, numero di utenti presenti nel database, numero di utenti in movimento, numero di utenti attivi, shortcut per la sezione report
- Mappa: mappa della città che mostra lo spostamento degli utenti tracciati in tempo reale, zoom

### 6.2 Aree principali (Utenti)

- Header: schermate disponibili (Dashboard/Utenti/Messaggi/Report), pulsante Logout
- Sidebar (sinistra): Registrazione nuovo utente
- Users list: lista utenti e barra di ricerca degli utenti 

### 6.3 Aree principali (Messaggi)

- Header: schermate disponibili (Dashboard/Utenti/Messaggi/Report), pulsante Logout
- Sidebar (sinistra): chat disponibili (Broadcast e utenti)
- Chat

### 6.4 Aree principali (Report)

- Header: schermate disponibili (Dashboard/Utenti/Messaggi/Report), pulsante Logout
- Richiedi report: generazione del report a partire da nome e granularità delle tempistiche
- Mappa: mappa con i tragitti e le statistiche degli utenti tracciati nei report richiesti 

## 7.Messaggi

### 7.1 Amministratore

Per scrivere un messaggio occorre selezionare la chat dalla sidebar laterale, a questo punto si può accedere alla chat, guardare lo storico dei messaggi, scrivere e inviare messaggi unicast. Inoltre è presente la chat Broadcast, attraverso la quale si può inviare un messaggio che arrivi a tutti gli utenti.

### 7.2 Utente

Nella sidebar laterale saranno presenti due chat: una chat unicast con l'amministratore, con cui l'utente potrà comunicare in caso di problemi, e una chat broadcast in cui può scrivere solo l'amministratore e di cui tutti gli utenti conservano lo storico dei messaggi.

## 8.Gestione utenti (solo amministratori)

- Registrazione utenti:
  - Inserisci i dati richiesti (nome, cognome, mail, password)
  - Spunta sulla casella "è admin" se si desidera che l'utente creato abbia i compiti da amministratore
  - Clicca su "Crea utente" 
- Ricerca utenti:
  - Scrivi nome, cognome o mail sulla barra di ricerca
  - In alternativa o in aggiunta clicca sulla sezione dei filtri e clicca sulle spunte di stato o di movimento che ti interessa vedere
- Promuovi utente ad amministratore (o rimuovi amministratore):
  - Clicca sullo switch a destra del nome utente
  - Conferma: a destra del nome comparirà il ruolo corrente della persona (Utente/Admin). Un amministratore sarà inoltre indicato con il colore arancione
- Cancella utente: 
  - Clicca sull'icona rossa con un cestino sopra
  - Conferma: clicca "Ok" sul banner di conferma che comparirà

## 9.Report (solo amministratori)

- Genera un report: 
  - Inserisci nome, cognome o mail per cercare l'utente che ti interessa esaminare
  - Seleziona l'utente
  - Seleziona la granularità del report (Giorno/Settimana/Mese)
  - Clicca su "Genera report"
  - Visualizzazione: sopra la mappa compariranno le statistiche esaminate (velocità media, tempo in movimento, tempo in pausa) mentre nella mappa verrà visualizzato il tragitto compiuto nell'arco di tempo indicato, con tanto di punto di partenza e punto di arrivo

