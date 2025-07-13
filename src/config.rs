use crate::cli::Args;
use crate::models::Reasoning;
use std::env;

pub struct Config {
    pub api_key: String,
    pub model: String,
    pub system_prompt: Option<String>,
    pub web_search_max_results: u32,
    pub stream_timeout: u64,
    pub verbose: bool,
    pub reasoning: Option<Reasoning>,
}

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

        Ok(Config {
            api_key,
            model,
            system_prompt,
            web_search_max_results,
            stream_timeout,
            verbose,
            reasoning,
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
