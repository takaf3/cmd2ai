mod api;
mod cli;
mod config;
mod error;
mod local_tools;
mod models;
mod orchestrator;
mod session;
mod ui;

use clap::Parser;
use colored::*;
use std::process;

use cli::Args;
use config::Config;
use local_tools::LocalSettings;
use local_tools::LocalToolRegistry;
use models::Message;
use orchestrator::{run, OrchestratorContext};
use session::{
    clear_all_sessions, create_new_session, find_recent_session, save_session,
    trim_conversation_history,
};

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let args = Args::parse();

    // Handle --clear option
    if args.clear_history {
        match clear_all_sessions() {
            Ok(_) => {
                println!("{}", "All conversation history cleared.".green());
                return Ok(());
            }
            Err(e) => {
                eprintln!("{}", format!("Error clearing history: {}", e).red());
                process::exit(1);
            }
        }
    }

    // Handle --config-init option
    if args.config_init {
        let example_config = include_str!("../config.example.yaml");
        let config_path = std::path::PathBuf::from(".cmd2ai.yaml");

        if config_path.exists() {
            eprintln!(
                "{} Config file already exists at .cmd2ai.yaml",
                "Error:".red()
            );
            eprintln!("Use a different path or remove the existing file.");
            process::exit(1);
        }

        match std::fs::write(&config_path, example_config) {
            Ok(_) => {
                println!("{}", "Config file created at .cmd2ai.yaml".green());
                println!("Edit this file to configure your local tools.");
                println!("YAML format supports comments for better documentation!");
                return Ok(());
            }
            Err(e) => {
                eprintln!("{} Failed to create config file: {}", "Error:".red(), e);
                process::exit(1);
            }
        }
    }

    if args.command.is_empty() {
        print_usage();
        process::exit(1);
    }

    let command = args.command.join(" ");

    // Load configuration
    let config = match Config::from_env_and_args(&args) {
        Ok(config) => config,
        Err(e) => {
            eprintln!("{} {}", "Error:".red(), e);
            process::exit(1);
        }
    };

    let _final_model = config.model.clone();

    // Load or create session
    let mut session = if args.new_conversation {
        create_new_session()
    } else {
        let existing_session = find_recent_session();

        if args.force_continue && existing_session.is_some() {
            existing_session.unwrap()
        } else {
            existing_session.unwrap_or_else(create_new_session)
        }
    };

    // Build messages array
    let mut messages = session.messages.clone();

    // Add system message if this is a new conversation or no system message exists
    if messages.is_empty() || messages.first().map(|m| &m.role) != Some(&"system".to_string()) {
        let date_prompt = format!("Today's date is {}.", Config::get_current_date());
        let system_content = if let Some(prompt) = &config.system_prompt {
            format!("{}\n\n{}", date_prompt, prompt)
        } else {
            date_prompt
        };

        messages.insert(
            0,
            Message {
                role: "system".to_string(),
                content: Some(system_content),
                tool_calls: None,
                tool_call_id: None,
            },
        );
    }

    // Add user message
    messages.push(Message {
        role: "user".to_string(),
        content: Some(command.clone()),
        tool_calls: None,
        tool_call_id: None,
    });

    // Trim history if needed
    trim_conversation_history(&mut messages);

    // Log reasoning configuration before moving it
    if config.verbose && config.reasoning.is_some() {
        eprintln!("{}", "[AI] Reasoning: enabled".dimmed());
        if let Some(ref reasoning) = config.reasoning {
            if let Some(ref effort) = reasoning.effort {
                eprintln!("{}", format!("[AI] Reasoning effort: {}", effort).dimmed());
            }
            if let Some(max_tokens) = reasoning.max_tokens {
                eprintln!(
                    "{}",
                    format!("[AI] Reasoning max tokens: {}", max_tokens).dimmed()
                );
            }
            if reasoning.exclude == Some(true) {
                eprintln!("{}", "[AI] Reasoning output: excluded".dimmed());
            }
        }
    }

    // Get available tools unless explicitly disabled
    let local_tools_enabled =
        config.tools_enabled && config.local_tools_config.enabled && !args.no_tools;

    // Create local tools registry if enabled
    let local_tools_registry = if local_tools_enabled {
        let settings = LocalSettings::from_config(&config.local_tools_config, config.verbose);
        Some(LocalToolRegistry::new(&config.local_tools_config, settings))
    } else {
        None
    };

    // Create orchestrator context
    let context = OrchestratorContext {
        config,
        args,
        local_tools_registry,
    };

    // Run orchestrator (pass mutable reference so it can modify messages with tool calls)
    let assistant_response = match run(context, &mut messages).await {
        Ok(response) => response,
        Err(e) => {
            eprintln!("{} {}", "Error:".red(), e);
            process::exit(1);
        }
    };

    // Save session with assistant's response
    if !assistant_response.is_empty() {
        session.messages = messages;
        session.messages.push(Message {
            role: "assistant".to_string(),
            content: Some(assistant_response),
            tool_calls: None,
            tool_call_id: None,
        });
        session.last_updated = chrono::Local::now();

        if let Err(e) = save_session(&session) {
            // Note: config is moved into context, so we can't access verbose here
            // This is acceptable as session save errors are non-critical
                eprintln!(
                    "{}",
                    format!("[AI] Warning: Failed to save session: {}", e).dimmed()
                );
        }
    }

    Ok(())
}

fn print_usage() {
    eprintln!("{}", "Usage: ai [OPTIONS] <command>".red());
    eprintln!(
        "{}",
        "  -n, --new                  Start a new conversation".dimmed()
    );
    eprintln!(
        "{}",
        "  -c, --continue             Continue previous conversation even if expired".dimmed()
    );
    eprintln!(
        "{}",
        "      --clear                Clear all conversation history".dimmed()
    );
    eprintln!(
        "{}",
        "      --reasoning-effort     Set reasoning effort level (high, medium, low)".dimmed()
    );
    eprintln!(
        "{}",
        "      --reasoning-max-tokens Set maximum tokens for reasoning".dimmed()
    );
    eprintln!(
        "{}",
        "      --reasoning-exclude    Use reasoning but exclude from response".dimmed()
    );
    eprintln!(
        "{}",
        "      --reasoning-enabled    Enable reasoning with default parameters".dimmed()
    );
    eprintln!(
        "{}",
        "      --no-tools             Disable all tools for this query".dimmed()
    );
    eprintln!(
        "{}",
        "      --config-init          Initialize a config file with example local tools".dimmed()
    );
    eprintln!(
        "{}",
        "      --api-endpoint         Custom API base URL (e.g., http://localhost:11434/v1)"
            .dimmed()
    );
    eprintln!("{}", "  -h, --help                 Print help".dimmed());
}
