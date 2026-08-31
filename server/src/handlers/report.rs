use axum::{
    extract::{Extension, Query, Path, State},
    http::StatusCode,
    middleware,
    response::{IntoResponse, Response},
    routing::get,
    Json,
    Router,
};
use chrono::{DateTime, Datelike, Days, NaiveDate, Utc};
use serde::Deserialize;
use crate::{
    auth::{self, Claims},
    dao::{movement_sessions_dao, position_log_dao},
    errors::error_response,
    models::{MovementSession, MovementState, Position, ReportPeriod, RouteReport},
    state::AppState,
};

#[derive(Deserialize)]
pub struct ReportQuery {
    pub user_id: i64,
    pub period: ReportPeriod,
}

/// GET /api/report?user_id=&period=  (solo admin)
pub async fn get_report_handler(State(state): State<AppState>, Query(params): Query<ReportQuery>) -> Response {
    build_report(&state, params.user_id, params.period).await
}

/// Costruisce il report di un utente per il periodo richiesto.
///
/// Punto chiave: le posizioni vengono raggruppate per sessione di movimento
/// (`MovementSession`) invece che restituite come un unico elenco piatto.
/// Questo evita che il frontend disegni una linea che collega due sessioni
/// scollegate tra loro (es. una tratta a Torino e una a Milano nello stesso
/// giorno), e allo stesso tempo evita che il calcolo della velocità media
/// sommi la distanza "fantasma" tra la fine di una sessione e l'inizio della
/// successiva.
async fn build_report(state: &AppState, user_id: i64, period: ReportPeriod) -> Response {
    let (start, end) = get_start_end_from_report_period(period);

    let mut sessions = match movement_sessions_dao::get_sessions_in_range(&state.db, user_id, start, end) {
        Ok(s) => s,
        Err(e) => {
            tracing::error!("errore get_sessions_in_range: {e}");
            return error_response(
                StatusCode::INTERNAL_SERVER_ERROR,
                "Impossibile recuperare le sessioni di movimento",
            );
        }
    };
    // Non ci fidiamo dell'ordine restituito dalla query: se per lo stesso
    // utente esistono più sessioni nello stesso periodo (es. simulazioni
    // ripetute senza attendere la disconnessione), il frontend individua la
    // tappa di "partenza" prendendo il primo elemento di `segments`. Senza
    // un ordinamento esplicito qui, quel primo elemento potrebbe non essere
    // il primo in ordine cronologico, facendo apparire un punto di partenza
    // diverso (e sbagliato) a seconda del periodo (giorno/settimana/mese).
    sessions.sort_by_key(|s| s.started_at);

    let mut positions = match position_log_dao::get_positions_in_range(&state.db, user_id, start, end) {
        Ok(p) => p,
        Err(e) => {
            tracing::error!("errore get_positions_in_range: {e}");
            return error_response(
                StatusCode::INTERNAL_SERVER_ERROR,
                "Impossibile recuperare le posizioni",
            );
        }
    };
    // Stesso discorso per le posizioni: `compute_avg_speed_kmh` assume che
    // i punti dentro ogni segmento siano in ordine cronologico (usa
    // `windows(2)` per calcolare le distanze tra punti consecutivi).
    positions.sort_by_key(|p| p.recorded_at);

    let (movement_duration_secs, pause_duration_secs) = compute_durations(&sessions, start, end);
    let segments = build_segments(&sessions, &positions);
    let avg_speed_kmh = compute_avg_speed_kmh(&segments, movement_duration_secs);

    Json(RouteReport {
        user_id,
        period,
        segments,
        avg_speed_kmh,
        movement_duration_secs,
        pause_duration_secs,
    })
    .into_response()
}

fn build_segments(sessions: &[MovementSession], positions: &[Position]) -> Vec<Vec<Position>> {
    let now = Utc::now();

    sessions
        .iter()
        .filter(|s| s.state == MovementState::Moving)
        .map(|session| {
            let session_end = session.ended_at.unwrap_or(now);
            
            let mut segment: Vec<Position> = positions
                .iter()
                .filter(|p| p.recorded_at >= session.started_at && p.recorded_at < session_end)
                .copied()
                .collect();
            
            if let Some(prev_pos) = positions
                .iter()
                .filter(|p| p.recorded_at < session.started_at)
                .max_by_key(|p| p.recorded_at)
            {
                segment.insert(0, *prev_pos);
            }
            
            segment
        })
        .filter(|segment| !segment.is_empty())
        .collect()
}

/// GET /api/me/positions (utente autenticato, solo le sue posizioni per la mappa)
///
/// Ritorna le posizioni della **sessione corrente aperta** in `movement_sessions`.
/// Se l'utente non ha una sessione aperta (disconnesso), la lista è vuota.
/// Altrimenti restituisce tutte le posizioni da `started_at - 60s` della sessione
/// aperta fino ad ora.
pub async fn get_own_positions_handler(
    State(state): State<AppState>,
    Extension(claims): Extension<Claims>,
) -> Response {
    let session = match movement_sessions_dao::get_open_session_for_user(&state.db, claims.sub) {
        Ok(Some(s)) => s,
        Ok(None) => return Json(Vec::<Position>::new()).into_response(),
        Err(e) => {
            tracing::error!("errore get_open_session_for_user: {e}");
            return error_response(StatusCode::INTERNAL_SERVER_ERROR, "Impossibile recuperare la sessione");
        }
    };

    let now = Utc::now();
    let start = session.started_at - chrono::Duration::seconds(60);

    let positions = match position_log_dao::get_positions_in_range(&state.db, claims.sub, start, now) {
        Ok(p) => p,
        Err(e) => {
            tracing::error!("errore get_positions_in_range: {e}");
            return error_response(StatusCode::INTERNAL_SERVER_ERROR, "Impossibile recuperare le posizioni");
        }
    };

    Json(positions).into_response()
}

pub async fn get_user_positions_handler(
    State(state): State<AppState>,
    Path(user_id): Path<i64>,
) -> Response {
    let session = match movement_sessions_dao::get_open_session_for_user(&state.db, user_id) {
        Ok(Some(s)) => s,
        Ok(None) => return Json(Vec::<Position>::new()).into_response(),
        Err(e) => {
            tracing::error!("errore get_open_session_for_user: {e}");
            return error_response(StatusCode::INTERNAL_SERVER_ERROR, "Impossibile recuperare la sessione");
        }
    };

    let now = Utc::now();
    let start = session.started_at - chrono::Duration::seconds(60);

    let positions = match position_log_dao::get_positions_in_range(&state.db, user_id, start, now) {
        Ok(p) => p,
        Err(e) => {
            tracing::error!("errore get_positions_in_range: {e}");
            return error_response(StatusCode::INTERNAL_SERVER_ERROR, "Impossibile recuperare le posizioni");
        }
    };

    Json(positions).into_response()
}

pub fn get_start_end_from_report_period(report_period: ReportPeriod) -> (DateTime<Utc>, DateTime<Utc>) {
    let now = Utc::now();
    match report_period {
        ReportPeriod::Day => {
            let start_of_the_day = now
                .date_naive()
                .and_hms_opt(0, 0, 0)
                .expect("00:00:00 is always a valid time")
                .and_utc();
            (start_of_the_day, now)
        }
        ReportPeriod::Week => {
            let days_from_monday = now.date_naive().weekday().num_days_from_monday();

            let start_of_the_week = now
                .date_naive()
                .checked_sub_days(Days::new(days_from_monday as u64))
                .expect("Date subtraction will not overflow for realistic dates")
                .and_hms_opt(0, 0, 0)
                .expect("00:00:00 is always a valid time")
                .and_utc();
            (start_of_the_week, now)
        }
        ReportPeriod::Month => {
            let start_of_the_month = NaiveDate::from_ymd_opt(now.year(), now.month(), 1)
                .expect("Day 1 is always valid for any valid year/month combination")
                .and_hms_opt(0, 0, 0)
                .expect("00:00:00 is always a valid time")
                .and_utc();
            (start_of_the_month, now)
        }
    }
}

pub fn haversine_distance_km(p1: &Position, p2: &Position) -> f64 {
    const EARTH_RADIUS_KM: f64 = 6371.0;

    let lat1 = p1.lat.to_radians();
    let lat2 = p2.lat.to_radians();
    let delta_lat = (p2.lat - p1.lat).to_radians();
    let delta_lon = (p2.lon - p1.lon).to_radians();

    let a = (delta_lat / 2.0).sin().powi(2)
        + lat1.cos() * lat2.cos() * (delta_lon / 2.0).sin().powi(2);

    let c = 2.0 * a.sqrt().atan2((1.0 - a).sqrt());

    EARTH_RADIUS_KM * c
}

/// Calcola la velocità media sommando SOLO le distanze tra punti consecutivi
/// che appartengono alla STESSA sessione di movimento (`segments` è già
/// raggruppato per sessione da `build_segments`). Non viene mai calcolata
/// una distanza tra l'ultimo punto di un segmento e il primo del successivo,
/// quindi una "sessione fantasma" (es. tratte in città diverse) non può più
/// gonfiare la velocità media.
pub fn compute_avg_speed_kmh(segments: &[Vec<Position>], movement_duration_secs: i64) -> f64 {
    if movement_duration_secs <= 0 {
        return 0.0;
    }

    const MAX_GAP_SECS: i64 = 90;
    const MIN_SEGMENT_DISTANCE_KM: f64 = 0.005; // 5 metri: ignora rumore GPS

    let total_distance_km: f64 = segments
        .iter()
        .flat_map(|segment| segment.windows(2))
        .filter_map(|pair| {
            let gap = (pair[1].recorded_at - pair[0].recorded_at).num_seconds();
            if !(gap > 0 && gap <= MAX_GAP_SECS) {
                return None;
            }
            let distance = haversine_distance_km(&pair[0], &pair[1]);
            (distance >= MIN_SEGMENT_DISTANCE_KM).then_some(distance) // ignora rumore GPS < 5m
        })
        .sum();

    let total_hours = movement_duration_secs as f64 / 3600.0;
    total_distance_km / total_hours
}

pub fn compute_durations(
    sessions: &[MovementSession],
    start: DateTime<Utc>,
    end: DateTime<Utc>,
) -> (i64, i64) {
    sessions.iter().fold((0, 0), |(mut move_acc, mut pause_acc), session| {
        let session_start = session.started_at.max(start);
        let session_end = session.ended_at.unwrap_or(end).min(end);
        let duration = (session_end - session_start).num_seconds().max(0);

        match session.state {
            MovementState::Moving => move_acc += duration,
            MovementState::Stopped => pause_acc += duration,
        }
        (move_acc, pause_acc)
    })
}

pub fn router() -> Router<AppState> {
    Router::new()
        .merge(admin_router())
        .merge(self_router())
}

fn admin_router() -> Router<AppState> {
    Router::new()
        .route("/api/admin/users/:user_id/positions", get(get_user_positions_handler))
        .route("/api/report", get(get_report_handler))
        .layer(middleware::from_fn(auth::jwt_admin_middleware))
}

fn self_router() -> Router<AppState> {
    Router::new()
        .route("/api/me/positions", get(get_own_positions_handler))
        .layer(middleware::from_fn(auth::jwt_auth_middleware))
}