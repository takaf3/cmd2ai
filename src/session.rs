use crate::models::{Message, Session};
use std::env;
use std::fs;
use std::path::{Path, PathBuf};
use uuid::Uuid;

pub const SESSION_EXPIRY_MINUTES: i64 = 30;
pub const MAX_CONVERSATION_PAIRS: usize = 3; // Keep last 3 exchanges (6 messages)

pub fn get_cache_dir() -> PathBuf {
    let home = env::var("HOME").expect("HOME environment variable not set");
    let cache_dir = Path::new(&home).join(".cache").join("cmd2ai");
    if !cache_dir.exists() {
        fs::create_dir_all(&cache_dir).expect("Failed to create cache directory");
    }
    cache_dir
}

pub fn find_recent_session() -> Option<Session> {
    let cache_dir = get_cache_dir();
    let now = chrono::Local::now();

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
            let age_minutes = (now - session.last_updated).num_minutes();
            if age_minutes < SESSION_EXPIRY_MINUTES {
                return Some(session.clone());
            } else {
                // Clean up expired session
                let _ = fs::remove_file(path);
            }
        }
    }

    None
}

pub fn save_session(session: &Session) -> Result<(), Box<dyn std::error::Error>> {
    let cache_dir = get_cache_dir();
    let session_file = cache_dir.join(format!("session-{}.json", session.session_id));
    let content = serde_json::to_string_pretty(session)?;
    fs::write(session_file, content)?;
    Ok(())
}

pub fn clear_all_sessions() -> Result<(), Box<dyn std::error::Error>> {
    let cache_dir = get_cache_dir();
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

pub fn trim_conversation_history(messages: &mut Vec<Message>) {
    // Keep system message (if exists) + last N conversation pairs
    let mut system_messages: Vec<Message> = messages
        .iter()
        .filter(|m| m.role == "system")
        .cloned()
        .collect();

    let conversation_messages: Vec<Message> = messages
        .iter()
        .filter(|m| m.role != "system")
        .cloned()
        .collect();

    // Keep only the last MAX_CONVERSATION_PAIRS exchanges
    let keep_count = MAX_CONVERSATION_PAIRS * 2; // Each pair has user + assistant
    let trimmed: Vec<Message> = conversation_messages
        .into_iter()
        .rev()
        .take(keep_count)
        .collect::<Vec<_>>()
        .into_iter()
        .rev()
        .collect();

    messages.clear();
    messages.append(&mut system_messages);
    messages.extend(trimmed);
}

pub fn create_new_session() -> Session {
    Session {
        session_id: Uuid::new_v4().to_string(),
        last_updated: chrono::Local::now(),
        messages: vec![],
    }
}
