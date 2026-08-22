# Manuale Utente

## 1.Cosa è Georuggine

Georuggine è un'applicazione finalizzata alla gestione della geolocalizzazione e della comunicazione con una flotta di veicoli. Nello specifico, consente il tracciamento in tempo reale dei veicoli e la comunicazione (sia unicast che broadcast) tra amministratori e autisti.

## 2.Requisiti

- sistemi operativi su cui deployamo TODO
- altre cose TODO

## 3.Avvio app

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

TODO valutare se poi ci saranno variazioni a questo, io sto seguendo lo scheletro di quello visto online

## 4.Primo accesso: registrazione e login

TODO valutare se aggiungere immagini per renderlo più chiaro

## 5.Messaggi

### 5.1 Messaggi unicast

Per scrivere un messaggio da parte dell'amministratore verso un determinato utente o verso l'amministratore

### 5.2 Messaggi broadcast (solo amministratori)

Per scrivere un messaggio a tutti gli utenti

## 6.Analisi veicoli (solo amministratori)

## 7.Risoluzione problemi (FAQ)

l'ho copiato dall'altro, potrebbe essere una buona idea metterlo
TODO pensare a eventuali problemi o difficoltà che l'utente può riscontrare e a come risolverli

