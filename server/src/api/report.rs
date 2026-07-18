pub async fn get_report_handler(State(db): State<SharedDb>, Query(params): Query<ReportQuery>) -> impl IntoResponse;
pub struct ReportQuery { pub user_id: i64, pub period: ReportPeriod }

pub fn fetch_positions(db: &SharedDb, user_id: i64, period: &ReportPeriod) -> Result<Vec<Position>>;
pub fn fetch_sessions(db: &SharedDb, user_id: i64, period: &ReportPeriod) -> Result<Vec<MovementSession>>;
pub fn haversine_distance_km(p1: &Position, p2: &Position) -> f64;
pub fn compute_trajectory(positions: &[Position]) -> Vec<Position>;
pub fn compute_avg_speed_kmh(positions: &[Position]) -> f64;
pub fn compute_movement_duration_secs(sessions: &[MovementSession]) -> i64;
pub fn compute_pause_duration_secs(sessions: &[MovementSession]) -> i64;