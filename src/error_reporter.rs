use reqwest::Client;
use serde::{Deserialize, Serialize};
use std::path::PathBuf;
use std::time::{Duration, SystemTime, UNIX_EPOCH};

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
    client: Client,
    server_url: String,
    last_report_time: std::time::Instant,
    min_interval: Duration,
    device_id: String,
}

impl ErrorReporter {
    pub fn new(server_url: &str) -> Self {
        let device_id = Self::get_device_id();

        Self {
            client: Client::new(),
            server_url: server_url.to_string(),
            last_report_time: std::time::Instant::now() - Duration::from_secs(120), // allow immediate first report
            min_interval: Duration::from_secs(60), // Max 1 report per minute
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

    pub async fn report_error(
        &mut self,
        error_type: &str,
        error_message: &str,
        stack_trace: Option<&str>,
        context: Option<serde_json::Value>,
        request_id: Option<&str>,
    ) -> Result<ReportResponse, String> {
        // Rate limiting
        if self.last_report_time.elapsed() < self.min_interval {
            return Err("Rate limited (max 1 report per minute)".to_string());
        }
        self.last_report_time = std::time::Instant::now();

        let report = ErrorReport {
            app_version: env!("CARGO_PKG_VERSION").to_string(),
            os_version: std::env::consts::OS.to_string(),
            error_type: error_type.to_string(),
            error_message: error_message.to_string(),
            stack_trace: stack_trace.map(|s| s.to_string()),
            timestamp: SystemTime::now()
                .duration_since(UNIX_EPOCH)
                .unwrap_or_default()
                .as_secs(),
            device_id: self.device_id.clone(),
            request_id: request_id.map(|s| s.to_string()),
            context,
        };

        let response = self.client
            .post(format!("{}/api/companion/error", self.server_url))
            .json(&report)
            .timeout(Duration::from_secs(5))
            .send()
            .await
            .map_err(|e| format!("Kunne ikke sende fejlrapport til server: {}", e))?;

        if !response.status().is_success() {
            return Err(format!("Server svarede med status: {}", response.status()));
        }

        let data: ReportResponse = response
            .json()
            .await
            .map_err(|e| format!("Kunne ikke parse serversvar: {}", e))?;

        Ok(data)
    }
}
