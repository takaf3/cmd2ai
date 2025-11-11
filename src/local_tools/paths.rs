use std::path::{Path, PathBuf};

/// Safely resolve a user-provided path within the base directory
/// Prevents path traversal attacks
pub fn safe_resolve_path(user_path: &str, base_dir: &Path) -> Result<PathBuf, String> {
    // Basic validation: reject empty or very long paths
    if user_path.is_empty() || user_path.len() > 4096 {
        return Err("Invalid path: path must be non-empty and under 4096 characters".to_string());
    }

    // Normalize the path (resolves . and ..)
    let normalized = PathBuf::from(user_path);

    // Resolve against base directory
    let resolved = base_dir
        .join(normalized)
        .canonicalize()
        .map_err(|e| format!("Failed to resolve path: {}", e))?;

    // Ensure the resolved path is within the base directory
    let base_canonical = base_dir
        .canonicalize()
        .map_err(|e| format!("Failed to canonicalize base directory: {}", e))?;

    if !resolved.starts_with(&base_canonical) {
        return Err(format!(
            "Path traversal detected: '{}' escapes base directory",
            user_path
        ));
    }

    Ok(resolved)
}

/// Canonicalize a path within the base directory, returning the absolute path string
/// This is used for templated command arguments to ensure paths are validated
pub fn canonicalize_within_base_dir(user_path: &str, base_dir: &Path) -> Result<String, String> {
    let resolved = safe_resolve_path(user_path, base_dir)?;
    stringify_path(&resolved)
}

/// Check if a string looks like a command-line option (starts with '-')
pub fn is_option_like(s: &str) -> bool {
    s.starts_with('-')
}

/// Convert a PathBuf to a String, handling non-UTF-8 paths gracefully
pub fn stringify_path(p: &Path) -> Result<String, String> {
    p.to_str()
        .ok_or_else(|| format!("Path contains invalid UTF-8: {}", p.display()))
        .map(|s| s.to_string())
}

