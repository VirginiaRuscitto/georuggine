use chrono::{Datelike, Days, NaiveDate};
use axum::{extract::State, extract::Query, response::IntoResponse, Json};
use chrono::{DateTime, Utc};
use rusqlite::{ Result };
use crate::{
    database::connection::SharedDb,
    models::{MovementSession, RouteReport, Position, ReportPeriod, MovementState},
    dao::{movement_sessions_dao, position_log_dao}
};

pub async fn get_report_handler(State(db): State<SharedDb>, Query(params): Query<ReportQuery>) -> impl IntoResponse{
    let sessions = fetch_sessions(&db, params.user_id, &params.period).unwrap();
    let positions = fetch_positions(&db, params.user_id, &params.period).unwrap();

    let report = RouteReport{
        user_id: params.user_id,
        period: params.period,
        trajectory: positions.clone(),
        avg_speed_kmh: compute_avg_speed_kmh(&positions),
        movement_duration_secs: compute_movement_duration_secs(&sessions),
        pause_duration_secs: compute_pause_duration_secs(&sessions),
    };

    Json(report).into_response()
}

pub fn get_start_end_from_report_period(report_period: ReportPeriod) -> (DateTime<Utc>, DateTime<Utc>){
    match report_period{
        ReportPeriod::Day => {
            let start_of_the_day = Utc::now()
                .date_naive()
                .and_hms_opt(0,0,0)
                .expect("00:00:00 is always a valid time")
                .and_utc();
            ( start_of_the_day, Utc::now() )
        },
        ReportPeriod::Week => {
            let days_from_monday = Utc::now().date_naive().weekday().num_days_from_monday();

            let start_of_the_week = Utc::now().date_naive()
                .checked_sub_days(Days::new(days_from_monday as u64))
                .expect("Date subtraction will not overflow for realistic dates")
                .and_hms_opt(0, 0, 0)
                .expect("00:00:00 is always a valid time")
                .and_utc();
            (start_of_the_week, Utc::now())
        },
        ReportPeriod::Month => {
            let start_of_the_month = NaiveDate::from_ymd_opt(Utc::now().year(), Utc::now().month(), 1)
                .expect("Day 1 is always valid for any valid year/month combination")
                .and_hms_opt(0, 0, 0)
                .expect("00:00:00 is always a valid time")
                .and_utc();
            ( start_of_the_month, Utc::now() )
        }
    }
}
pub struct ReportQuery { pub user_id: i64, pub period: ReportPeriod }

pub fn fetch_positions(db: &SharedDb, user_id: i64, period: &ReportPeriod) -> Result<Vec<Position>>{
    let (start, end) = get_start_end_from_report_period(*period);

    position_log_dao::get_positions_in_range(db, user_id, start, end)
}
pub fn fetch_sessions(db: &SharedDb, user_id: i64, period: &ReportPeriod) -> Result<Vec<MovementSession>>{
    let (start, end) = get_start_end_from_report_period(*period);

    movement_sessions_dao
    ::get_sessions_in_range(&db,
                            user_id,
                            start,
                            end
    )
}

/// Calculates the distance in kilometers between two positions using the Haversine formula.
pub fn haversine_distance_km(p1: &Position, p2: &Position) -> f64 {
    // Earth's mean radius in kilometers
    const EARTH_RADIUS_KM: f64 = 6371.0;

    let lat1 = p1.lat.to_radians();
    let lat2 = p2.lat.to_radians();
    let delta_lat = (p2.lat - p1.lat).to_radians();
    let delta_lon = (p2.lon - p1.lon).to_radians();

    // Haversine formula
    let a = (delta_lat / 2.0).sin().powi(2)
        + lat1.cos() * lat2.cos() * (delta_lon / 2.0).sin().powi(2);

    let c = 2.0 * a.sqrt().atan2((1.0 - a).sqrt());

    EARTH_RADIUS_KM * c
}
pub fn compute_avg_speed_kmh(positions: &[Position]) -> f64{
    // Se ci sono meno di 2 posizioni, non c'è spostamento né intervallo di tempo
    if positions.len() < 2 {
        return 0.0;
    }

    // Somma la distanza totale in chilometri usando la funzione haversine
    let total_distance_km: f64 = positions
        .windows(2)
        .map(|pair| haversine_distance_km(&pair[0], &pair[1]))
        .sum();

    // Calcolo del tempo totale usando recorded_at
    let start_time = positions.first().unwrap().recorded_at;
    let end_time = positions.last().unwrap().recorded_at;

    let total_duration_secs = (end_time - start_time).num_milliseconds() as f64 / 1000.0;

    if total_duration_secs <= 0.0 {
        return 0.0;
    }

    // Conversione in ore e calcolo km/h
    let total_hours = total_duration_secs / 3600.0;

    total_distance_km / total_hours
}
pub fn compute_movement_duration_secs(sessions: &[MovementSession]) -> i64{
    let now = Utc::now();

    sessions
        .iter()
        .filter(|session| session.state == MovementState::Moving)
        .map(|session| {
            let end = session.ended_at.unwrap_or(now);
            (end - session.started_at).num_seconds()
        })
        .sum()
}
pub fn compute_pause_duration_secs(sessions: &[MovementSession]) -> i64{
    let now = Utc::now();

    sessions
        .iter()
        .filter(|session| session.state == MovementState::Stopped)
        .map(|session| {
            let end = session.ended_at.unwrap_or(now);
            (end - session.started_at).num_seconds()
        })
        .sum()
}

#[cfg(test)]
mod tests {
    use axum::response::IntoResponse;
    use axum::extract::{State, Path, Query};
    use chrono::{Utc, Duration};
    use rusqlite::fallible_iterator::FallibleIterator;
    use serde::{Deserialize, Serialize};

    use crate::database::connection::SharedDb;
    use crate::dao::{users_dao, movement_sessions_dao};
    use crate::models::{NewUser, MovementState, ReportPeriod, RouteReport};
    use crate::handlers::report::{get_report_handler, ReportQuery};
    use crate::handlers::users::get_users_handler;

    #[tokio::test]
    async fn report_handler_integration_test() {
        // 1. Setup DB
        let connection: SharedDb = crate::database::connection::shared_connection().unwrap();

        connection.lock().unwrap().execute_batch("DELETE FROM movement_sessions; DELETE FROM users;").unwrap();

        let now = Utc::now();

        // 2. Preparazione dei dati (Seed)
        let user_id = users_dao::insert_user(&connection, &NewUser {
            name: "Report".to_string(),
            surname: "Tester".to_string(),
            email: "report@tester.com".to_string(),
            password_hash: "xyz".to_string(),
        }).unwrap();

        // Sessione 1: Moving (10 secondi)
        let id_1 = movement_sessions_dao::insert_movement_session(
            &connection, user_id, MovementState::Moving, now - Duration::seconds(30)
        ).unwrap();
        movement_sessions_dao::close_movement_session(&connection, id_1, now - Duration::seconds(20)).unwrap();

        // Sessione 2: Stopped (15 secondi)
        let id_2 = movement_sessions_dao::insert_movement_session(
            &connection, user_id, MovementState::Stopped, now - Duration::seconds(20)
        ).unwrap();
        movement_sessions_dao::close_movement_session(&connection, id_2, now - Duration::seconds(5)).unwrap();

        // Costruiamo la query string (es. ?period=Daily)
        let query_period = Query(ReportQuery {
            user_id,
            period: ReportPeriod::Day, // Assicurati che questo copra la data di 'now'
        });

        // 4. Esecuzione dell'handler
        let response = get_report_handler(
            State(connection),
            query_period
        ).await.into_response();

        // 5. Asserzioni di base sulla risposta HTTP
        assert_eq!(response.status(), axum::http::StatusCode::OK);

        // 6. Lettura e deserializzazione del body JSON
        let body_bytes = axum::body::to_bytes(response.into_body(), usize::MAX)
            .await
            .expect("failed to read body");

        let report: RouteReport = serde_json::from_slice(&body_bytes)
            .expect("failed to deserialize report response body");

        // 7. Asserzioni sul risultato dei calcoli esposti al client
        assert_eq!(report.movement_duration_secs, 10);
        assert_eq!(report.pause_duration_secs, 15);
    }
}