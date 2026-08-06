use std::fs::OpenOptions;
use std::io::Write;
use std::time::Duration;

use sysinfo::{Pid, System};
use tracing_subscriber::EnvFilter;

// Questa è la prima cosa da fare nel main
pub fn init_tracing() {
    tracing_subscriber::fmt()
        .with_env_filter(
            EnvFilter::try_from_default_env().unwrap_or_else(|_| EnvFilter::new("info")),
        )
        .init();
}

const CPU_LOG_PATH: &str = "cpu_usage.log";
const CPU_LOG_INTERVAL_SECS: u64 = 120; // 2 minuti

// Task in background (da avviare con `tokio::spawn`) che ogni 2 minuti
// misura il tempo CPU consumato dal processo del server e lo appende
// a `cpu_usage.log`, con timestamp.
pub async fn cpu_logging_task() {
    let pid = Pid::from_u32(std::process::id());
    let mut sys = System::new();

    loop {
        sys.refresh_process(pid);
        tokio::time::sleep(Duration::from_millis(200)).await;
        sys.refresh_process(pid);

        match sys.process(pid) {
            Some(process) => {
                let cpu_usage = process.cpu_usage();
                let now = chrono::Utc::now().to_rfc3339();
                let line = format!("{now} cpu_usage={cpu_usage:.2}%\n");

                if let Err(e) = append_log_line(&line) {
                    tracing::warn!("errore scrittura log CPU: {e}");
                }
            }
            None => {
                tracing::warn!("processo con pid {} non trovato per il log CPU", pid);
            }
        }

        tokio::time::sleep(Duration::from_secs(CPU_LOG_INTERVAL_SECS)).await;
    }
}

fn append_log_line(line: &str) -> std::io::Result<()> {
    let mut file = OpenOptions::new()
        .create(true)
        .append(true)
        .open(CPU_LOG_PATH)?;
    file.write_all(line.as_bytes())
}