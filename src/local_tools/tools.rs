use super::registry::{LocalSettings, LocalToolRegistry};
use colored::Colorize;
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
    let settings = registry.settings();
    if settings.verbose {
        let args_str = serde_json::to_string(arguments)
            .unwrap_or_else(|_| "<invalid json>".to_string());
        let truncated = if args_str.len() > 200 {
            format!("{}...", &args_str[..200])
        } else {
            args_str
        };
        eprintln!(
            "{}",
            format!("[tools] Calling tool '{}' with args: {}", tool_name, truncated).dimmed()
        );
    }

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

pub fn handle_read_file(args: &Value, settings: &LocalSettings) -> Result<String, String> {
    let path_str = args
        .get("path")
        .and_then(|v| v.as_str())
        .ok_or_else(|| "Missing required argument: path".to_string())?;

    if settings.verbose {
        eprintln!(
            "{}",
            format!(
                "[tools] Resolving path: '{}' (base_dir={})",
                path_str,
                settings.base_dir.display()
            )
            .dimmed()
        );
    }

    // Safely resolve the path
    let resolved_path = safe_resolve_path(path_str, &settings.base_dir)?;

    if settings.verbose {
        eprintln!(
            "{}",
            format!("[tools] Resolved path: {} -> {}", path_str, resolved_path.display())
                .dimmed()
        );
    }

    // Check if file exists
    if !resolved_path.exists() {
        return Err(format!("File not found: {}", path_str));
    }

    // Check if it's a file (not a directory)
    if !resolved_path.is_file() {
        return Err(format!("Path is not a file: {}", path_str));
    }

    // Check file size
    let metadata =
        fs::metadata(&resolved_path).map_err(|e| format!("Failed to read file metadata: {}", e))?;
    if metadata.len() > settings.max_file_size_bytes {
        return Err(format!(
            "File too large: {} bytes (max: {} bytes)",
            metadata.len(),
            settings.max_file_size_bytes
        ));
    }

    if settings.verbose {
        eprintln!(
            "{}",
            format!("[tools] Reading file: {} ({} bytes)", resolved_path.display(), metadata.len())
                .dimmed()
        );
    }

    // Read file (UTF-8 only)
    fs::read_to_string(&resolved_path).map_err(|e| format!("Failed to read file: {}", e))
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
