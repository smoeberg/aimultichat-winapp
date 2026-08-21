use serde::{Deserialize, Serialize};
use std::path::PathBuf;
use std::sync::Mutex;
use std::time::{Duration, Instant};

#[derive(Debug, Serialize)]
pub struct ErrorReport {
    pub app_version: String,
    pub os_version: String,
    pub error_type: String,
    pub error_message: String,
    pub stack_trace: Option<String>,
    pub timestamp: u64,
    pub device_id: String,
    pub request_id: Option<String>,
    pub context: Option<serde_json::Value>,
}

#[derive(Debug, Deserialize)]
pub struct ReportResponse {
    pub id: String,
    pub acknowledged: bool,
}

pub struct ErrorReporter {
    pub server_url: String,
    pub last_report_time: Mutex<Instant>,
    pub min_interval: Duration,
    pub device_id: String,
}

impl ErrorReporter {
    pub fn new(server_url: &str) -> Self {
        let device_id = Self::get_device_id();

        Self {
            server_url: server_url.to_string(),
            last_report_time: Mutex::new(Instant::now() - Duration::from_secs(120)),
            min_interval: Duration::from_secs(60),
            device_id,
        }
    }

    fn get_device_id() -> String {
        let app_data = std::env::var("APPDATA")
            .map(PathBuf::from)
            .unwrap_or_else(|_| std::env::temp_dir().join("eira-fallback"));
        let path = app_data.join("Eira Companion").join("device_id");

        if let Ok(content) = std::fs::read_to_string(&path) {
            let trimmed = content.trim();
            if !trimmed.is_empty() {
                return trimmed.to_string();
            }
        }

        let id = uuid::Uuid::new_v4().to_string();
        if let Some(parent) = path.parent() {
            let _ = std::fs::create_dir_all(parent);
        }
        let _ = std::fs::write(&path, &id);
        id
    }

    pub fn update_server_url(&mut self, url: &str) {
        self.server_url = url.to_string();
    }
}
