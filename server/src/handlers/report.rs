use axum::{
    extract::{Query, State},
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
    auth,
    dao::{movement_sessions_dao, position_log_dao, users_dao},
    errors::error_response,
    models::{MovementSession, MovementState, Position, ReportPeriod, RouteReport},
    state::AppState,
};

#[derive(Deserialize)]
pub struct ReportQuery { 
    pub user_id: i64, 
    pub period: ReportPeriod 
}

pub async fn get_report_handler(State(state): State<AppState>, Query(params): Query<ReportQuery>) -> Response {
    let (start, end) = get_start_end_from_report_period(params.period);

    let user_exists = match users_dao::get_user_by_id(&state.db, params.user_id) {
        Ok(Some(_)) => true,
        Ok(None) => return error_response(StatusCode::NOT_FOUND, "Utente non trovato"),
        Err(e) => {
            tracing::error!("errore get_user_by_id: {e}");
            return error_response(StatusCode::INTERNAL_SERVER_ERROR, "Errore del server");
        }
    };

    let positions = match position_log_dao::get_positions_in_range(&state.db, params.user_id, start, end) {
        Ok(p) => p,
        Err(e) => {
            tracing::error!("errore get_positions_in_range: {e}");
            return error_response(StatusCode::INTERNAL_SERVER_ERROR, "Impossibile calcolare il tragitto");
        }
    };

    let sessions = match movement_sessions_dao::get_sessions_in_range(&state.db, params.user_id, start, end) {
        Ok(s) => s,
        Err(e) => {
            tracing::error!("errore get_sessions_in_range: {e}");
            return error_response(StatusCode::INTERNAL_SERVER_ERROR, "Impossibile calcolare le durate del movimento e delle pause");
        }
    };

    let avg_speed_kmh = compute_avg_speed_kmh(&positions);
    let (movement_duration_secs, pause_duration_secs) = compute_durations(&sessions, start, end);

    let report = RouteReport {
        user_id: params.user_id,
        period: params.period,
        trajectory: positions,
        avg_speed_kmh,
        movement_duration_secs,
        pause_duration_secs,
    };

    Json(report).into_response()
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

pub fn compute_avg_speed_kmh(positions: &[Position]) -> f64 {
    if positions.len() < 2 {
        return 0.0;
    }

    let total_distance_km: f64 = positions
        .windows(2)
        .map(|pair| haversine_distance_km(&pair[0], &pair[1]))
        .sum();

    let start_time = positions.first().unwrap().recorded_at;
    let end_time = positions.last().unwrap().recorded_at;

    let total_duration_secs = (end_time - start_time).num_milliseconds() as f64 / 1000.0;

    if total_duration_secs <= 0.0 {
        return 0.0;
    }

    let total_hours = total_duration_secs / 3600.0;

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
        .route("/api/report", get(get_report_handler))
        .layer(middleware::from_fn(auth::jwt_admin_middleware))
}