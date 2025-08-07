use crate::cli::Args;
use crate::models::Reasoning;
use anyhow::{Context, Result};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::env;
use std::fs;
use std::path::{Path, PathBuf};

pub struct Config {
    pub api_key: String,
    pub model: String,
    pub system_prompt: Option<String>,
    pub web_search_max_results: u32,
    pub stream_timeout: u64,
    pub verbose: bool,
    pub reasoning: Option<Reasoning>,
    pub mcp_config: McpConfig,
}

#[derive(Debug, Clone, Default, Deserialize, Serialize)]
pub struct McpConfig {
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

fn default_auto_detect() -> bool { true }
fn default_timeout() -> u64 { 30 }
fn default_enabled() -> bool { true }
fn default_max_servers() -> usize { 3 }
fn default_min_match_score() -> f64 { 0.3 }

impl Config {
    pub fn from_env_and_args(args: &Args) -> Result<Self, String> {
        // Get API key
        let api_key = env::var("OPENROUTER_API_KEY")
            .map_err(|_| "OPENROUTER_API_KEY environment variable not set")?;

        // Get model
        let model = env::var("AI_MODEL").unwrap_or_else(|_| "openai/gpt-4.1-mini".to_string());

        // Get system prompt
        let system_prompt = env::var("AI_SYSTEM_PROMPT").ok();

        // Get web search max results
        let web_search_max_results = env::var("AI_WEB_SEARCH_MAX_RESULTS")
            .ok()
            .and_then(|s| s.parse::<u32>().ok())
            .unwrap_or(5)
            .clamp(1, 10);

        // Get stream timeout
        let stream_timeout = env::var("AI_STREAM_TIMEOUT")
            .ok()
            .and_then(|s| s.parse::<u64>().ok())
            .unwrap_or(30);

        // Get verbose flag
        let verbose = env::var("AI_VERBOSE").unwrap_or_default() == "true";

        // Build reasoning configuration from command-line arguments and environment variables
        let reasoning = Self::build_reasoning_config(args);

        // Load MCP configuration
        let mcp_config = McpConfig::load().unwrap_or_default();

        Ok(Config {
            api_key,
            model,
            system_prompt,
            web_search_max_results,
            stream_timeout,
            verbose,
            reasoning,
            mcp_config,
        })
    }

    fn build_reasoning_config(args: &Args) -> Option<Reasoning> {
        // Environment variables
        let env_reasoning_enabled = env::var("AI_REASONING_ENABLED")
            .ok()
            .and_then(|v| match v.to_lowercase().as_str() {
                "true" | "1" | "yes" => Some(true),
                _ => None,
            })
            .unwrap_or(false);

        let env_reasoning_effort = env::var("AI_REASONING_EFFORT")
            .ok()
            .filter(|e| ["high", "medium", "low"].contains(&e.to_lowercase().as_str()))
            .map(|e| e.to_lowercase());

        let env_reasoning_max_tokens = env::var("AI_REASONING_MAX_TOKENS")
            .ok()
            .and_then(|s| s.parse::<u32>().ok());

        let env_reasoning_exclude = env::var("AI_REASONING_EXCLUDE")
            .ok()
            .and_then(|v| match v.to_lowercase().as_str() {
                "true" | "1" | "yes" => Some(true),
                _ => None,
            })
            .unwrap_or(false);

        // Determine final values with CLI args taking precedence
        let final_reasoning_enabled = args.reasoning_enabled || env_reasoning_enabled;
        let final_reasoning_effort = args.reasoning_effort.clone().or(env_reasoning_effort);
        let final_reasoning_max_tokens = args.reasoning_max_tokens.or(env_reasoning_max_tokens);
        let final_reasoning_exclude = args.reasoning_exclude || env_reasoning_exclude;

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

    pub fn get_search_date() -> String {
        chrono::Local::now().format("%B %d, %Y").to_string()
    }
}

impl McpConfig {
    pub fn load() -> Result<Self> {
        let config_paths = Self::get_config_paths();
        
        for path in config_paths {
            if path.exists() {
                let contents = fs::read_to_string(&path)
                    .with_context(|| format!("Failed to read config file: {}", path.display()))?;
                
                let config: McpConfig = serde_json::from_str(&contents)
                    .with_context(|| format!("Failed to parse config file: {}", path.display()))?;
                
                return Ok(config);
            }
        }
        
        // No config file found, return default
        Ok(McpConfig::default())
    }
    
    pub fn get_config_paths() -> Vec<PathBuf> {
        let mut paths = Vec::new();
        
        // 1. Current directory (highest priority - local override)
        paths.push(PathBuf::from(".cmd2ai.json"));
        
        // 2. User's config directory (global config)
        if let Some(home_dir) = dirs::home_dir() {
            paths.push(home_dir.join(".config").join("cmd2ai").join("cmd2ai.json"));
        }
        
        paths
    }
    
    pub fn save(&self, path: &Path) -> Result<()> {
        let contents = serde_json::to_string_pretty(self)
            .context("Failed to serialize config")?;
        
        if let Some(parent) = path.parent() {
            fs::create_dir_all(parent)
                .with_context(|| format!("Failed to create config directory: {}", parent.display()))?;
        }
        
        fs::write(path, contents)
            .with_context(|| format!("Failed to write config file: {}", path.display()))?;
        
        Ok(())
    }
    
    pub fn get_enabled_servers(&self) -> Vec<&ServerConfig> {
        self.servers
            .iter()
            .filter(|s| s.enabled)
            .collect()
    }
    
    pub fn detect_servers_for_query(&self, query: &str) -> Vec<&ServerConfig> {
        if !self.settings.auto_detect {
            return Vec::new();
        }
        
        let enabled_servers = self.get_enabled_servers();
        let query_lower = query.to_lowercase();
        let mut scored_servers: Vec<(&ServerConfig, f64)> = Vec::new();
        
        for server in enabled_servers {
            if server.auto_activate_keywords.is_empty() {
                continue;
            }
            
            let mut matches = 0;
            let mut total_keywords = 0;
            for keyword in &server.auto_activate_keywords {
                total_keywords += 1;
                // Check for word boundary matches to avoid false positives
                let keyword_lower = keyword.to_lowercase();
                if query_lower.contains(&keyword_lower) {
                    matches += 1;
                }
            }
            
            if matches > 0 {
                let score = matches as f64 / total_keywords as f64;
                if score >= self.tool_selection.min_match_score {
                    scored_servers.push((server, score));
                }
            }
        }
        
        // Sort by score (highest first) and take top N
        scored_servers.sort_by(|a, b| b.1.partial_cmp(&a.1).unwrap());
        scored_servers
            .into_iter()
            .take(self.tool_selection.max_servers)
            .map(|(server, _)| server)
            .collect()
    }
    
    pub fn expand_env_vars(env: &HashMap<String, String>) -> HashMap<String, String> {
        let mut expanded = HashMap::new();
        
        for (key, value) in env {
            let expanded_value = if value.starts_with("${") && value.ends_with("}") {
                let var_name = &value[2..value.len()-1];
                env::var(var_name).unwrap_or_else(|_| value.clone())
            } else {
                value.clone()
            };
            expanded.insert(key.clone(), expanded_value);
        }
        
        expanded
    }
}
