use cmd2ai::config::{LocalToolConfig, TemplateValidation};
use cmd2ai::local_tools::paths::{canonicalize_within_base_dir, is_option_like, safe_resolve_path};
use tempfile::TempDir;

    // Note: template_args is private, so we test the path utilities and config validation
    // Integration tests would test the full flow through execute_command

    #[test]
    fn test_is_option_like() {
        assert!(is_option_like("-a"));
        assert!(is_option_like("--help"));
        assert!(is_option_like("-"));
        assert!(!is_option_like("path"));
        assert!(!is_option_like("file.txt"));
        assert!(!is_option_like(""));
    }

    #[test]
    fn test_safe_resolve_path_within_base() {
        let temp_dir = TempDir::new().unwrap();
        let base_dir = temp_dir.path();
        
        // Create a test file
        let test_file = base_dir.join("test.txt");
        std::fs::write(&test_file, "test").unwrap();
        
        // Should resolve relative path correctly
        let resolved = safe_resolve_path("test.txt", base_dir).unwrap();
        assert_eq!(resolved, test_file.canonicalize().unwrap());
    }

#[test]
fn test_safe_resolve_path_rejects_traversal() {
    let temp_dir = TempDir::new().unwrap();
    let base_dir = temp_dir.path();
    
    // Create a subdirectory to test traversal
    let subdir = base_dir.join("subdir");
    std::fs::create_dir_all(&subdir).unwrap();
    
    // Should reject path traversal (even if it doesn't escape in practice)
    let result = safe_resolve_path("../../../etc/passwd", base_dir);
    assert!(result.is_err());
    // The error might be "Path traversal detected" or "Failed to resolve path" depending on the actual path
    let err_msg = result.unwrap_err();
    assert!(
        err_msg.contains("Path traversal detected") || 
        err_msg.contains("Failed to resolve path") ||
        err_msg.contains("escapes base directory")
    );
}

    #[test]
    fn test_safe_resolve_path_rejects_absolute() {
        let temp_dir = TempDir::new().unwrap();
        let base_dir = temp_dir.path();
        
        // Should reject absolute paths outside base_dir
        let result = safe_resolve_path("/etc/passwd", base_dir);
        assert!(result.is_err());
    }

    #[test]
    fn test_safe_resolve_path_rejects_empty() {
        let temp_dir = TempDir::new().unwrap();
        let base_dir = temp_dir.path();
        
        let result = safe_resolve_path("", base_dir);
        assert!(result.is_err());
        assert!(result.unwrap_err().contains("non-empty"));
    }

    #[test]
    fn test_canonicalize_within_base_dir() {
        let temp_dir = TempDir::new().unwrap();
        let base_dir = temp_dir.path();
        
        // Create a test file
        let test_file = base_dir.join("test.txt");
        std::fs::write(&test_file, "test").unwrap();
        
        // Should return canonical absolute path string
        let canonical = canonicalize_within_base_dir("test.txt", base_dir).unwrap();
        assert!(canonical.starts_with('/') || canonical.starts_with("\\"));
        assert!(canonical.contains("test.txt"));
    }

    #[test]
    fn test_template_validation_config() {
        // Test that TemplateValidation can be deserialized
        let yaml = r#"
kind: path
allow_absolute: false
deny_patterns:
  - "\\.\\./"
"#;
        let validation: TemplateValidation = serde_yaml::from_str(yaml).unwrap();
        assert_eq!(validation.kind, "path");
        assert_eq!(validation.allow_absolute, false);
        assert_eq!(validation.deny_patterns.unwrap().len(), 1);
    }

    #[test]
    fn test_local_tool_config_with_security_fields() {
        // Test that LocalToolConfig can be deserialized with new security fields
        let yaml = r#"
name: test_tool
enabled: true
type: command
command: ls
args: ["-la", "{{path}}"]
restrict_to_base_dir: true
insert_double_dash: true
template_validations:
  path:
    kind: path
    allow_absolute: false
"#;
        let config: LocalToolConfig = serde_yaml::from_str(yaml).unwrap();
        assert_eq!(config.name, "test_tool");
        assert_eq!(config.restrict_to_base_dir, true);
        assert_eq!(config.insert_double_dash, Some(true));
        assert!(config.template_validations.is_some());
    }

    #[test]
    fn test_local_tool_config_defaults() {
        // Test that security fields have secure defaults
        let yaml = r#"
name: test_tool
enabled: true
type: command
command: ls
args: []
"#;
        let config: LocalToolConfig = serde_yaml::from_str(yaml).unwrap();
        // restrict_to_base_dir should default to true
        assert_eq!(config.restrict_to_base_dir, true);
        // insert_double_dash should default to None (auto-detect)
        assert_eq!(config.insert_double_dash, None);
    }

