mod api;
mod defaults;
mod reasoning;
mod tools;
mod validation;

use crate::cli::Args;
use crate::models::Reasoning;
use anyhow::{Context, Result};
use serde::{Deserialize, Serialize};
use std::env;
use std::fs;
use std::path::PathBuf;

pub use api::ApiConfig;
pub use reasoning::ReasoningConfig;
pub use tools::{LocalToolConfig, LocalToolsConfig, TemplateValidation, ToolsConfig};
pub use validation::{expand_env_var_in_string, expand_env_vars};

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct SessionConfig {
    #[serde(default)]
    pub verbose: Option<bool>,
}

impl Default for SessionConfig {
    fn default() -> Self {
        Self { verbose: None }
    }
}

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct ModelConfig {
    #[serde(default)]
    pub default_model: Option<String>,
    #[serde(default)]
    pub system_prompt: Option<String>,
}

impl Default for ModelConfig {
    fn default() -> Self {
        Self {
            default_model: None,
            system_prompt: None,
        }
    }
}

pub struct Config {
    pub api_key: String,
    pub api_endpoint: String,
    pub model: String,
    pub system_prompt: Option<String>,
    pub stream_timeout: u64,
    pub verbose: bool,
    pub reasoning: Option<Reasoning>,
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
    pub tools: ToolsConfig,
    #[serde(default)]
    pub local_tools: LocalToolsConfig,
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
            // Check env var first - if set, use its value; otherwise fall through to JSON config
            match env::var("AI_TOOLS_ENABLED").ok() {
                Some(v) => matches!(v.to_lowercase().as_str(), "true" | "1" | "yes"),
                None => json_config.tools.enabled,
            }
        };

        // Get local_tools config
        let local_tools_config = json_config.local_tools;

        // Build reasoning configuration from CLI args, env vars, and JSON config
        let reasoning = Self::build_reasoning_config(args, &json_config.reasoning);

        Ok(Config {
            api_key,
            api_endpoint,
            model,
            system_prompt,
            stream_timeout,
            verbose,
            reasoning,
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

