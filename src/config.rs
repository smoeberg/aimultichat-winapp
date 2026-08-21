use serde::{Deserialize, Serialize};
use std::fs;
use std::path::PathBuf;

#[derive(Debug, Deserialize, Serialize, Clone)]
pub struct AppConfig {
    pub server_url: String,
}

impl AppConfig {
    pub fn load() -> Self {
        let config_path = Self::get_config_path();
        if let Ok(content) = fs::read_to_string(&config_path) {
            if let Ok(config) = serde_json::from_str(&content) {
                return config;
            }
        }

        if let Ok(url) = std::env::var("MULTICHAT_URL") {
            return Self { server_url: url };
        }

        if let Ok(url) = std::env::var("EIRA_CHAT_URL") {
            return Self { server_url: url };
        }

        Self {
            server_url: "https://ai.eira.dk".to_string(),
        }
    }

    fn get_config_path() -> PathBuf {
        let app_data = std::env::var("APPDATA")
            .map(PathBuf::from)
            .unwrap_or_else(|_| PathBuf::from("."));
        app_data
            .join("Eira Companion")
            .join("config.json")
    }

    pub fn save(&self) -> Result<(), String> {
        let path = Self::get_config_path();
        if let Some(parent) = path.parent() {
            fs::create_dir_all(parent).map_err(|e| e.to_string())?;
        }
        let content = serde_json::to_string_pretty(self).map_err(|e| e.to_string())?;
        fs::write(path, content).map_err(|e| e.to_string())?;
        Ok(())
    }
}
