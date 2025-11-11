use super::registry::{LocalSettings, LocalToolRegistry};
use serde_json::{json, Value};
use std::fs;
use std::path::{Path, PathBuf};

pub fn format_tools_for_llm(registry: &LocalToolRegistry) -> Vec<Value> {
    registry
        .list()
        .iter()
        .map(|tool| {
            json!({
                "type": "function",
                "function": {
                    "name": tool.name,
                    "description": tool.description,
                    "parameters": tool.input_schema,
                }
            })
        })
        .collect()
}

pub async fn call_local_tool(
    registry: &LocalToolRegistry,
    tool_name: &str,
    arguments: &Value,
) -> Result<String, String> {
    // Validate arguments first
    registry.validate_arguments(tool_name, arguments)?;

    // Get the tool
    let tool = registry
        .get(tool_name)
        .ok_or_else(|| format!("Tool '{}' not found", tool_name))?;

    // Call the handler (now async)
    let handler = &tool.handler;
    handler(arguments, registry.settings()).await
}

// Tool handlers

pub fn handle_echo(args: &Value, _settings: &LocalSettings) -> Result<String, String> {
    let text = args
        .get("text")
        .and_then(|v| v.as_str())
        .ok_or_else(|| "Missing required argument: text".to_string())?;
    Ok(text.to_string())
}

pub fn handle_time_now(_args: &Value, _settings: &LocalSettings) -> Result<String, String> {
    let now = chrono::Utc::now();
    Ok(now.to_rfc3339())
}

pub fn handle_read_file(args: &Value, settings: &LocalSettings) -> Result<String, String> {
    let path_str = args
        .get("path")
        .and_then(|v| v.as_str())
        .ok_or_else(|| "Missing required argument: path".to_string())?;

    // Safely resolve the path
    let resolved_path = safe_resolve_path(path_str, &settings.base_dir)?;

    // Check if file exists
    if !resolved_path.exists() {
        return Err(format!("File not found: {}", path_str));
    }

    // Check if it's a file (not a directory)
    if !resolved_path.is_file() {
        return Err(format!("Path is not a file: {}", path_str));
    }

    // Check file size
    let metadata = fs::metadata(&resolved_path)
        .map_err(|e| format!("Failed to read file metadata: {}", e))?;
    if metadata.len() > settings.max_file_size_bytes {
        return Err(format!(
            "File too large: {} bytes (max: {} bytes)",
            metadata.len(),
            settings.max_file_size_bytes
        ));
    }

    // Read file (UTF-8 only)
    fs::read_to_string(&resolved_path)
        .map_err(|e| format!("Failed to read file: {}", e))
}

pub fn handle_list_dir(args: &Value, settings: &LocalSettings) -> Result<String, String> {
    let path_str = args
        .get("path")
        .and_then(|v| v.as_str())
        .ok_or_else(|| "Missing required argument: path".to_string())?;

    // Safely resolve the path
    let resolved_path = safe_resolve_path(path_str, &settings.base_dir)?;

    // Check if directory exists
    if !resolved_path.exists() {
        return Err(format!("Directory not found: {}", path_str));
    }

    // Check if it's a directory
    if !resolved_path.is_dir() {
        return Err(format!("Path is not a directory: {}", path_str));
    }

    // List directory contents
    let entries = fs::read_dir(&resolved_path)
        .map_err(|e| format!("Failed to read directory: {}", e))?;

    let mut results = Vec::new();
    for entry in entries {
        let entry = entry.map_err(|e| format!("Failed to read directory entry: {}", e))?;
        let path = entry.path();
        let name = path
            .file_name()
            .and_then(|n| n.to_str())
            .unwrap_or("<invalid>")
            .to_string();

        let file_type = if path.is_dir() {
            "directory"
        } else if path.is_file() {
            "file"
        } else {
            "other"
        };

        let metadata = entry.metadata().ok();
        let size = metadata
            .as_ref()
            .map(|m| m.len())
            .unwrap_or(0);

        results.push(format!("{} ({}, {} bytes)", name, file_type, size));
    }

    results.sort();
    Ok(results.join("\n"))
}

/// Safely resolve a user-provided path within the base directory
/// Prevents path traversal attacks
fn safe_resolve_path(user_path: &str, base_dir: &Path) -> Result<PathBuf, String> {
    // Basic validation: reject empty or very long paths
    if user_path.is_empty() || user_path.len() > 4096 {
        return Err("Invalid path: path must be non-empty and under 4096 characters".to_string());
    }

    // Normalize the path (resolves . and ..)
    let normalized = PathBuf::from(user_path);
    
    // Resolve against base directory
    let resolved = base_dir.join(normalized).canonicalize()
        .map_err(|e| format!("Failed to resolve path: {}", e))?;
    
    // Ensure the resolved path is within the base directory
    let base_canonical = base_dir.canonicalize()
        .map_err(|e| format!("Failed to canonicalize base directory: {}", e))?;
    
    if !resolved.starts_with(&base_canonical) {
        return Err(format!(
            "Path traversal detected: '{}' escapes base directory",
            user_path
        ));
    }
    
    Ok(resolved)
}

