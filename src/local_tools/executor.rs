use crate::config::LocalToolConfig;
use crate::config::McpConfig;
use serde_json::Value;
use std::fs;
use std::path::{Path, PathBuf};
use std::process::Stdio;
use std::time::Duration;
use tokio::process::Command;
use tokio::io::AsyncWriteExt;
use tokio::time::timeout;

use super::registry::LocalSettings;

/// Execute a dynamic tool (script or command)
pub async fn execute_dynamic_tool(
    tool_config: &LocalToolConfig,
    arguments: &Value,
    settings: &LocalSettings,
) -> Result<String, String> {
    let tool_type = tool_config.r#type.as_deref().ok_or_else(|| {
        format!("Tool '{}' is missing 'type' field (must be 'script' or 'command')", tool_config.name)
    })?;

    match tool_type {
        "script" => execute_script(tool_config, arguments, settings).await,
        "command" => execute_command(tool_config, arguments, settings).await,
        _ => Err(format!("Unknown tool type '{}' for tool '{}'", tool_type, tool_config.name)),
    }
}

async fn execute_script(
    tool_config: &LocalToolConfig,
    arguments: &Value,
    settings: &LocalSettings,
) -> Result<String, String> {
    let interpreter = tool_config.interpreter.as_ref().ok_or_else(|| {
        format!("Tool '{}' (type: script) requires 'interpreter' field", tool_config.name)
    })?;

    // Determine script source: inline or file path
    let script_path = if let Some(ref inline_script) = tool_config.script {
        // Write inline script to temporary file
        let temp_dir = settings.base_dir.join(".cmd2ai-tools").join("tmp");
        fs::create_dir_all(&temp_dir).map_err(|e| {
            format!("Failed to create temp directory: {}", e)
        })?;
        
        let temp_file = temp_dir.join(format!("{}.{}", 
            tool_config.name.replace('/', "_"),
            get_script_extension(interpreter)
        ));
        
        fs::write(&temp_file, inline_script).map_err(|e| {
            format!("Failed to write script file: {}", e)
        })?;
        
        // Set executable permissions (Unix-like systems)
        #[cfg(unix)]
        {
            use std::os::unix::fs::PermissionsExt;
            let mut perms = fs::metadata(&temp_file)
                .map_err(|e| format!("Failed to get file metadata: {}", e))?
                .permissions();
            perms.set_mode(0o755);
            fs::set_permissions(&temp_file, perms).map_err(|e| {
                format!("Failed to set script permissions: {}", e)
            })?;
        }
        
        temp_file
    } else if let Some(ref script_path_str) = tool_config.script_path {
        // Resolve script path relative to base_dir
        safe_resolve_path(script_path_str, &settings.base_dir)?
    } else {
        return Err(format!(
            "Tool '{}' (type: script) requires either 'script' (inline) or 'script_path' field",
            tool_config.name
        ));
    };

    // Resolve working directory
    let working_dir = if let Some(ref wd) = tool_config.working_dir {
        safe_resolve_path(wd, &settings.base_dir)?
    } else {
        settings.base_dir.clone()
    };

    // Expand environment variables
    let env_vars = McpConfig::expand_env_vars(&tool_config.env);

    // Prepare command
    let mut cmd = Command::new(interpreter);
    cmd.arg(&script_path)
        .current_dir(&working_dir)
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped());

    // Set environment variables
    for (key, value) in env_vars {
        cmd.env(key, value);
    }

    // Spawn process
    let mut child = cmd.spawn().map_err(|e| {
        format!("Failed to spawn script process: {}", e)
    })?;

    // Write arguments as JSON to stdin
    let args_json = serde_json::to_string(arguments).map_err(|e| {
        format!("Failed to serialize arguments: {}", e)
    })?;
    
    if let Some(mut stdin) = child.stdin.take() {
        stdin.write_all(args_json.as_bytes()).await.map_err(|e| {
            format!("Failed to write to stdin: {}", e)
        })?;
        stdin.flush().await.map_err(|e| {
            format!("Failed to flush stdin: {}", e)
        })?;
    }

    // Wait for process with timeout
    let timeout_duration = Duration::from_secs(tool_config.timeout_secs);
    let output = timeout(timeout_duration, child.wait_with_output()).await
        .map_err(|_| {
            format!("Script execution timed out after {} seconds", tool_config.timeout_secs)
        })?
        .map_err(|e| {
            format!("Failed to wait for process: {}", e)
        })?;

    // Check exit status
    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr);
        return Err(format!(
            "Script exited with code {}: {}",
            output.status.code().unwrap_or(-1),
            stderr
        ));
    }

    // Check output size
    if output.stdout.len() > tool_config.max_output_bytes as usize {
        return Err(format!(
            "Script output too large: {} bytes (max: {} bytes)",
            output.stdout.len(),
            tool_config.max_output_bytes
        ));
    }

    // Return stdout
    String::from_utf8(output.stdout).map_err(|e| {
        format!("Script output is not valid UTF-8: {}", e)
    })
}

async fn execute_command(
    tool_config: &LocalToolConfig,
    arguments: &Value,
    settings: &LocalSettings,
) -> Result<String, String> {
    let command = tool_config.command.as_ref().ok_or_else(|| {
        format!("Tool '{}' (type: command) requires 'command' field", tool_config.name)
    })?;

    // Resolve working directory
    let working_dir = if let Some(ref wd) = tool_config.working_dir {
        safe_resolve_path(wd, &settings.base_dir)?
    } else {
        settings.base_dir.clone()
    };

    // Expand environment variables and args
    let env_vars = McpConfig::expand_env_vars(&tool_config.env);
    let expanded_args: Vec<String> = tool_config.args.iter()
        .map(|arg| McpConfig::expand_env_var_in_string(arg))
        .collect();

    // Prepare command
    let mut cmd = Command::new(command);
    cmd.args(&expanded_args)
        .current_dir(&working_dir)
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped());

    // Set environment variables
    for (key, value) in env_vars {
        cmd.env(key, value);
    }

    // Spawn process
    let mut child = cmd.spawn().map_err(|e| {
        format!("Failed to spawn command process: {}", e)
    })?;

    // Write arguments as JSON to stdin
    let args_json = serde_json::to_string(arguments).map_err(|e| {
        format!("Failed to serialize arguments: {}", e)
    })?;
    
    if let Some(mut stdin) = child.stdin.take() {
        stdin.write_all(args_json.as_bytes()).await.map_err(|e| {
            format!("Failed to write to stdin: {}", e)
        })?;
        stdin.flush().await.map_err(|e| {
            format!("Failed to flush stdin: {}", e)
        })?;
    }

    // Wait for process with timeout
    let timeout_duration = Duration::from_secs(tool_config.timeout_secs);
    let output = timeout(timeout_duration, child.wait_with_output()).await
        .map_err(|_| {
            format!("Command execution timed out after {} seconds", tool_config.timeout_secs)
        })?
        .map_err(|e| {
            format!("Failed to wait for process: {}", e)
        })?;

    // Check exit status
    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr);
        return Err(format!(
            "Command exited with code {}: {}",
            output.status.code().unwrap_or(-1),
            stderr
        ));
    }

    // Check output size
    if output.stdout.len() > tool_config.max_output_bytes as usize {
        return Err(format!(
            "Command output too large: {} bytes (max: {} bytes)",
            output.stdout.len(),
            tool_config.max_output_bytes
        ));
    }

    // Return stdout
    String::from_utf8(output.stdout).map_err(|e| {
        format!("Command output is not valid UTF-8: {}", e)
    })
}

/// Safely resolve a path within the base directory
fn safe_resolve_path(user_path: &str, base_dir: &Path) -> Result<PathBuf, String> {
    if user_path.is_empty() || user_path.len() > 4096 {
        return Err("Invalid path: path must be non-empty and under 4096 characters".to_string());
    }

    let normalized = PathBuf::from(user_path);
    let resolved = base_dir.join(normalized).canonicalize()
        .map_err(|e| format!("Failed to resolve path: {}", e))?;
    
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

/// Get script file extension based on interpreter
fn get_script_extension(interpreter: &str) -> &str {
    if interpreter.contains("python") {
        "py"
    } else if interpreter.contains("node") || interpreter.contains("bun") {
        "js"
    } else if interpreter.contains("bash") || interpreter.contains("sh") {
        "sh"
    } else if interpreter.contains("ruby") {
        "rb"
    } else {
        "txt"
    }
}

