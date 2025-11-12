use serde::{Deserialize, Serialize};
use std::collections::HashMap;

use crate::config::defaults::{
    default_allow_absolute, default_local_tools_enabled, default_max_file_size_mb,
    default_max_output_bytes, default_restrict_to_base_dir, default_tool_timeout,
    default_tools_enabled, default_validation_kind, is_default_allow_absolute,
    is_default_restrict_to_base_dir, is_default_stdin_json, default_stdin_json,
};

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct ToolsConfig {
    #[serde(default = "default_tools_enabled")]
    pub enabled: bool,
}

impl Default for ToolsConfig {
    fn default() -> Self {
        Self {
            enabled: default_tools_enabled(),
        }
    }
}

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct LocalToolsConfig {
    #[serde(default = "default_local_tools_enabled")]
    pub enabled: bool,
    #[serde(default)]
    pub base_dir: Option<String>,
    #[serde(default = "default_max_file_size_mb")]
    pub max_file_size_mb: u64,
    #[serde(default)]
    pub tools: Vec<LocalToolConfig>,
}

impl Default for LocalToolsConfig {
    fn default() -> Self {
        Self {
            enabled: default_local_tools_enabled(),
            base_dir: None,
            max_file_size_mb: default_max_file_size_mb(),
            tools: Vec::new(),
        }
    }
}

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct LocalToolConfig {
    pub name: String,
    #[serde(default = "default_local_tools_enabled")]
    pub enabled: bool,
    #[serde(default)]
    pub settings: serde_json::Value,

    // Dynamic tool fields (optional - only for custom tools)
    #[serde(default)]
    #[serde(skip_serializing_if = "Option::is_none")]
    pub r#type: Option<String>, // "script" or "command"

    #[serde(default)]
    #[serde(skip_serializing_if = "Option::is_none")]
    pub description: Option<String>,

    #[serde(default)]
    #[serde(skip_serializing_if = "Option::is_none")]
    pub input_schema: Option<serde_json::Value>,

    // Script-specific fields
    #[serde(default)]
    #[serde(skip_serializing_if = "Option::is_none")]
    pub interpreter: Option<String>, // e.g., "python3", "node", "bash"

    #[serde(default)]
    #[serde(skip_serializing_if = "Option::is_none")]
    pub script: Option<String>, // Inline script content

    #[serde(default)]
    #[serde(skip_serializing_if = "Option::is_none")]
    pub script_path: Option<String>, // Path to script file (relative to base_dir)

    // Command-specific fields
    #[serde(default)]
    #[serde(skip_serializing_if = "Option::is_none")]
    pub command: Option<String>,

    #[serde(default)]
    pub args: Vec<String>,

    // Common optional settings
    #[serde(default = "default_tool_timeout")]
    pub timeout_secs: u64,

    #[serde(default = "default_max_output_bytes")]
    pub max_output_bytes: u64,

    #[serde(default)]
    #[serde(skip_serializing_if = "Option::is_none")]
    pub working_dir: Option<String>, // Relative to base_dir

    #[serde(default)]
    pub env: HashMap<String, String>, // Environment variables (with ${VAR} expansion)

    // Command-specific: whether to send JSON arguments via stdin
    // Defaults to true for backward compatibility
    #[serde(default = "default_stdin_json")]
    #[serde(skip_serializing_if = "is_default_stdin_json")]
    pub stdin_json: bool,

    // Security settings for command tools with templated arguments
    #[serde(default = "default_restrict_to_base_dir")]
    #[serde(skip_serializing_if = "is_default_restrict_to_base_dir")]
    pub restrict_to_base_dir: bool,

    #[serde(default)]
    #[serde(skip_serializing_if = "Option::is_none")]
    pub insert_double_dash: Option<bool>, // None means auto-detect based on path placeholders

    #[serde(default)]
    #[serde(skip_serializing_if = "Option::is_none")]
    pub template_validations: Option<HashMap<String, TemplateValidation>>,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct TemplateValidation {
    #[serde(default = "default_validation_kind")]
    pub kind: String, // "path" | "string" | "number"

    #[serde(default)]
    #[serde(skip_serializing_if = "Option::is_none")]
    pub allow_patterns: Option<Vec<String>>, // Regex patterns to allow

    #[serde(default)]
    #[serde(skip_serializing_if = "Option::is_none")]
    pub deny_patterns: Option<Vec<String>>, // Regex patterns to deny

    #[serde(default = "default_allow_absolute")]
    #[serde(skip_serializing_if = "is_default_allow_absolute")]
    pub allow_absolute: bool, // Allow absolute paths (only for path kind)
}

