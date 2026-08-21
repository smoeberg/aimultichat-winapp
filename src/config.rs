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

        Self {
            server_url: "https://ai.eira.dk".to_string(),
        }
    }

    pub fn validate_url(url_str: &str) -> bool {
        if let Ok(parsed) = url::Url::parse(url_str) {
            // Must be https in production, http only localhost in debug
            if parsed.scheme() != "https" {
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

            // Reject credentials (username/password in URL)
            if !parsed.username().is_empty() || parsed.password().is_some() {
                return false;
            }

            // Reject non-standard paths (must be root or empty)
            let path = parsed.path();
            if path != "/" && !path.is_empty() {
                return false;
            }

            // Reject query strings and fragments in base config URL
            if parsed.query().is_some() || parsed.fragment().is_some() {
                return false;
            }

            if let Some(host) = parsed.host_str() {
                #[cfg(not(debug_assertions))]
                {
                    // Production strict allowlist: ai.eira.dk or subdomain, no custom ports
                    if parsed.port().is_some() {
                        return false;
                    }
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
            .unwrap_or_else(|_| {
                // Fallback to local temp if APPDATA is missing
                std::env::temp_dir().join("eira-fallback")
            });
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

        // Atomic write via temp file in the same directory
        let temp_filename = format!("config_{}.tmp", std::process::id());
        let temp_path = path.with_file_name(temp_filename);
        
        let content = serde_json::to_string_pretty(self).map_err(|e| e.to_string())?;
        fs::write(&temp_path, content).map_err(|e| e.to_string())?;
        
        // Ensure atomic replace
        if let Err(e) = fs::rename(&temp_path, &path) {
            let _ = fs::remove_file(&temp_path);
            return Err(e.to_string());
        }

        Ok(())
    }
}
