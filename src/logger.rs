use serde::{Deserialize, Serialize};
use std::fs::{self, File};
use std::io::{Read, Write};
use std::path::PathBuf;
use std::time::{SystemTime, UNIX_EPOCH};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LogEntry {
    pub timestamp: u64,
    pub level: String,
    pub module: String,
    pub message: String,
    pub context: Option<serde_json::Value>,
}

pub struct Logger {
    log_dir: PathBuf,
    current_file: Option<File>,
    max_file_size: u64,
    max_files: usize,
}

impl Logger {
    pub fn new() -> Self {
        let log_dir = Self::get_log_dir();
        fs::create_dir_all(&log_dir).unwrap_or_default();

        Self {
            log_dir,
            current_file: None,
            max_file_size: 5 * 1024 * 1024, // 5 MB
            max_files: 10,
        }
    }

    fn get_log_dir() -> PathBuf {
        let app_data = std::env::var("APPDATA")
            .map(PathBuf::from)
            .unwrap_or_else(|_| std::env::temp_dir().join("eira-fallback"));
        app_data.join("Eira Companion").join("logs")
    }

    pub fn log(&mut self, level: &str, module: &str, message: &str, context: Option<serde_json::Value>) {
        let entry = LogEntry {
            timestamp: SystemTime::now()
                .duration_since(UNIX_EPOCH)
                .unwrap_or_default()
                .as_secs(),
            level: level.to_string(),
            module: module.to_string(),
            message: message.to_string(),
            context,
        };

        self.write_entry(&entry);
    }

    fn write_entry(&mut self, entry: &LogEntry) {
        self.rotate_if_needed();

        let line = match serde_json::to_string(entry) {
            Ok(l) => l + "\n",
            Err(_) => return,
        };

        if let Some(file) = &mut self.current_file {
            let _ = file.write_all(line.as_bytes());
            let _ = file.flush();
        }
    }

    fn rotate_if_needed(&mut self) {
        if let Some(file) = &self.current_file {
            if let Ok(metadata) = file.metadata() {
                if metadata.len() > self.max_file_size {
                    self.current_file = None;
                }
            }
        }

        if self.current_file.is_none() {
            let timestamp = SystemTime::now()
                .duration_since(UNIX_EPOCH)
                .unwrap_or_default()
                .as_secs();
            let path = self.log_dir.join(format!("companion_{}.log", timestamp));
            if let Ok(file) = File::create(&path) {
                self.current_file = Some(file);
            }
        }

        self.cleanup_old_logs();
    }

    fn cleanup_old_logs(&self) {
        if let Ok(entries) = fs::read_dir(&self.log_dir) {
            let mut files: Vec<_> = entries
                .filter_map(|e| e.ok())
                .filter(|e| e.file_type().map(|f| f.is_file()).unwrap_or(false))
                .collect();

            files.sort_by_key(|e| e.metadata().and_then(|m| m.modified()).ok());

            while files.len() > self.max_files {
                if let Some(oldest) = files.first() {
                    let _ = fs::remove_file(oldest.path());
                }
                files.remove(0);
            }
        }
    }

    pub fn get_logs(&self, max_lines: usize) -> String {
        let mut all_logs = Vec::new();
        if let Ok(entries) = fs::read_dir(&self.log_dir) {
            let mut files: Vec<_> = entries
                .filter_map(|e| e.ok())
                .filter(|e| e.file_type().map(|f| f.is_file()).unwrap_or(false))
                .collect();

            // Sort newest first
            files.sort_by_key(|e| std::cmp::Reverse(e.metadata().and_then(|m| m.modified()).ok()));

            for file_entry in files {
                if let Ok(mut f) = File::open(file_entry.path()) {
                    let mut content = String::new();
                    if f.read_to_string(&mut content).is_ok() {
                        for line in content.lines().rev() {
                            all_logs.push(line.to_string());
                            if all_logs.len() >= max_lines {
                                break;
                            }
                        }
                    }
                }
                if all_logs.len() >= max_lines {
                    break;
                }
            }
        }
        all_logs.reverse();
        all_logs.join("\n")
    }
}
