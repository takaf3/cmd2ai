mod filesystem;
mod storage;

pub use filesystem::FilesystemSessionStore;
pub use storage::SessionStore;

use crate::models::Message;
use chrono::Local;
use uuid::Uuid;

pub const MAX_CONVERSATION_PAIRS: usize = 3; // Keep last 3 exchanges (6 messages)

/// Trim conversation history to keep only the last N exchanges
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

/// Create a new session
pub fn create_new_session() -> crate::models::Session {
    crate::models::Session {
        session_id: Uuid::new_v4().to_string(),
        last_updated: Local::now(),
        messages: vec![],
    }
}

/// Convenience functions that use the default filesystem store
pub fn find_recent_session() -> Option<crate::models::Session> {
    FilesystemSessionStore::new().find_recent_session()
}

pub fn save_session(session: &crate::models::Session) -> Result<(), Box<dyn std::error::Error>> {
    FilesystemSessionStore::new().save_session(session)
}

pub fn clear_all_sessions() -> Result<(), Box<dyn std::error::Error>> {
    FilesystemSessionStore::new().clear_all_sessions()
}

