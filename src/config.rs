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
            if let Ok(config) = serde_json::from_str::<AppConfig>(&content) {
                if Self::validate_url(&config.server_url) {
                    return config;
                }
            }
        }

        // Development-only env override
        #[cfg(debug_assertions)]
        {
            if let Ok(url) = std::env::var("MULTICHAT_URL").or_else(|_| std::env::var("EIRA_CHAT_URL")) {
                if Self::validate_url(&url) {
                    return Self { server_url: url };
                }
            }
        }

        // Production default & hard enforcement
        Self {
            server_url: "https://ai.eira.dk".to_string(),
        }
    }

    pub fn validate_url(url_str: &str) -> bool {
        if let Ok(parsed) = url::Url::parse(url_str) {
            if parsed.scheme() != "https" {
                // Allow http only on localhost during debug
                #[cfg(debug_assertions)]
                {
                    if parsed.scheme() == "http" {
                        if let Some(host) = parsed.host_str() {
                            return host == "localhost" || host == "127.0.0.1";
                        }
                    }
                }
                return false;
            }

            if let Some(host) = parsed.host_str() {
                #[cfg(not(debug_assertions))]
                {
                    // Production strict allowlist: must be ai.eira.dk or subdomain
                    return host == "ai.eira.dk" || host.ends_with(".ai.eira.dk");
                }
                #[cfg(debug_assertions)]
                {
                    return true;
                }
            }
        }
        false
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
        if !Self::validate_url(&self.server_url) {
            return Err("Ugyldig eller ikke-tilladt server-URL".into());
        }

        let path = Self::get_config_path();
        if let Some(parent) = path.parent() {
            fs::create_dir_all(parent).map_err(|e| e.to_string())?;
        }

        // P1-01: Atomic write via temp file
        let temp_path = path.with_extension("tmp");
        let content = serde_json::to_string_pretty(self).map_err(|e| e.to_string())?;
        fs::write(&temp_path, content).map_err(|e| e.to_string())?;
        fs::rename(&temp_path, &path).map_err(|e| e.to_string())?;

        Ok(())
    }
}
