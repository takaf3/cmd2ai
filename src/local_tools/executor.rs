use crate::config::{expand_env_var_in_string, expand_env_vars, LocalToolConfig, TemplateValidation};
use colored::Colorize;
use regex::Regex;
use serde_json::Value;
use std::fs;
use std::process::Stdio;
use std::time::{Duration, Instant};
use tokio::io::AsyncWriteExt;
use tokio::process::Command;
use tokio::time::timeout;

use super::paths::{canonicalize_within_base_dir, is_option_like, safe_resolve_path};
use super::registry::LocalSettings;

/// Execute a dynamic tool (script or command)
pub async fn execute_dynamic_tool(
    tool_config: &LocalToolConfig,
    arguments: &Value,
    settings: &LocalSettings,
) -> Result<String, String> {
    let tool_type = tool_config.r#type.as_deref().ok_or_else(|| {
        format!(
            "Tool '{}' is missing 'type' field (must be 'script' or 'command')",
            tool_config.name
        )
    })?;

    match tool_type {
        "script" => execute_script(tool_config, arguments, settings).await,
        "command" => execute_command(tool_config, arguments, settings).await,
        _ => Err(format!(
            "Unknown tool type '{}' for tool '{}'",
            tool_type, tool_config.name
        )),
    }
}

async fn execute_script(
    tool_config: &LocalToolConfig,
    arguments: &Value,
    settings: &LocalSettings,
) -> Result<String, String> {
    let start_time = Instant::now();
    let interpreter = tool_config.interpreter.as_ref().ok_or_else(|| {
        format!(
            "Tool '{}' (type: script) requires 'interpreter' field",
            tool_config.name
        )
    })?;

    // Determine script source: inline or file path
    let script_path = if let Some(ref inline_script) = tool_config.script {
        // Write inline script to temporary file
        let temp_dir = settings.base_dir.join(".cmd2ai-tools").join("tmp");
        fs::create_dir_all(&temp_dir)
            .map_err(|e| format!("Failed to create temp directory: {}", e))?;

        let temp_file = temp_dir.join(format!(
            "{}.{}",
            tool_config.name.replace('/', "_"),
            get_script_extension(interpreter)
        ));

        fs::write(&temp_file, inline_script)
            .map_err(|e| format!("Failed to write script file: {}", e))?;

        // Set executable permissions (Unix-like systems)
        #[cfg(unix)]
        {
            use std::os::unix::fs::PermissionsExt;
            let mut perms = fs::metadata(&temp_file)
                .map_err(|e| format!("Failed to get file metadata: {}", e))?
                .permissions();
            perms.set_mode(0o755);
            fs::set_permissions(&temp_file, perms)
                .map_err(|e| format!("Failed to set script permissions: {}", e))?;
        }

        if settings.verbose {
            eprintln!(
                "{}",
                format!("[tools] Created inline script: {}", temp_file.display()).dimmed()
            );
        }
        temp_file
    } else if let Some(ref script_path_str) = tool_config.script_path {
        // Resolve script path relative to base_dir
        let resolved = safe_resolve_path(script_path_str, &settings.base_dir)?;
        if settings.verbose {
            eprintln!(
                "{}",
                format!(
                    "[tools] Resolved script_path: {} -> {}",
                    script_path_str,
                    resolved.display()
                )
                .dimmed()
            );
        }
        resolved
    } else {
        return Err(format!(
            "Tool '{}' (type: script) requires either 'script' (inline) or 'script_path' field",
            tool_config.name
        ));
    };

    // Resolve working directory
    let working_dir = if let Some(ref wd) = tool_config.working_dir {
        let resolved = safe_resolve_path(wd, &settings.base_dir)?;
        if settings.verbose {
            eprintln!(
                "{}",
                format!(
                    "[tools] Resolved working_dir: {} -> {}",
                    wd,
                    resolved.display()
                )
                .dimmed()
            );
        }
        resolved
    } else {
        settings.base_dir.clone()
    };

    // Expand environment variables
    let env_vars = expand_env_vars(&tool_config.env);

    // Log pre-execution info
    if settings.verbose {
        let env_keys: Vec<String> = env_vars.keys().cloned().collect();
        let env_info = if env_keys.is_empty() {
            String::new()
        } else {
            format!(", env={}", env_keys.join(","))
        };
        eprintln!(
            "{}",
            format!(
                "[tools] run: {} {} (cwd={}, timeout={}s{})",
                interpreter,
                script_path.display(),
                working_dir.display(),
                tool_config.timeout_secs,
                env_info
            )
            .dimmed()
        );
        let args_json = serde_json::to_string(arguments)
            .unwrap_or_else(|_| "<invalid>".to_string());
        let truncated = if args_json.len() > 100 {
            format!("{}...", &args_json[..100])
        } else {
            args_json
        };
        eprintln!(
            "{}",
            format!("[tools] stdin: {}", truncated).dimmed()
        );
    }

    // Prepare command
    let mut cmd = Command::new(interpreter);
    cmd.arg(&script_path)
        .current_dir(&working_dir)
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped());

    // Set environment variables
    for (key, value) in &env_vars {
        cmd.env(key, value);
    }

    // Spawn process
    let mut child = cmd
        .spawn()
        .map_err(|e| format!("Failed to spawn script process: {}", e))?;

    // Write arguments as JSON to stdin
    let args_json = serde_json::to_string(arguments)
        .map_err(|e| format!("Failed to serialize arguments: {}", e))?;

    if let Some(mut stdin) = child.stdin.take() {
        stdin
            .write_all(args_json.as_bytes())
            .await
            .map_err(|e| format!("Failed to write to stdin: {}", e))?;
        stdin
            .flush()
            .await
            .map_err(|e| format!("Failed to flush stdin: {}", e))?;
    }

    // Wait for process with timeout
    let timeout_duration = Duration::from_secs(tool_config.timeout_secs);
    let output = timeout(timeout_duration, child.wait_with_output())
        .await
        .map_err(|_| {
            format!(
                "Script execution timed out after {} seconds",
                tool_config.timeout_secs
            )
        })?
        .map_err(|e| format!("Failed to wait for process: {}", e))?;

    let duration = start_time.elapsed();

    // Log post-execution info
    if settings.verbose {
        let exit_code = output.status.code().unwrap_or(-1);
        let stderr_preview = if !output.stderr.is_empty() {
            let stderr_str = String::from_utf8_lossy(&output.stderr);
            let truncated = if stderr_str.len() > 200 {
                format!("{}...", &stderr_str[..200])
            } else {
                stderr_str.to_string()
            };
            format!(", stderr={}", truncated)
        } else {
            String::new()
        };
        eprintln!(
            "{}",
            format!(
                "[tools] done: exit_code={}, duration={:.2}s, output_size={} bytes{}",
                exit_code,
                duration.as_secs_f64(),
                output.stdout.len(),
                stderr_preview
            )
            .dimmed()
        );
    }

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
    String::from_utf8(output.stdout).map_err(|e| format!("Script output is not valid UTF-8: {}", e))
}

async fn execute_command(
    tool_config: &LocalToolConfig,
    arguments: &Value,
    settings: &LocalSettings,
) -> Result<String, String> {
    let start_time = Instant::now();
    let command = tool_config.command.as_ref().ok_or_else(|| {
        format!(
            "Tool '{}' (type: command) requires 'command' field",
            tool_config.name
        )
    })?;

    // Resolve working directory
    let working_dir = if let Some(ref wd) = tool_config.working_dir {
        let resolved = safe_resolve_path(wd, &settings.base_dir)?;
        if settings.verbose {
            eprintln!(
                "{}",
                format!(
                    "[tools] Resolved working_dir: {} -> {}",
                    wd,
                    resolved.display()
                )
                .dimmed()
            );
        }
        resolved
    } else {
        settings.base_dir.clone()
    };

    // Expand environment variables
    let env_vars = expand_env_vars(&tool_config.env);

    // Template arguments: replace {{key}} with values from arguments JSON
    let env_expanded_args: Vec<String> = tool_config
        .args
        .iter()
        .map(|arg| expand_env_var_in_string(arg))
        .collect();
    let templated_args = template_args(
        &env_expanded_args,
        arguments,
        tool_config,
        settings,
    )?;

    // Log pre-execution info
    if settings.verbose {
        let args_display: Vec<String> = templated_args
            .iter()
            .map(|a| {
                if a.contains(' ') {
                    format!("\"{}\"", a)
                } else {
                    a.clone()
                }
            })
            .collect();
        let cmd_line = format!("{} {}", command, args_display.join(" "));
        let env_keys: Vec<String> = env_vars.keys().cloned().collect();
        let env_info = if env_keys.is_empty() {
            String::new()
        } else {
            format!(", env={}", env_keys.join(","))
        };
        eprintln!(
            "{}",
            format!(
                "[tools] run: {} (cwd={}, timeout={}s{})",
                cmd_line,
                working_dir.display(),
                tool_config.timeout_secs,
                env_info
            )
            .dimmed()
        );
        if tool_config.stdin_json {
            let args_json = serde_json::to_string(arguments)
                .unwrap_or_else(|_| "<invalid>".to_string());
            let truncated = if args_json.len() > 100 {
                format!("{}...", &args_json[..100])
            } else {
                args_json
            };
            eprintln!(
                "{}",
                format!("[tools] stdin: {}", truncated).dimmed()
            );
        }
    }

    // Prepare command
    let mut cmd = Command::new(command);
    cmd.args(&templated_args)
        .current_dir(&working_dir)
        .stdout(Stdio::piped())
        .stderr(Stdio::piped());

    // Set stdin based on stdin_json flag
    if tool_config.stdin_json {
        cmd.stdin(Stdio::piped());
    } else {
        cmd.stdin(Stdio::null());
    }

    // Set environment variables
    for (key, value) in &env_vars {
        cmd.env(key, value);
    }

    // Spawn process
    let mut child = cmd
        .spawn()
        .map_err(|e| format!("Failed to spawn command process: {}", e))?;

    // Write arguments as JSON to stdin (only if stdin_json is true)
    if tool_config.stdin_json {
        let args_json = serde_json::to_string(arguments)
            .map_err(|e| format!("Failed to serialize arguments: {}", e))?;

        if let Some(mut stdin) = child.stdin.take() {
            stdin
                .write_all(args_json.as_bytes())
                .await
                .map_err(|e| format!("Failed to write to stdin: {}", e))?;
            stdin
                .flush()
                .await
                .map_err(|e| format!("Failed to flush stdin: {}", e))?;
        }
    }

    // Wait for process with timeout
    let timeout_duration = Duration::from_secs(tool_config.timeout_secs);
    let output = timeout(timeout_duration, child.wait_with_output())
        .await
        .map_err(|_| {
            format!(
                "Command execution timed out after {} seconds",
                tool_config.timeout_secs
            )
        })?
        .map_err(|e| format!("Failed to wait for process: {}", e))?;

    let duration = start_time.elapsed();

    // Log post-execution info
    if settings.verbose {
        let exit_code = output.status.code().unwrap_or(-1);
        let stderr_preview = if !output.stderr.is_empty() {
            let stderr_str = String::from_utf8_lossy(&output.stderr);
            let truncated = if stderr_str.len() > 200 {
                format!("{}...", &stderr_str[..200])
            } else {
                stderr_str.to_string()
            };
            format!(", stderr={}", truncated)
        } else {
            String::new()
        };
        eprintln!(
            "{}",
            format!(
                "[tools] done: exit_code={}, duration={:.2}s, output_size={} bytes{}",
                exit_code,
                duration.as_secs_f64(),
                output.stdout.len(),
                stderr_preview
            )
            .dimmed()
        );
    }

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
    String::from_utf8(output.stdout)
        .map_err(|e| format!("Command output is not valid UTF-8: {}", e))
}

/// Template arguments: replace {{key}} with values from arguments JSON
/// This function validates and sanitizes templated values, especially paths,
/// to prevent argument injection and path traversal attacks.
fn template_args(
    args: &[String],
    arguments: &Value,
    tool_config: &LocalToolConfig,
    settings: &LocalSettings,
) -> Result<Vec<String>, String> {
    let re = Regex::new(r"\{\{([^}]+)\}\}").unwrap();
    let mut has_path_placeholders = false;
    let mut templated_args = Vec::new();
    let mut args_with_placeholders = Vec::new(); // Track which args had placeholders BEFORE substitution

    for (arg_idx, arg) in args.iter().enumerate() {
        let mut result = arg.clone();
        let mut had_placeholder = false;
        
        // Collect all matches with their byte positions first
        // This prevents cascading replacements where a replacement value
        // contains a placeholder pattern that gets replaced again
        let mut replacements: Vec<(usize, usize, String)> = Vec::new();
        
        for cap in re.captures_iter(arg) {
            had_placeholder = true;
            let key = &cap[1];
            let placeholder = &cap[0];
            let start = cap.get(0).unwrap().start();
            let end = cap.get(0).unwrap().end();
            
            // Get value from arguments JSON
            if let Some(value) = arguments.get(key) {
                let value_str = match value {
                    Value::String(s) => s.clone(),
                    Value::Number(n) => n.to_string(),
                    Value::Bool(b) => b.to_string(),
                    Value::Null => String::new(),
                    _ => serde_json::to_string(value).unwrap_or_else(|_| placeholder.to_string()),
                };
                
                // Determine validation policy for this key
                let validation = get_validation_policy(key, tool_config);
                
                // Validate and transform the value based on policy
                let validated_value = validate_and_transform_value(
                    key,
                    &value_str,
                    &validation,
                    tool_config,
                    settings,
                )?;
                
                if validation.kind == "path" {
                    has_path_placeholders = true;
                }
                
                replacements.push((start, end, validated_value));
            }
            // If key not found, leave placeholder as-is (validation should catch missing required fields)
        }
        
        // Replace from end to start to preserve positions
        replacements.sort_by(|a, b| b.0.cmp(&a.0));
        for (start, end, replacement) in replacements {
            result.replace_range(start..end, &replacement);
        }
        
        templated_args.push(result);
        if had_placeholder {
            args_with_placeholders.push(arg_idx);
        }
    }

    // Insert "--" before first templated argument if needed to prevent option injection
    let should_insert_double_dash = match tool_config.insert_double_dash {
        Some(true) => true,
        Some(false) => false,
        None => has_path_placeholders, // Auto-detect: insert if any path placeholders exist
    };

    if should_insert_double_dash {
        // Find the first argument that contained a templated value (before substitution)
        if let Some(&first_templated_idx) = args_with_placeholders.first() {
            let mut final_args = Vec::new();
            for (idx, arg) in templated_args.iter().enumerate() {
                if idx == first_templated_idx {
                    final_args.push("--".to_string());
                }
                final_args.push(arg.clone());
            }
            Ok(final_args)
        } else {
            // No templated arguments, so nothing to insert before
            Ok(templated_args)
        }
    } else {
        Ok(templated_args)
    }
}

/// Get validation policy for a template key
fn get_validation_policy(key: &str, tool_config: &LocalToolConfig) -> TemplateValidation {
    // Check if explicit validation is configured
    if let Some(ref validations) = tool_config.template_validations {
        if let Some(validation) = validations.get(key) {
            return validation.clone();
        }
    }
    
    // Heuristic: treat keys matching path pattern as paths
    let path_pattern = Regex::new(r"(?i)^(.*_)?path(s)?$").unwrap();
    if path_pattern.is_match(key) {
        TemplateValidation {
            kind: "path".to_string(),
            allow_patterns: None,
            deny_patterns: None,
            allow_absolute: false,
        }
    } else {
        // Default to string validation
        TemplateValidation {
            kind: "string".to_string(),
            allow_patterns: None,
            deny_patterns: None,
            allow_absolute: false,
        }
    }
}

/// Validate and transform a templated value based on its validation policy
fn validate_and_transform_value(
    key: &str,
    value: &str,
    validation: &TemplateValidation,
    tool_config: &LocalToolConfig,
    settings: &LocalSettings,
) -> Result<String, String> {
    match validation.kind.as_str() {
        "path" => {
            // Reject option-like values (unless explicitly allowed)
            if is_option_like(value) {
                return Err(format!(
                    "Invalid path argument '{}': value '{}' looks like a command-line option. \
                    Path arguments cannot start with '-'. If you need to pass options, \
                    configure template_validations for '{}' with allow_patterns.",
                    key, value, key
                ));
            }

            // If restrict_to_base_dir is enabled (default), validate path
            if tool_config.restrict_to_base_dir {
                // Check if absolute paths are allowed
                if value.starts_with('/') && !validation.allow_absolute {
                    return Err(format!(
                        "Invalid path argument '{}': absolute path '{}' is not allowed. \
                        Use a relative path instead, or set allow_absolute: true in template_validations.",
                        key, value
                    ));
                }

                // Validate and canonicalize the path
                let canonical_path = canonicalize_within_base_dir(value, &settings.base_dir)
                    .map_err(|e| format!("Invalid path argument '{}': {}", key, e))?;
                
                Ok(canonical_path)
            } else {
                // Path restriction disabled - just return as-is (not recommended)
                Ok(value.to_string())
            }
        }
        "string" | _ => {
            // Apply regex pattern validation if configured
            if let Some(ref allow_patterns) = validation.allow_patterns {
                let mut matched = false;
                for pattern in allow_patterns {
                    let re = Regex::new(pattern)
                        .map_err(|e| format!("Invalid allow_pattern regex '{}': {}", pattern, e))?;
                    if re.is_match(value) {
                        matched = true;
                        break;
                    }
                }
                if !matched {
                    return Err(format!(
                        "Invalid string argument '{}': value '{}' does not match any allow_pattern",
                        key, value
                    ));
                }
            }

            if let Some(ref deny_patterns) = validation.deny_patterns {
                for pattern in deny_patterns {
                    let re = Regex::new(pattern)
                        .map_err(|e| format!("Invalid deny_pattern regex '{}': {}", pattern, e))?;
                    if re.is_match(value) {
                        return Err(format!(
                            "Invalid string argument '{}': value '{}' matches deny_pattern '{}'",
                            key, value, pattern
                        ));
                    }
                }
            }

            Ok(value.to_string())
        }
    }
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
