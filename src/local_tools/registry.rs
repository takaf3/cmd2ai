use crate::config::LocalToolsConfig;
use colored::Colorize;
use jsonschema::{Draft, JSONSchema};
use serde_json::{json, Value};
use std::collections::HashMap;
use std::future::Future;
use std::path::PathBuf;
use std::pin::Pin;

use super::builtins;
use super::dynamic;

#[derive(Debug, Clone)]
pub struct LocalSettings {
    pub base_dir: PathBuf,
    pub max_file_size_bytes: u64,
    pub verbose: bool,
}

impl LocalSettings {
    pub fn from_config(config: &LocalToolsConfig, verbose: bool) -> Self {
        let base_dir = config
            .base_dir
            .as_ref()
            .map(|s| {
                // Expand environment variables
                crate::config::expand_env_var_in_string(s)
            })
            .and_then(|s| {
                if s.is_empty() {
                    None
                } else {
                    Some(PathBuf::from(s))
                }
            })
            .or_else(|| dirs::home_dir())
            .unwrap_or_else(|| PathBuf::from("."));

        let max_file_size_bytes = config.max_file_size_mb * 1024 * 1024;

        Self {
            base_dir,
            max_file_size_bytes,
            verbose,
        }
    }
}

/// Type alias for tool handler functions
pub type ToolHandler = Box<
    dyn for<'a> Fn(
            &'a Value,
            &'a LocalSettings,
        )
            -> Pin<Box<dyn Future<Output = Result<String, String>> + Send + 'a>>
        + Send
        + Sync,
>;

pub struct LocalTool {
    pub name: String,
    pub description: String,
    pub input_schema: Value,
    pub handler: ToolHandler,
}

pub struct LocalToolRegistry {
    tools: HashMap<String, LocalTool>,
    settings: LocalSettings,
}

impl LocalToolRegistry {
    pub fn new(config: &LocalToolsConfig, settings: LocalSettings) -> Self {
        let mut registry = Self {
            tools: HashMap::new(),
            settings,
        };

        // Register built-in tools
        registry.register_builtin_tools(config);

        // Register dynamic tools from config
        registry.register_dynamic_tools(config);

        registry
    }

    fn register_builtin_tools(&mut self, config: &LocalToolsConfig) {
        // Check if each tool is enabled in config
        let is_enabled = |name: &str| -> bool {
            config
                .tools
                .iter()
                .find(|t| t.name == name)
                .map(|t| t.enabled)
                .unwrap_or(true) // Default to enabled if not specified
        };

        // read_file tool
        if is_enabled("read_file") {
            if self.settings.verbose {
                eprintln!("{}", "[tools] Registering built-in tool: read_file".dimmed());
            }
            self.tools.insert(
                "read_file".to_string(),
                LocalTool {
                    name: "read_file".to_string(),
                    description: "Read and return the contents of a file. Limited to files within the base directory and under the size limit.".to_string(),
                    input_schema: json!({
                        "type": "object",
                        "properties": {
                            "path": {
                                "type": "string",
                                "description": "Path to the file to read (relative to base directory)"
                            }
                        },
                        "required": ["path"],
                        "additionalProperties": false
                    }),
                    handler: Box::new(|args, settings| {
                        let args = args.clone();
                        let settings = settings.clone();
                        Box::pin(async move {
                            builtins::handle_read_file(&args, &settings)
                        })
                    }),
                },
            );
        }

    }

    fn register_dynamic_tools(&mut self, config: &LocalToolsConfig) {
        for tool_config in &config.tools {
            // Skip if not enabled
            if !tool_config.enabled {
                if self.settings.verbose {
                    eprintln!(
                        "{}",
                        format!("[tools] Skipping disabled tool: {}", tool_config.name).dimmed()
                    );
                }
                continue;
            }

            // Skip if no type field (built-in tool, already registered)
            if tool_config.r#type.is_none() {
                continue;
            }

            // Skip if tool with same name already exists (built-in takes precedence)
            if self.tools.contains_key(&tool_config.name) {
                if self.settings.verbose {
                    eprintln!(
                        "{}",
                        format!(
                            "[tools] Skipping tool '{}' (built-in takes precedence)",
                            tool_config.name
                        )
                        .dimmed()
                    );
                }
                continue;
            }

            // Create dynamic tool
            match dynamic::create_dynamic_tool(tool_config, &self.settings) {
                Ok(tool) => {
                    if self.settings.verbose {
                        eprintln!(
                            "{}",
                            format!("[tools] Registered dynamic tool: {}", tool_config.name)
                                .dimmed()
                        );
                    }
                    self.tools.insert(tool_config.name.clone(), tool);
                }
                Err(e) => {
                    // Log error but don't fail - just skip this tool
                    eprintln!(
                        "Warning: Failed to register dynamic tool '{}': {}",
                        tool_config.name, e
                    );
                }
            }
        }
    }

    pub fn get(&self, name: &str) -> Option<&LocalTool> {
        self.tools.get(name)
    }

    pub fn list(&self) -> Vec<&LocalTool> {
        self.tools.values().collect()
    }

    pub fn settings(&self) -> &LocalSettings {
        &self.settings
    }

    pub fn validate_arguments(&self, tool_name: &str, arguments: &Value) -> Result<(), String> {
        let tool = self
            .tools
            .get(tool_name)
            .ok_or_else(|| format!("Tool '{}' not found", tool_name))?;

        if self.settings.verbose {
            let args_str = serde_json::to_string(arguments)
                .unwrap_or_else(|_| "<invalid json>".to_string());
            let truncated = if args_str.len() > 200 {
                format!("{}...", &args_str[..200])
            } else {
                args_str
            };
            eprintln!(
                "{}",
                format!("[tools] Validating arguments for '{}': {}", tool_name, truncated)
                    .dimmed()
            );
        }

        // Compile the JSON schema
        let schema = JSONSchema::options()
            .with_draft(Draft::Draft7)
            .compile(&tool.input_schema)
            .map_err(|e| format!("Invalid tool schema: {}", e))?;

        // Validate arguments against schema
        if let Err(errors) = schema.validate(arguments) {
            let error_messages: Vec<String> = errors
                .map(|e| format!("{}: {}", e.instance_path, e.to_string()))
                .collect();
            let error_msg = error_messages.join("; ");
            if self.settings.verbose {
                eprintln!(
                    "{}",
                    format!("[tools] Validation failed for '{}': {}", tool_name, error_msg)
                        .dimmed()
                );
            }
            return Err(error_msg);
        }

        if self.settings.verbose {
            eprintln!(
                "{}",
                format!("[tools] Validation passed for '{}'", tool_name).dimmed()
            );
        }

        Ok(())
    }
}
