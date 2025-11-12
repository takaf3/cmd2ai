use cmd2ai::local_tools::builtins::handle_read_file;
use cmd2ai::local_tools::LocalSettings;
use serde_json::json;
use std::fs;
use tempfile::TempDir;

#[test]
fn test_read_file_success() {
    let temp_dir = TempDir::new().unwrap();
    let test_file = temp_dir.path().join("test.txt");
    fs::write(&test_file, "Hello, world!").unwrap();

    let settings = LocalSettings {
        base_dir: temp_dir.path().to_path_buf(),
        max_file_size_bytes: 1024,
        verbose: false,
    };

    let args = json!({
        "path": "test.txt"
    });

    let result = handle_read_file(&args, &settings).unwrap();
    assert_eq!(result, "Hello, world!");
}

#[test]
fn test_read_file_missing_path() {
    let temp_dir = TempDir::new().unwrap();
    let settings = LocalSettings {
        base_dir: temp_dir.path().to_path_buf(),
        max_file_size_bytes: 1024,
        verbose: false,
    };

    let args = json!({});

    let result = handle_read_file(&args, &settings);
    assert!(result.is_err());
    assert!(result.unwrap_err().contains("Missing required argument: path"));
}

#[test]
fn test_read_file_not_found() {
    let temp_dir = TempDir::new().unwrap();
    let settings = LocalSettings {
        base_dir: temp_dir.path().to_path_buf(),
        max_file_size_bytes: 1024,
        verbose: false,
    };

    let args = json!({
        "path": "nonexistent.txt"
    });

    let result = handle_read_file(&args, &settings);
    assert!(result.is_err());
    assert!(result.unwrap_err().contains("File not found"));
}

#[test]
fn test_read_file_too_large() {
    let temp_dir = TempDir::new().unwrap();
    let test_file = temp_dir.path().join("large.txt");
    let large_content = "x".repeat(2048);
    fs::write(&test_file, large_content).unwrap();

    let settings = LocalSettings {
        base_dir: temp_dir.path().to_path_buf(),
        max_file_size_bytes: 1024, // Smaller than file size
        verbose: false,
    };

    let args = json!({
        "path": "large.txt"
    });

    let result = handle_read_file(&args, &settings);
    assert!(result.is_err());
    assert!(result.unwrap_err().contains("File too large"));
}

#[test]
fn test_read_file_path_traversal_prevention() {
    let temp_dir = TempDir::new().unwrap();
    let settings = LocalSettings {
        base_dir: temp_dir.path().to_path_buf(),
        max_file_size_bytes: 1024,
        verbose: false,
    };

    // Try to access file outside base_dir
    let args = json!({
        "path": "../../etc/passwd"
    });

    let result = handle_read_file(&args, &settings);
    assert!(result.is_err());
    // Should fail due to path traversal prevention
}

