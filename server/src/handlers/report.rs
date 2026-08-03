use chrono::{Datelike, Days, NaiveDate};
use axum::{extract::State, extract::Query, response::IntoResponse, Json};
use chrono::{DateTime, Utc};
use crate::{
    database::connection::SharedDb,
    models::{MovementSession, Position, ReportPeriod},
    dao::{movement_sessions_dao}
};

pub async fn get_report_handler(State(db): State<SharedDb>, Query(params): Query<ReportQuery>) -> impl IntoResponse{
    let (start, end) = get_start_end_from_report_period(params.period);

    let report: Vec<MovementSession> =
        movement_sessions_dao
        ::get_sessions_in_range(&db,
                                params.user_id,
                                start,
                                end
        )?;

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
    todo!()
}
pub fn fetch_sessions(db: &SharedDb, user_id: i64, period: &ReportPeriod) -> Result<Vec<MovementSession>>{
    todo!()
}
pub fn haversine_distance_km(p1: &Position, p2: &Position) -> f64{
    todo!()
}
pub fn compute_trajectory(positions: &[Position]) -> Vec<Position>{
    todo!()
}
pub fn compute_avg_speed_kmh(positions: &[Position]) -> f64{
    todo!()
}
pub fn compute_movement_duration_secs(sessions: &[MovementSession]) -> i64{
    todo!()
}
pub fn compute_pause_duration_secs(sessions: &[MovementSession]) -> i64{
    todo!()
}