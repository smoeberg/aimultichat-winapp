// src/logger.rs
#![allow(dead_code)]
#![allow(unused_variables)]
#![allow(unused_mut)]

use serde::{Deserialize, Serialize};
use std::fs::{self, File};
use std::io::Write;
use std::path::PathBuf;
use std::time::{SystemTime, UNIX_EPOCH};

#[derive(Debug, Clone, Serialize, Deserialize)]
#[allow(dead_code)]
pub struct LogEntry {
    pub timestamp: u64,
    pub level: String,
    pub module: String,
    pub message: String,
    pub context: Option<serde_json::Value>,
}

#[allow(dead_code)]
pub struct Logger {
    log_dir: PathBuf,
    current_file: Option<File>,
    max_file_size: u64,
    max_files: usize,
}

impl Logger {
    #[allow(dead_code)]
    pub fn new() -> Self {
        let log_dir = dirs::data_local_dir()
            .unwrap_or_else(|| PathBuf::from("."))
            .join("Eira Companion")
            .join("logs");

        fs::create_dir_all(&log_dir).unwrap_or_default();

        Self {
            log_dir,
            current_file: None,
            max_file_size: 5 * 1024 * 1024,
            max_files: 10,
        }
    }

    #[allow(dead_code)]
    pub fn log(
        &mut self,
        level: &str,
        module: &str,
        message: &str,
        context: Option<serde_json::Value>,
    ) {
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

    #[allow(dead_code)]
    fn write_entry(&mut self, entry: &LogEntry) {
        self.rotate_if_needed();

        let line = serde_json::to_string(entry).unwrap_or_default() + "\n";

        if let Some(file) = &mut self.current_file {
            let _ = file.write_all(line.as_bytes());
            let _ = file.flush();
        }
    }

    #[allow(dead_code)]
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

    #[allow(dead_code)]
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

    #[allow(dead_code)]
    pub fn get_logs(&self, lines: usize) -> String {
        let mut all_logs = String::new();
        if let Ok(entries) = fs::read_dir(&self.log_dir) {
            let mut files: Vec<_> = entries
                .filter_map(|e| e.ok())
                .filter(|e| e.file_type().map(|f| f.is_file()).unwrap_or(false))
                .collect();

            files.sort_by_key(|e| e.metadata().and_then(|m| m.modified()).ok());

            for file in files.iter().rev().take(5) {
                if let Ok(content) = fs::read_to_string(file.path()) {
                    let lines_vec: Vec<&str> = content.lines().collect();
                    let take = lines_vec.len().min(lines);
                    let start = lines_vec.len().saturating_sub(take);
                    for line in &lines_vec[start..] {
                        all_logs.push_str(line);
                        all_logs.push('\n');
                    }
                }
            }
        }
        all_logs
    }
}
