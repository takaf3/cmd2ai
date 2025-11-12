use cmd2ai::models::{Message, Session};
use cmd2ai::session::{FilesystemSessionStore, SessionStore};
use chrono::Local;
use std::fs;
use tempfile::TempDir;

fn create_test_session(id: &str, age_minutes: i64) -> Session {
    Session {
        session_id: id.to_string(),
        last_updated: Local::now() - chrono::Duration::minutes(age_minutes),
        messages: vec![Message {
            role: "user".to_string(),
            content: Some("test".to_string()),
            tool_calls: None,
            tool_call_id: None,
        }],
    }
}

#[test]
fn test_save_and_find_recent_session() {
    let temp_dir = TempDir::new().unwrap();
    let cache_dir = temp_dir.path().join(".cache").join("cmd2ai");
    fs::create_dir_all(&cache_dir).unwrap();

    // Override HOME for this test
    std::env::set_var("HOME", temp_dir.path().to_str().unwrap());

    let store = FilesystemSessionStore::new();
    let session = create_test_session("test-123", 0);

    // Save session
    store.save_session(&session).unwrap();

    // Find it
    let found = store.find_recent_session().unwrap();
    assert_eq!(found.session_id, "test-123");
}

#[test]
fn test_find_recent_session_expired() {
    let temp_dir = TempDir::new().unwrap();
    let cache_dir = temp_dir.path().join(".cache").join("cmd2ai");
    fs::create_dir_all(&cache_dir).unwrap();

    std::env::set_var("HOME", temp_dir.path().to_str().unwrap());

    let store = FilesystemSessionStore::new();
    let session = create_test_session("expired-123", 60); // 60 minutes old

    // Save expired session
    store.save_session(&session).unwrap();

    // Should not find expired session
    let found = store.find_recent_session();
    assert!(found.is_none());
}

#[test]
fn test_clear_all_sessions() {
    let temp_dir = TempDir::new().unwrap();
    let cache_dir = temp_dir.path().join(".cache").join("cmd2ai");
    fs::create_dir_all(&cache_dir).unwrap();

    std::env::set_var("HOME", temp_dir.path().to_str().unwrap());

    let store = FilesystemSessionStore::new();
    let session1 = create_test_session("session-1", 0);
    let session2 = create_test_session("session-2", 0);

    // Save multiple sessions
    store.save_session(&session1).unwrap();
    store.save_session(&session2).unwrap();

    // Clear all
    store.clear_all_sessions().unwrap();

    // Should find nothing
    let found = store.find_recent_session();
    assert!(found.is_none());
}

#[test]
fn test_find_most_recent_session() {
    let temp_dir = TempDir::new().unwrap();
    let cache_dir = temp_dir.path().join(".cache").join("cmd2ai");
    fs::create_dir_all(&cache_dir).unwrap();

    std::env::set_var("HOME", temp_dir.path().to_str().unwrap());

    let store = FilesystemSessionStore::new();
    let old_session = create_test_session("old", 10);
    let new_session = create_test_session("new", 0);

    // Save both sessions
    store.save_session(&old_session).unwrap();
    // Wait a tiny bit to ensure different timestamps
    std::thread::sleep(std::time::Duration::from_millis(10));
    store.save_session(&new_session).unwrap();

    // Should find the most recent one
    let found = store.find_recent_session().unwrap();
    assert_eq!(found.session_id, "new");
}

