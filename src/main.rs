mod cli;
mod config;
mod highlight;
mod mcp;
mod models;
mod session;

use clap::Parser;
use colored::*;
use futures::StreamExt;
use reqwest::header::{HeaderMap, HeaderValue, AUTHORIZATION, CONTENT_TYPE};
use std::io::{self, Write};
use std::process;
use tokio::time::{timeout, Duration};

use cli::Args;
use config::Config;
use highlight::CodeBuffer;
use mcp::McpClient;
use models::{Citation, Message, RequestBody, StreamResponse};
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
        let example_config = include_str!("../config.example.json");
        let config_path = std::path::PathBuf::from(".cmd2ai.json");
        
        if config_path.exists() {
            eprintln!("{} Config file already exists at .cmd2ai.json", "Error:".red());
            eprintln!("Use a different path or remove the existing file.");
            process::exit(1);
        }
        
        match std::fs::write(&config_path, example_config) {
            Ok(_) => {
                println!("{}", "Config file created at .cmd2ai.json".green());
                println!("Edit this file to configure your MCP servers.");
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

    // Initialize MCP client with servers from command line and/or config
    let mcp_client = {
        let mut servers_to_connect = Vec::new();
        
        // First, add servers from command line arguments
        for server_spec in &args.mcp_servers {
            let parts: Vec<&str> = server_spec.splitn(3, ':').collect();
            if parts.len() < 2 {
                eprintln!("{} Invalid MCP server format: {}", "Error:".red(), server_spec);
                eprintln!("Expected format: name:command or name:command:arg1,arg2,...");
                process::exit(1);
            }
            
            let server_name = parts[0].to_string();
            let command = parts[1].to_string();
            let args_str = if parts.len() > 2 { parts[2] } else { "" };
            let server_args: Vec<String> = if !args_str.is_empty() {
                args_str.split(',').map(|s| s.to_string()).collect()
            } else {
                vec![]
            };
            
            servers_to_connect.push((server_name, command, server_args, std::collections::HashMap::new()));
        }
        
        // Auto-detect servers from config - now the default behavior unless tools are disabled
        let should_use_tools = !config.disable_tools;  // Tools are on by default unless explicitly disabled
        
        if should_use_tools && servers_to_connect.is_empty() {
            if config.verbose {
                eprintln!("{}", format!("[AI] Loading MCP servers from config").dimmed());
                eprintln!("{}", format!("[AI] Available servers in config: {}", config.mcp_config.servers.len()).dimmed());
            }
            
            // Default behavior: Use ALL enabled servers, let AI decide which tools to use
            let servers_to_use = config.mcp_config.get_enabled_servers();
            
            if config.verbose {
                if servers_to_use.is_empty() {
                    eprintln!("{}", "[AI] No enabled MCP servers found".dimmed());
                } else {
                    eprintln!("{}", format!("[AI] Connecting to {} MCP server(s)", servers_to_use.len()).dimmed());
                }
            }
            
            for server in servers_to_use {
                if config.verbose {
                    eprintln!("{}", format!("[AI] - {} ({})", server.name, server.description).dimmed());
                }
                
                let env_vars = config::McpConfig::expand_env_vars(&server.env);
                servers_to_connect.push((
                    server.name.clone(),
                    server.command.clone(),
                    server.args.clone(),
                    env_vars,
                ));
            }
        }
        
        // Connect to servers if any were specified or detected
        if !servers_to_connect.is_empty() {
            let client = McpClient::new(config.verbose);
            
            for (server_name, command, server_args, env_vars) in servers_to_connect {
                if config.verbose {
                    println!("{}", format!("Connecting to MCP server '{}'...", server_name).cyan());
                }
                
                // Set environment variables for this server
                for (key, value) in env_vars {
                    std::env::set_var(key, value);
                }
                
                if let Err(e) = client.connect_server(&server_name, &command, server_args).await {
                    eprintln!("{} Failed to connect to MCP server '{}': {}", "Error:".red(), server_name, e);
                    process::exit(1);
                }
            }
            
            Some(client)
        } else {
            None
        }
    };

    // Web search is now handled by MCP tools (like gemini), not built-in :online suffix
    let final_model = config.model.clone();

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
    let tools = if !config.disable_tools && mcp_client.is_some() {
        if let Some(ref client) = mcp_client {
            let mcp_tools = client.list_tools().await;
            if !mcp_tools.is_empty() {
                if config.verbose {
                    println!("{}", format!("Available MCP tools: {}", mcp_tools.len()).cyan());
                }
                Some(mcp::tools::format_tools_for_llm(&mcp_tools))
            } else {
                None
            }
        } else {
            None
        }
    } else {
        None
    };

    // Use non-streaming when tools are available for proper tool handling
    // OpenRouter's streaming API doesn't properly stream tool call arguments
    let use_streaming = tools.is_none();
    
    let request_body = RequestBody {
        model: final_model.clone(),
        messages: messages.clone(),
        stream: use_streaming,
        reasoning: config.reasoning.clone(),
        tools: tools.clone(),
    };
    
    // Debug: Print tools being sent
    if config.verbose && tools.is_some() {
        eprintln!("{}", "[AI] Sending tools to model for function calling".dimmed());
    }

    if config.verbose {
        eprintln!("{}", format!("[AI] Using model: {}", final_model).dimmed());
    }

    // Make API request
    if config.verbose {
        eprintln!("{}", "[AI] Making API request...".dimmed());
    }
    let response = make_api_request(&config.api_key, &config.api_endpoint, &request_body).await?;

    if config.verbose {
        eprintln!("{}", format!("[AI] Response status: {}", response.status()).dimmed());
    }

    if !response.status().is_success() {
        let status = response.status();
        let error_text = response.text().await?;
        eprintln!(
            "{} HTTP error! status: {}, message: {}",
            "Error:".red(),
            status,
            error_text
        );
        process::exit(1);
    }

    // Process response based on whether we're streaming or not
    let assistant_response = if use_streaming {
        // Streaming path - no tools available
        let streaming_result = process_streaming_response(
            response,
            config.stream_timeout,
            args.reasoning_exclude,
            config.verbose,
        )
        .await?;
        
        streaming_result.content
    } else {
        // Non-streaming path - handle tools properly
        let response_text = response.text().await?;
        if config.verbose {
            eprintln!("{}", format!("[AI] Raw response: {}", response_text).dimmed());
        }
        
        // Parse the response
        let response_json: serde_json::Value = serde_json::from_str(&response_text)?;
        
        // Process the non-streaming response with tool handling
        if let Some(choices) = response_json.get("choices").and_then(|c| c.as_array()) {
            if let Some(first_choice) = choices.first() {
                // Check for reasoning content first
                if let Some(reasoning_content) = first_choice.get("message")
                    .and_then(|m| m.get("reasoning_content"))
                    .and_then(|r| r.as_str()) {
                    
                    if !args.reasoning_exclude && !reasoning_content.is_empty() {
                        println!();
                        println!("{}", format!("{}[{}]{}", "┌─".dimmed(), "REASONING".cyan().bold(), "──────────────────────────────────────────────".dimmed()));
                        let display_reasoning = reasoning_content
                            .replace("**", "")
                            .trim()
                            .to_string();
                        println!("{}", display_reasoning.dimmed());
                        println!("{}", "└──────────────────────────────────────────────────────────".dimmed());
                        println!();
                    }
                }
                
                if let Some(message) = first_choice.get("message") {
                    // Check if there are tool calls
                    if let Some(tool_calls) = message.get("tool_calls").and_then(|tc| tc.as_array()) {
                        if !tool_calls.is_empty() {
                            if config.verbose {
                                println!("{}", "Executing tools...".cyan());
                            }
                            
                            // Execute each tool call
                            let mut tool_results = Vec::new();
                            for tool_call in tool_calls {
                                if let (Some(id), Some(function)) = (
                                    tool_call.get("id").and_then(|i| i.as_str()),
                                    tool_call.get("function")
                                ) {
                                    if let (Some(name), Some(arguments_str)) = (
                                        function.get("name").and_then(|n| n.as_str()),
                                        function.get("arguments").and_then(|a| a.as_str())
                                    ) {
                                        println!("{}", format!("Calling tool: {}...", name).cyan());
                                        
                                        // Parse arguments
                                        if let Ok(arguments) = serde_json::from_str::<serde_json::Value>(arguments_str) {
                                            // Execute tool via MCP
                                            if let Some(ref client) = mcp_client {
                                                let mcp_tool_call = mcp::McpToolCall {
                                                    name: name.to_string(),
                                                    arguments,
                                                };
                                                
                                                match client.call_tool(&mcp_tool_call).await {
                                                    Ok(result) => {
                                                        if let Some(content) = result.content.first() {
                                                            println!("{}", content.text);
                                                            
                                                            // Store the result to send back to the AI
                                                            tool_results.push(Message {
                                                                role: "tool".to_string(),
                                                                content: Some(content.text.clone()),
                                                                tool_calls: None,
                                                                tool_call_id: Some(id.to_string()),
                                                            });
                                                        }
                                                    }
                                                    Err(e) => {
                                                        eprintln!("{}", format!("Tool execution error: {}", e).red());
                                                        tool_results.push(Message {
                                                            role: "tool".to_string(),
                                                            content: Some(format!("Error: {}", e)),
                                                            tool_calls: None,
                                                            tool_call_id: Some(id.to_string()),
                                                        });
                                                    }
                                                }
                                            }
                                        }
                                    }
                                }
                            }
                            
                            // If we executed tools, we need to send the results back and get a new response
                            if !tool_results.is_empty() {
                                // Add the assistant's message with tool calls to the conversation
                                // Convert tool_calls array to proper ToolCall objects
                                let tool_calls_typed: Vec<models::ToolCall> = tool_calls
                                    .iter()
                                    .filter_map(|tc| serde_json::from_value(tc.clone()).ok())
                                    .collect();
                                
                                messages.push(Message {
                                    role: "assistant".to_string(),
                                    content: message.get("content").and_then(|c| c.as_str()).map(|s| s.to_string()),
                                    tool_calls: if tool_calls_typed.is_empty() { None } else { Some(tool_calls_typed) },
                                    tool_call_id: None,
                                });
                                
                                // Add tool results to the conversation
                                for result in tool_results {
                                    messages.push(result);
                                }
                                
                                // Make another API call to get the final response - NOW WITH STREAMING!
                                let followup_request = RequestBody {
                                    model: final_model.clone(),
                                    messages: messages.clone(),
                                    stream: true,  // Enable streaming for the final answer
                                    reasoning: config.reasoning.clone(),
                                    tools: None,  // Don't send tools again for the final response
                                };
                                
                                if config.verbose {
                                    eprintln!("{}", "[AI] Making follow-up request with tool results (streaming enabled)...".dimmed());
                                }
                                
                                let followup_response = make_api_request(&config.api_key, &config.api_endpoint, &followup_request).await?;
                                
                                if !followup_response.status().is_success() {
                                    let status = followup_response.status();
                                    let error_text = followup_response.text().await?;
                                    eprintln!(
                                        "{} HTTP error! status: {}, message: {}",
                                        "Error:".red(),
                                        status,
                                        error_text
                                    );
                                    process::exit(1);
                                }
                                
                                // Process the follow-up STREAMING response for better UX
                                let followup_result = process_streaming_response(
                                    followup_response,
                                    config.stream_timeout,
                                    args.reasoning_exclude,
                                    config.verbose,
                                )
                                .await?;
                                
                                // Return the final streamed response
                                followup_result.content
                            } else {
                                // No tool calls, return the content
                                "Tools executed successfully".to_string()
                            }
                        } else {
                            // No tool calls, just return the content
                            "No tool calls in response".to_string()
                        }
                    } else if let Some(content) = message.get("content").and_then(|c| c.as_str()) {
                        // Use CodeBuffer to properly format the response with syntax highlighting
                        let mut code_buffer = highlight::CodeBuffer::new();
                        let formatted = code_buffer.append(content);
                        if !formatted.is_empty() {
                            print!("{}", formatted);
                        }
                        let remaining = code_buffer.flush();
                        if !remaining.is_empty() {
                            print!("{}", remaining.trim_end());
                        }
                        println!();
                        content.to_string()
                    } else {
                        "No response content".to_string()
                    }
                } else {
                    "No message in response".to_string()
                }
            } else {
                "No choices in response".to_string()
            }
        } else {
            "Invalid response format".to_string()
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
            if config.verbose {
                eprintln!(
                    "{}",
                    format!("[AI] Warning: Failed to save session: {}", e).dimmed()
                );
            }
        }
    }

    // Cleanup MCP servers on exit
    if let Some(client) = mcp_client {
        let _ = client.shutdown().await;
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
        "      --mcp-server           Connect to MCP server (format: name:command:arg1,arg2,...)".dimmed()
    );
    eprintln!(
        "{}",
        "      --no-tools             Disable MCP tools for this query".dimmed()
    );
    eprintln!(
        "{}",
        "      --config-init          Initialize a config file with example MCP servers".dimmed()
    );
    eprintln!(
        "{}",
        "      --api-endpoint         Custom API base URL (e.g., http://localhost:11434/v1)".dimmed()
    );
    eprintln!(
        "{}",
        "  -h, --help                 Print help".dimmed()
    );
}

async fn make_api_request(
    api_key: &str,
    api_endpoint: &str,
    request_body: &RequestBody,
) -> Result<reqwest::Response, reqwest::Error> {
    let mut headers = HeaderMap::new();
    headers.insert(
        AUTHORIZATION,
        HeaderValue::from_str(&format!("Bearer {}", api_key)).unwrap(),
    );
    headers.insert(CONTENT_TYPE, HeaderValue::from_static("application/json"));

    let client = reqwest::Client::builder()
        .default_headers(headers)
        .build()?;

    client
        .post(api_endpoint)
        .json(&request_body)
        .send()
        .await
}

struct StreamingResult {
    content: String,
}

async fn process_streaming_response(
    response: reqwest::Response,
    timeout_secs: u64,
    reasoning_exclude: bool,
    verbose: bool,
) -> Result<StreamingResult, Box<dyn std::error::Error>> {
    let mut stream = response.bytes_stream();
    let mut buffer = String::new();
    let mut citations: Vec<Citation> = vec![];
    let mut code_buffer = CodeBuffer::new();
    let mut last_flush = std::time::Instant::now();
    let flush_interval = std::time::Duration::from_millis(50);
    let mut incomplete_line = String::new();
    let mut assistant_response = String::new();
    let mut reasoning_response = String::new();
    let mut reasoning_buffer = String::new();
    let mut reasoning_displayed = false;
    let chunk_timeout = Duration::from_secs(timeout_secs);

    loop {
        match timeout(chunk_timeout, stream.next()).await {
            Ok(Some(chunk)) => {
                let chunk = chunk?;
                let text = String::from_utf8_lossy(&chunk);
                incomplete_line.push_str(&text);
            }
            Ok(None) => break,
            Err(_) => {
                eprintln!(
                    "{}",
                    format!(
                        "Error: Connection timeout - no data received for {} seconds",
                        timeout_secs
                    )
                    .red()
                );
                eprintln!(
                    "{}",
                    "The AI service may be experiencing issues or the connection was lost."
                        .dimmed()
                );

                let remaining = code_buffer.flush();
                if !remaining.is_empty() {
                    print!("{}", remaining.trim_end());
                    println!();
                }

                io::stdout().flush()?;
                process::exit(1);
            }
        }

        // Find last newline to ensure we only process complete lines
        if let Some(last_newline_pos) = incomplete_line.rfind('\n') {
            buffer.push_str(&incomplete_line[..=last_newline_pos]);
            incomplete_line = incomplete_line[last_newline_pos + 1..].to_string();
        } else {
            continue;
        }

        // Process complete lines
        while let Some(line_end) = buffer.find('\n') {
            let line = buffer[..line_end].to_string();
            buffer = buffer[line_end + 1..].to_string();

            if line.is_empty() || line.starts_with(':') {
                continue;
            }

            // Parse SSE field
            if let Some(colon_pos) = line.find(':') {
                let field = line[..colon_pos].trim();
                let value = line[colon_pos + 1..].trim_start();

                match field {
                    "data" => {
                        if value == "[DONE]" {
                            // Close reasoning section if it was displayed
                            if reasoning_displayed && !reasoning_exclude {
                                // Always ensure we're on a new line before closing the box
                                println!();
                                println!(
                                    "{}",
                                    "└──────────────────────────────────────────────────────────"
                                        .dimmed()
                                );
                            }

                            // Flush any remaining content
                            let remaining = code_buffer.flush();
                            if !remaining.is_empty() {
                                print!("{}", remaining.trim_end());
                            }

                            // Display citations if any
                            if !citations.is_empty() {
                                println!("{}", "\n\n---\nSources:".dimmed());
                                for (index, citation) in citations.iter().enumerate() {
                                    println!(
                                        "{}",
                                        format!("[{}] {}", index + 1, citation.title).cyan()
                                    );
                                    println!("{}", format!("    {}", citation.url).dimmed());
                                }
                            }

                            println!();
                            io::stdout().flush()?;

                            return Ok(StreamingResult {
                                content: assistant_response,
                            });
                        }

                        // Parse JSON data
                        match serde_json::from_str::<StreamResponse>(value) {
                            Ok(parsed) => {
                                if let Some(choices) = parsed.choices {
                                    for choice in choices {
                                        if let Some(delta) = choice.delta {
                                            // Process reasoning tokens
                                            if let Some(reasoning) = delta.reasoning {
                                                reasoning_response.push_str(&reasoning);
                                                reasoning_buffer.push_str(&reasoning);

                                                if !reasoning_exclude {
                                                    if !reasoning_displayed {
                                                        println!();
                                                        println!("{}", format!("{}[{}]{}", "┌─".dimmed(), "REASONING".cyan().bold(), "──────────────────────────────────────────────".dimmed()));
                                                        reasoning_displayed = true;
                                                    }

                                                    // Clean up markdown formatting for display
                                                    let display_reasoning = reasoning
                                                        .replace("**", "")
                                                        .trim_end()
                                                        .to_string();
                                                    
                                                    if !display_reasoning.is_empty() {
                                                        print!("{}", display_reasoning.dimmed());
                                                        if last_flush.elapsed() > flush_interval {
                                                            io::stdout().flush()?;
                                                            last_flush = std::time::Instant::now();
                                                        }
                                                    }
                                                }
                                            }

                                            // Tool calls are not processed in streaming mode
                                            
                                            // Process content
                                            if let Some(content) = delta.content {
                                                // Only close reasoning block if we have actual content (not just empty string)
                                                if reasoning_displayed && !reasoning_exclude && !content.trim().is_empty() {
                                                    // Always ensure we're on a new line before closing the box
                                                    println!();
                                                    println!("{}", "└──────────────────────────────────────────────────────────".dimmed());
                                                    println!(); // Add spacing after reasoning block
                                                    reasoning_displayed = false;
                                                    reasoning_buffer.clear();
                                                }

                                                assistant_response.push_str(&content);

                                                let formatted = code_buffer.append(&content);
                                                if !formatted.is_empty() {
                                                    print!("{}", formatted);

                                                    if last_flush.elapsed() > flush_interval {
                                                        io::stdout().flush()?;
                                                        last_flush = std::time::Instant::now();
                                                    }
                                                }
                                            }

                                            // Process annotations
                                            if let Some(annotations) = delta.annotations {
                                                for annotation in annotations {
                                                    if annotation.annotation_type == "url_citation"
                                                    {
                                                        if let Some(citation) =
                                                            annotation.url_citation
                                                        {
                                                            if !citations
                                                                .iter()
                                                                .any(|c| c.url == citation.url)
                                                            {
                                                                citations.push(citation);
                                                            }
                                                        }
                                                    }
                                                }
                                            }
                                        }
                                    }
                                }
                            }
                            Err(e) => {
                                if verbose {
                                    eprintln!(
                                        "{}",
                                        format!("[AI] JSON parse error: {}", e).dimmed()
                                    );
                                }
                            }
                        }
                    }
                    "event" | "id" | "retry" => {
                        if verbose {
                            eprintln!("{}", format!("[AI] SSE {}: {}", field, value).dimmed());
                        }
                    }
                    _ => {
                        if verbose {
                            eprintln!("{}", format!("[AI] Unknown SSE field: {}", field).dimmed());
                        }
                    }
                }
            }
        }
    }

    // Handle case where stream ends without [DONE]
    if reasoning_displayed && !reasoning_exclude {
        let trailing_newlines = reasoning_buffer
            .chars()
            .rev()
            .take_while(|&c| c == '\n')
            .count();

        if trailing_newlines > 1 {
            print!("\x1b[{}A", trailing_newlines - 1);
        }

        println!(
            "{}",
            "└──────────────────────────────────────────────────────────".dimmed()
        );
    }

    let remaining = code_buffer.flush();
    if !remaining.is_empty() {
        print!("{}", remaining.trim_end());
    }

    if !citations.is_empty() {
        println!("{}", "\n\n---\nSources:".dimmed());
        for (index, citation) in citations.iter().enumerate() {
            println!("{}", format!("[{}] {}", index + 1, citation.title).cyan());
            println!("{}", format!("    {}", citation.url).dimmed());
        }
    }

    println!();
    io::stdout().flush()?;

    Ok(StreamingResult {
        content: assistant_response,
    })
}
