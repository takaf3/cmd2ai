use crate::models::Session;

/// Trait for session storage backends
pub trait SessionStore: Send + Sync {
    /// Find the most recent valid session
    fn find_recent_session(&self) -> Option<Session>;

    /// Save a session
    fn save_session(&self, session: &Session) -> Result<(), Box<dyn std::error::Error>>;

    /// Clear all sessions
    fn clear_all_sessions(&self) -> Result<(), Box<dyn std::error::Error>>;
}

