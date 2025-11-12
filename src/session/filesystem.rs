use super::storage::SessionStore;
use crate::models::Session;
use chrono::Local;
use std::env;
use std::fs;
use std::path::{Path, PathBuf};

pub const SESSION_EXPIRY_MINUTES: i64 = 30;

pub struct FilesystemSessionStore;

impl FilesystemSessionStore {
    pub fn new() -> Self {
        Self
    }

    fn get_cache_dir(&self) -> PathBuf {
        let home = env::var("HOME").expect("HOME environment variable not set");
        let cache_dir = Path::new(&home).join(".cache").join("cmd2ai");
        if !cache_dir.exists() {
            fs::create_dir_all(&cache_dir).expect("Failed to create cache directory");
        }
        cache_dir
    }
}

impl SessionStore for FilesystemSessionStore {
    fn find_recent_session(&self) -> Option<Session> {
        let cache_dir = self.get_cache_dir();
        let now = Local::now();

        // Read all session files and find the most recent valid one
        if let Ok(entries) = fs::read_dir(&cache_dir) {
            let mut sessions: Vec<(PathBuf, Session)> = entries
                .filter_map(|entry| entry.ok())
                .filter_map(|entry| {
                    let path = entry.path();
                    if path.extension()? == "json"
                        && path.file_name()?.to_str()?.starts_with("session-")
                    {
                        let content = fs::read_to_string(&path).ok()?;
                        let session: Session = serde_json::from_str(&content).ok()?;
                        Some((path, session))
                    } else {
                        None
                    }
                })
                .collect();

            // Sort by last_updated (most recent first)
            sessions.sort_by(|a, b| b.1.last_updated.cmp(&a.1.last_updated));

            // Return the most recent session if it's not expired
            if let Some((path, session)) = sessions.first() {
                let age_minutes = now
                    .signed_duration_since(session.last_updated)
                    .num_minutes();
                if age_minutes.abs() < SESSION_EXPIRY_MINUTES {
                    return Some(session.clone());
                } else {
                    // Clean up expired session
                    let _ = fs::remove_file(path);
                }
            }
        }

        None
    }

    fn save_session(&self, session: &Session) -> Result<(), Box<dyn std::error::Error>> {
        let cache_dir = self.get_cache_dir();
        let session_file = cache_dir.join(format!("session-{}.json", session.session_id));
        let content = serde_json::to_string_pretty(session)?;
        fs::write(session_file, content)?;
        Ok(())
    }

    fn clear_all_sessions(&self) -> Result<(), Box<dyn std::error::Error>> {
        let cache_dir = self.get_cache_dir();
        if let Ok(entries) = fs::read_dir(&cache_dir) {
            for entry in entries.filter_map(|e| e.ok()) {
                let path = entry.path();
                if path.extension() == Some(std::ffi::OsStr::new("json"))
                    && path
                        .file_name()
                        .unwrap()
                        .to_str()
                        .unwrap()
                        .starts_with("session-")
                {
                    fs::remove_file(path)?;
                }
            }
        }
        Ok(())
    }
}

impl Default for FilesystemSessionStore {
    fn default() -> Self {
        Self::new()
    }
}

