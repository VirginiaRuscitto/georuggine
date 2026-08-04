use chrono::{Datelike, Days, NaiveDate};
use axum::{extract::State, extract::Query, response::IntoResponse, Json};
use chrono::{DateTime, Utc};
use tokio;
use crate::{
    database::connection::SharedDb,
    models::{MovementSession, RouteReport, Position, ReportPeriod, MovementState},
    dao::{movement_sessions_dao, position_log_dao}
};

pub struct ReportQuery { pub user_id: i64, pub period: ReportPeriod }

pub async fn get_report_handler(
    State(db): State<SharedDb>,
    Query(params): Query<ReportQuery>,
) -> impl IntoResponse {
    let (start, end) = get_start_end_from_report_period(params.period);

    let db_1 = db.clone();
    let db_2 = db.clone();
    let user_id = params.user_id;

    // Pipeline 1: Fetch positions AND compute speed on Thread 1
    let position_pipeline = tokio::task::spawn_blocking(move || {
        let positions = position_log_dao::get_positions_in_range(&db_1, user_id, start, end)?;
        let avg_speed_kmh = compute_avg_speed_kmh(&positions);

        Ok::<(Vec<Position>, f64), rusqlite::Error>((positions, avg_speed_kmh))
    });

    // Pipeline 2: Fetch sessions AND compute time durations on Thread 2
    let session_pipeline = tokio::task::spawn_blocking(move || {
        let sessions = movement_sessions_dao::get_sessions_in_range(&db_2, user_id, start, end)?;
        let (movement_duration_secs, pause_duration_secs) = compute_durations(&sessions, end);

        Ok::<(i64, i64), rusqlite::Error>((movement_duration_secs, pause_duration_secs))
    });

    // Run both entire pipelines concurrently
    let (pos_res, sess_res) = tokio::join!(position_pipeline, session_pipeline);

    let (trajectory, avg_speed_kmh) = pos_res.unwrap().unwrap();
    let (movement_duration_secs, pause_duration_secs) = sess_res.unwrap().unwrap();

    let report = RouteReport {
        user_id: params.user_id,
        period: params.period,
        trajectory,
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

pub fn compute_durations(sessions: &[MovementSession], default_end: DateTime<Utc>) -> (i64, i64) {
    sessions
        .iter()
        .fold((0, 0), |(mut move_acc, mut pause_acc), session| {
            let end = session.ended_at.unwrap_or(default_end);
            let duration = (end - session.started_at).num_seconds();

            match session.state {
                MovementState::Moving => move_acc += duration,
                MovementState::Stopped => pause_acc += duration,
            }

            (move_acc, pause_acc)
        })
}

#[cfg(test)]
mod tests {
    use std::thread;
    use axum::extract::{Query, State};
    use chrono::{Duration, Utc};
    use rusqlite::{params};
    use tokio::time::Instant;
    use crate::dao::{movement_sessions_dao, position_log_dao, users_dao};
    use crate::database::connection::SharedDb;
    use crate::handlers::report::{get_report_handler, ReportQuery};
    use crate::models::{MovementState, NewUser, ReportPeriod};

    pub fn clean_db(db: &SharedDb) {
        let mut conn = db.lock().unwrap();
        let tx = conn.transaction().unwrap();

        // 1. Delete contents in reverse order of table relationships (children first)
        tx.execute("DELETE FROM position_log", []);
        tx.execute("DELETE FROM movement_sessions", []);
        tx.execute("DELETE FROM users", []);

        // 2. Reset AUTOINCREMENT sequence counters back to 1
        tx.execute(
            "DELETE FROM sqlite_sequence WHERE name IN ('users', 'movement_sessions', 'position_log')",
            [],
        )
            .ok(); // .ok() prevents failure if sqlite_sequence table hasn't been created yet

        tx.commit();
    }
    fn populate_db( num_users: usize, sessions_per_user: usize, positions_per_user: usize ){
        let db: SharedDb = crate::database::connection::shared_connection().unwrap();

        clean_db(&db);

        let mut rng = rand::thread_rng();

        // Start time set to 00:00:00 UTC of current day to match ReportPeriod::Day
        let start_of_day = Utc::now()
            .date_naive()
            .and_hms_opt(0, 0, 0)
            .unwrap()
            .and_utc();

        // Calculate available CPU threads
        let num_threads = thread::available_parallelism()
            .map(|n| n.get())
            .unwrap_or(4);
        let users_per_thread = num_users / num_threads;

        let seeding_start = Instant::now();

        // Batch insertions into a single transaction to drastically speed up setup
        {
            let conn = db.lock().unwrap();
            conn.execute_batch("BEGIN TRANSACTION;").unwrap();
        }

        // 1. Multithreaded Database Seeding using std::thread::scope
        thread::scope(|scope| {
            for thread_idx in 0..num_threads {
                let db_ref = &db;

                // Determine user index range for this worker thread
                let start_user = thread_idx * users_per_thread;
                let end_user = if thread_idx == num_threads - 1 {
                    num_users
                } else {
                    start_user + users_per_thread
                };

                scope.spawn(move || {
                    for u in start_user..end_user {
                        // A. Insert User via DAO
                        let new_user = NewUser {
                            name: format!("User{}", u),
                            surname: format!("Test{}", u),
                            email: format!("user{}@example.com", u),
                            password_hash: "hashed_pass_123".to_string(),
                        };

                        let user_id = users_dao::insert_user(db_ref, &new_user)
                            .expect("Failed to insert user");

                        // B. Insert 10 Sessions per user via movement_sessions_dao
                        let mut session_cursor = start_of_day;
                        for s in 0..sessions_per_user {
                            let state = if s % 2 == 0 {
                                MovementState::Moving
                            } else {
                                MovementState::Stopped
                            };

                            let started_at = session_cursor;
                            let duration_secs = 300 + (s as i64 * 90); // Vary session lengths
                            let ended_at = started_at + Duration::seconds(duration_secs);
                            session_cursor = ended_at + Duration::seconds(120);

                            let session_id = movement_sessions_dao::insert_movement_session(
                                db_ref,
                                user_id,
                                state,
                                started_at,
                            )
                                .expect("Failed to insert session");

                            movement_sessions_dao::close_movement_session(
                                db_ref,
                                session_id,
                                ended_at,
                            )
                                .expect("Failed to close session");
                        }

                        // C. Insert 10,000 Position Logs per user via position_log_dao
                        let base_lat = 45.4642;
                        let base_lon = 9.1900;

                        for p in 0..positions_per_user {
                            let lat = base_lat + (p as f64 * 0.00005);
                            let lon = base_lon + (p as f64 * 0.00005);

                            // Space out positions by 2 seconds each across the day
                            let recorded_at = start_of_day + Duration::seconds(p as i64 * 2);

                            let pos_id = position_log_dao::insert_position(db_ref, user_id, lat, lon)
                                .expect("Failed to insert position");

                            // Override timestamp to ensure it falls within ReportPeriod::Day
                            let conn = db_ref.lock().unwrap();
                            conn.execute(
                                "UPDATE position_log SET recorded_at = ?1 WHERE id = ?2",
                                params![recorded_at, pos_id],
                            )
                                .expect("Failed to update position timestamp");
                        }
                    }
                });
            }
        });

        {
            let conn = db.lock().unwrap();
            conn.execute_batch("COMMIT;").unwrap();
        }

        println!("Multithreaded seeding completed in: {:?}", seeding_start.elapsed());
    }

    #[tokio::test]
    async fn test_get_report_handler_cpu_performance() {
        let num_users = 100;
        let sessions_per_user = 10;
        let positions_per_user = 10_000;

        populate_db(num_users, sessions_per_user,positions_per_user);

        // 4. Run `get_report_handler` and measure process CPU time
        let start_time = Instant::now();

        let db: SharedDb = crate::database::connection::shared_connection().unwrap();

        for u_id in 0..num_users as i64 {
            let query = ReportQuery {
                user_id: u_id,
                period: ReportPeriod::Day,
            };

            let _response = get_report_handler(State(db.clone()), Query(query)).await;

            let query = ReportQuery {
                user_id: u_id,
                period: ReportPeriod::Week,
            };

            let _response = get_report_handler(State(db.clone()), Query(query)).await;

            let query = ReportQuery {
                user_id: u_id,
                period: ReportPeriod::Month,
            };

            let _response = get_report_handler(State(db.clone()), Query(query)).await;
        }

        let elapsed = start_time.elapsed();

        // 5. Output measured performance
        println!("\n==============================================");
        println!("Execution time of 'get_report_handler': {:?}", elapsed);
        println!("==============================================\n");
    }
}