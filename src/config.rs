use crate::cli::Args;
use crate::models::Reasoning;
use anyhow::{Context, Result};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::env;
use std::fs;
use std::path::PathBuf;

pub struct Config {
    pub api_key: String,
    pub api_endpoint: String,
    pub model: String,
    pub system_prompt: Option<String>,
    pub stream_timeout: u64,
    pub verbose: bool,
    pub reasoning: Option<Reasoning>,
    pub mcp_config: McpConfig,
    pub disable_tools: bool,
    pub local_tools_config: LocalToolsConfig,
    pub tools_enabled: bool,
}

#[derive(Debug, Clone, Default, Deserialize, Serialize)]
pub struct JsonConfig {
    #[serde(default)]
    pub api: ApiConfig,
    #[serde(default)]
    pub model: ModelConfig,
    #[serde(default)]
    pub session: SessionConfig,
    #[serde(default)]
    pub reasoning: ReasoningConfig,
    #[serde(default)]
    pub mcp: McpConfig,
    #[serde(default)]
    pub tools: ToolsConfig,
    #[serde(default)]
    pub local_tools: LocalToolsConfig,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct ApiConfig {
    #[serde(default)]
    pub endpoint: Option<String>,
    #[serde(default)]
    pub stream_timeout: Option<u64>,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct ModelConfig {
    #[serde(default)]
    pub default_model: Option<String>,
    #[serde(default)]
    pub system_prompt: Option<String>,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct SessionConfig {
    #[serde(default)]
    pub verbose: Option<bool>,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct ReasoningConfig {
    #[serde(default)]
    pub enabled: Option<bool>,
    #[serde(default)]
    pub effort: Option<String>,
    #[serde(default)]
    pub max_tokens: Option<u32>,
    #[serde(default)]
    pub exclude: Option<bool>,
}

#[derive(Debug, Clone, Default, Deserialize, Serialize)]
pub struct McpConfig {
    #[serde(default)]
    pub disable_tools: Option<bool>,
    #[serde(default)]
    pub settings: McpSettings,
    #[serde(default)]
    pub servers: Vec<ServerConfig>,
    #[serde(default)]
    pub tool_selection: ToolSelection,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct McpSettings {
    #[serde(default = "default_auto_detect")]
    pub auto_detect: bool,
    #[serde(default = "default_timeout")]
    pub timeout: u64,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct ServerConfig {
    pub name: String,
    pub command: String,
    pub args: Vec<String>,
    #[serde(default)]
    pub auto_activate_keywords: Vec<String>,
    #[serde(default)]
    pub description: String,
    #[serde(default)]
    pub env: HashMap<String, String>,
    #[serde(default = "default_enabled")]
    pub enabled: bool,
    #[serde(default = "default_transport")]
    pub transport: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub url: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub headers: Option<HashMap<String, String>>,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct ToolSelection {
    #[serde(default = "default_max_servers")]
    pub max_servers: usize,
    #[serde(default = "default_min_match_score")]
    pub min_match_score: f64,
    #[serde(default)]
    pub prompt_before_activation: bool,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct ToolsConfig {
    #[serde(default = "default_tools_enabled")]
    pub enabled: bool,
}

#[derive(Debug, Clone, Default, Deserialize, Serialize)]
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

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct LocalToolConfig {
    pub name: String,
    #[serde(default = "default_enabled")]
    pub enabled: bool,
    #[serde(default)]
    pub settings: serde_json::Value,
}

impl Default for ApiConfig {
    fn default() -> Self {
        Self {
            endpoint: None,
            stream_timeout: None,
        }
    }
}

impl Default for ModelConfig {
    fn default() -> Self {
        Self {
            default_model: None,
            system_prompt: None,
        }
    }
}

impl Default for SessionConfig {
    fn default() -> Self {
        Self { verbose: None }
    }
}

impl Default for ReasoningConfig {
    fn default() -> Self {
        Self {
            enabled: None,
            effort: None,
            max_tokens: None,
            exclude: None,
        }
    }
}

impl Default for McpSettings {
    fn default() -> Self {
        Self {
            auto_detect: default_auto_detect(),
            timeout: default_timeout(),
        }
    }
}

impl Default for ToolSelection {
    fn default() -> Self {
        Self {
            max_servers: default_max_servers(),
            min_match_score: default_min_match_score(),
            prompt_before_activation: false,
        }
    }
}

fn default_auto_detect() -> bool {
    true
}
fn default_timeout() -> u64 {
    30
}
fn default_enabled() -> bool {
    true
}
fn default_max_servers() -> usize {
    3
}
fn default_min_match_score() -> f64 {
    0.3
}
fn default_transport() -> String {
    "stdio".to_string()
}
fn default_tools_enabled() -> bool {
    true
}
fn default_local_tools_enabled() -> bool {
    true
}
fn default_max_file_size_mb() -> u64 {
    10
}

impl Default for ToolsConfig {
    fn default() -> Self {
        Self {
            enabled: default_tools_enabled(),
        }
    }
}

impl Config {
    pub fn from_env_and_args(args: &Args) -> Result<Self, String> {
        // Load JSON configuration first
        let json_config = JsonConfig::load().unwrap_or_default();

        // Get API key (still required from env var for security)
        let api_key = env::var("OPENROUTER_API_KEY")
            .map_err(|_| "OPENROUTER_API_KEY environment variable not set")?;

        // Get API endpoint: CLI args > env var > JSON config > default
        let api_endpoint = args
            .api_endpoint
            .clone()
            .or_else(|| env::var("AI_API_ENDPOINT").ok())
            .or(json_config.api.endpoint.clone())
            .map(|endpoint| {
                // If the endpoint doesn't end with /chat/completions, append it
                if endpoint.ends_with("/chat/completions") {
                    endpoint
                } else if endpoint.ends_with("/v1") {
                    format!("{}/chat/completions", endpoint)
                } else if endpoint.ends_with("/v1/") {
                    format!("{}chat/completions", endpoint)
                } else {
                    // Assume it's a base URL without /v1
                    format!("{}/v1/chat/completions", endpoint.trim_end_matches('/'))
                }
            })
            .unwrap_or_else(|| "https://openrouter.ai/api/v1/chat/completions".to_string());

        // Get model: env var > JSON config > default
        let model = env::var("AI_MODEL")
            .ok()
            .or(json_config.model.default_model.clone())
            .unwrap_or_else(|| "openai/gpt-5".to_string());

        // Get system prompt: env var > JSON config
        let system_prompt = env::var("AI_SYSTEM_PROMPT")
            .ok()
            .or(json_config.model.system_prompt.clone());

        // Get stream timeout: env var > JSON config > default
        let stream_timeout = env::var("AI_STREAM_TIMEOUT")
            .ok()
            .and_then(|s| s.parse::<u64>().ok())
            .or(json_config.api.stream_timeout)
            .unwrap_or(30);

        // Get verbose flag: env var > JSON config > default
        let verbose = env::var("AI_VERBOSE")
            .ok()
            .map(|v| v == "true")
            .or(json_config.session.verbose)
            .unwrap_or(false);

        // Get tools_enabled: CLI arg (--no-tools) > env var > JSON config > default
        // If --no-tools is set, disable all tools regardless of other settings
        let tools_enabled = if args.no_tools {
            false
        } else {
            env::var("AI_TOOLS_ENABLED")
                .ok()
                .map(|v| matches!(v.to_lowercase().as_str(), "true" | "1" | "yes"))
                .unwrap_or(true)
                && json_config.tools.enabled
        };

        // Get disable_tools flag: CLI arg (--no-tools or --no-mcp-tools) > env var > JSON config > default
        // This is kept for backward compatibility but now only affects MCP tools
        let disable_tools = args.no_tools
            || args.no_mcp_tools
            || env::var("AI_DISABLE_TOOLS")
                .ok()
                .map(|v| matches!(v.to_lowercase().as_str(), "true" | "1" | "yes"))
                .or(json_config.mcp.disable_tools)
                .unwrap_or(false);

        // Get local_tools config
        let local_tools_config = json_config.local_tools;

        // Build reasoning configuration from CLI args, env vars, and JSON config
        let reasoning = Self::build_reasoning_config(args, &json_config.reasoning);

        // Use MCP configuration from JSON
        let mcp_config = json_config.mcp;

        Ok(Config {
            api_key,
            api_endpoint,
            model,
            system_prompt,
            stream_timeout,
            verbose,
            reasoning,
            mcp_config,
            disable_tools,
            local_tools_config,
            tools_enabled,
        })
    }

    fn build_reasoning_config(args: &Args, json_reasoning: &ReasoningConfig) -> Option<Reasoning> {
        // Environment variables
        let env_reasoning_enabled =
            env::var("AI_REASONING_ENABLED")
                .ok()
                .and_then(|v| match v.to_lowercase().as_str() {
                    "true" | "1" | "yes" => Some(true),
                    _ => None,
                });

        let env_reasoning_effort = env::var("AI_REASONING_EFFORT")
            .ok()
            .filter(|e| ["high", "medium", "low"].contains(&e.to_lowercase().as_str()))
            .map(|e| e.to_lowercase());

        let env_reasoning_max_tokens = env::var("AI_REASONING_MAX_TOKENS")
            .ok()
            .and_then(|s| s.parse::<u32>().ok());

        let env_reasoning_exclude =
            env::var("AI_REASONING_EXCLUDE")
                .ok()
                .and_then(|v| match v.to_lowercase().as_str() {
                    "true" | "1" | "yes" => Some(true),
                    _ => None,
                });

        // Determine final values: CLI args > env vars > JSON config
        let final_reasoning_enabled = args.reasoning_enabled
            || env_reasoning_enabled.unwrap_or(false)
            || json_reasoning.enabled.unwrap_or(false);

        let final_reasoning_effort = args
            .reasoning_effort
            .clone()
            .or(env_reasoning_effort)
            .or(json_reasoning.effort.clone());

        let final_reasoning_max_tokens = args
            .reasoning_max_tokens
            .or(env_reasoning_max_tokens)
            .or(json_reasoning.max_tokens);

        let final_reasoning_exclude = args.reasoning_exclude
            || env_reasoning_exclude.unwrap_or(false)
            || json_reasoning.exclude.unwrap_or(false);

        if final_reasoning_enabled
            || final_reasoning_effort.is_some()
            || final_reasoning_max_tokens.is_some()
            || final_reasoning_exclude
        {
            Some(Reasoning {
                effort: final_reasoning_effort
                    .filter(|e| ["high", "medium", "low"].contains(&e.as_str())),
                max_tokens: final_reasoning_max_tokens,
                exclude: if final_reasoning_exclude {
                    Some(true)
                } else {
                    None
                },
                enabled: if final_reasoning_enabled {
                    Some(true)
                } else {
                    None
                },
            })
        } else {
            None
        }
    }

    pub fn get_current_date() -> String {
        chrono::Local::now().format("%A, %B %d, %Y").to_string()
    }
}

impl JsonConfig {
    pub fn load() -> Result<Self> {
        let config_paths = Self::get_config_paths();

        for path in config_paths {
            if path.exists() {
                let contents = fs::read_to_string(&path)
                    .with_context(|| format!("Failed to read config file: {}", path.display()))?;

                // Try YAML first, then fall back to JSON for backward compatibility
                let config: JsonConfig = if path.extension().and_then(|s| s.to_str())
                    == Some("yaml")
                    || path.extension().and_then(|s| s.to_str()) == Some("yml")
                {
                    serde_yaml::from_str(&contents).with_context(|| {
                        format!("Failed to parse YAML config file: {}", path.display())
                    })?
                } else {
                    // Try JSON for backward compatibility
                    serde_json::from_str(&contents).with_context(|| {
                        format!("Failed to parse JSON config file: {}", path.display())
                    })?
                };

                return Ok(config);
            }
        }

        // No config file found, return default
        Ok(JsonConfig::default())
    }

    pub fn get_config_paths() -> Vec<PathBuf> {
        let mut paths = Vec::new();

        // 1. Current directory (highest priority - local override)
        paths.push(PathBuf::from(".cmd2ai.yaml"));
        paths.push(PathBuf::from(".cmd2ai.yml"));
        paths.push(PathBuf::from(".cmd2ai.json")); // Backward compatibility

        // 2. User's config directory (global config)
        if let Some(home_dir) = dirs::home_dir() {
            let config_dir = home_dir.join(".config").join("cmd2ai");
            paths.push(config_dir.join("cmd2ai.yaml"));
            paths.push(config_dir.join("cmd2ai.yml"));
            paths.push(config_dir.join("cmd2ai.json")); // Backward compatibility
        }

        paths
    }
}

impl McpConfig {
    pub fn get_enabled_servers(&self) -> Vec<&ServerConfig> {
        self.servers.iter().filter(|s| s.enabled).collect()
    }

    /// Score a server based on keyword matches in the prompt
    /// Returns a score between 0.0 and 1.0
    pub fn score_server_for_prompt(server: &ServerConfig, prompt: &str) -> f64 {
        if server.auto_activate_keywords.is_empty() {
            // If no keywords defined, give a default low score
            return 0.1;
        }

        let prompt_lower = prompt.to_lowercase();
        let mut matches = 0;
        let total_keywords = server.auto_activate_keywords.len();

        for keyword in &server.auto_activate_keywords {
            let keyword_lower = keyword.to_lowercase();
            // Check for exact word match or substring match
            if prompt_lower.contains(&keyword_lower) {
                matches += 1;
            }
        }

        // Return normalized score (0.0 to 1.0)
        if total_keywords == 0 {
            0.0
        } else {
            matches as f64 / total_keywords as f64
        }
    }

    /// Select servers based on keyword matching
    /// Returns top N servers above min_match_score threshold
    pub fn select_servers_by_keywords(&self, prompt: &str) -> Vec<&ServerConfig> {
        let enabled_servers = self.get_enabled_servers();

        // Score each server
        let mut scored: Vec<(f64, &ServerConfig)> = enabled_servers
            .into_iter()
            .map(|server| (Self::score_server_for_prompt(server, prompt), server))
            .collect();

        // Sort by score descending
        scored.sort_by(|a, b| b.0.partial_cmp(&a.0).unwrap_or(std::cmp::Ordering::Equal));

        // Filter by min_match_score and take top max_servers
        let min_score = self.tool_selection.min_match_score;
        let max_servers = self.tool_selection.max_servers;

        scored
            .into_iter()
            .filter(|(score, _)| *score >= min_score)
            .take(max_servers)
            .map(|(_, server)| server)
            .collect()
    }

    pub fn expand_env_vars(env: &HashMap<String, String>) -> HashMap<String, String> {
        let mut expanded = HashMap::new();

        for (key, value) in env {
            let expanded_value = Self::expand_env_var_in_string(value);
            expanded.insert(key.clone(), expanded_value);
        }

        expanded
    }

    pub fn expand_env_var_in_string(value: &str) -> String {
        let mut result = value.to_string();
        let re = regex::Regex::new(r"\$\{([^}]+)\}").unwrap();

        for cap in re.captures_iter(value) {
            let var_name = &cap[1];
            let replacement = env::var(var_name).unwrap_or_else(|_| format!("${{{}}}", var_name));
            result = result.replace(&cap[0], &replacement);
        }

        result
    }

    pub fn expand_args(args: &[String]) -> Vec<String> {
        args.iter()
            .map(|arg| Self::expand_env_var_in_string(arg))
            .collect()
    }
}
