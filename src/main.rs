mod cli;
mod config;
mod highlight;
mod mcp;
mod models;
mod search;
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
use mcp::{McpClient, McpToolCall};
use models::{Citation, Message, RequestBody, StreamResponse, WebPlugin};
use search::should_use_web_search;
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

    // Initialize MCP client if servers are specified
    let mcp_client = if !args.mcp_servers.is_empty() {
        let client = McpClient::new();
        
        for server_spec in &args.mcp_servers {
            let parts: Vec<&str> = server_spec.splitn(3, ':').collect();
            if parts.len() < 2 {
                eprintln!("{} Invalid MCP server format: {}", "Error:".red(), server_spec);
                eprintln!("Expected format: name:command or name:command:arg1,arg2,...");
                process::exit(1);
            }
            
            let server_name = parts[0];
            let command = parts[1];
            let args_str = if parts.len() > 2 { parts[2] } else { "" };
            let server_args: Vec<String> = if !args_str.is_empty() {
                args_str.split(',').map(|s| s.to_string()).collect()
            } else {
                vec![]
            };
            
            println!("{}", format!("Connecting to MCP server '{}'...", server_name).cyan());
            if let Err(e) = client.connect_server(server_name, command, server_args).await {
                eprintln!("{} Failed to connect to MCP server '{}': {}", "Error:".red(), server_name, e);
                process::exit(1);
            }
        }
        
        Some(client)
    } else {
        None
    };

    let use_web_search = should_use_web_search(&command, args.force_search, args.no_search);

    let final_model = if use_web_search && !config.model.contains(":online") {
        format!("{}:online", config.model)
    } else {
        config.model.clone()
    };

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

    let plugins = if use_web_search {
        Some(vec![WebPlugin {
            id: "web".to_string(),
            max_results: config.web_search_max_results,
            search_prompt: format!(
                "A web search was conducted on {}. Use the following web search results to answer the user's question.\n\nIMPORTANT: Do NOT include URLs or citations in your answer. Just provide a clean, natural response based on the information found. The sources will be displayed separately.",
                Config::get_search_date()
            ),
        }])
    } else {
        None
    };

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

    // Get available tools if MCP is enabled and use_tools flag is set
    let tools = if args.use_tools && mcp_client.is_some() {
        if let Some(ref client) = mcp_client {
            let mcp_tools = client.list_tools().await;
            if !mcp_tools.is_empty() {
                println!("{}", format!("Available MCP tools: {}", mcp_tools.len()).cyan());
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

    // Use non-streaming when tools are available to properly handle tool calls
    let use_streaming = tools.is_none();
    
    let request_body = RequestBody {
        model: final_model.clone(),
        messages: messages.clone(),
        stream: use_streaming,
        plugins,
        reasoning: config.reasoning,
        tools: tools.clone(),
    };
    
    // Debug: Print tools being sent
    if config.verbose && tools.is_some() {
        eprintln!("{}", "[AI] Sending tools to model for function calling".dimmed());
    }

    if config.verbose {
        eprintln!("{}", format!("[AI] Using model: {}", final_model).dimmed());
        eprintln!(
            "{}",
            format!(
                "[AI] Web search: {}",
                if use_web_search {
                    "enabled"
                } else {
                    "disabled"
                }
            )
            .dimmed()
        );
        if use_web_search {
            eprintln!(
                "{}",
                format!("[AI] Max search results: {}", config.web_search_max_results).dimmed()
            );
        }
    }

    // Make API request
    let response = make_api_request(&config.api_key, &request_body).await?;

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

    // Process response - streaming or non-streaming based on tools
    let assistant_response = if use_streaming {
        process_streaming_response(
            response,
            config.stream_timeout,
            args.reasoning_exclude,
            config.verbose,
        )
        .await?
    } else {
        // Non-streaming response for tool handling
        let response_text = response.text().await?;
        if config.verbose {
            eprintln!("{}", format!("[AI] Raw response: {}", response_text).dimmed());
        }
        
        // Parse the response
        let response_json: serde_json::Value = serde_json::from_str(&response_text)?;
        
        // Check for tool calls in the response
        if let Some(choices) = response_json.get("choices").and_then(|c| c.as_array()) {
            if let Some(first_choice) = choices.first() {
                if let Some(message) = first_choice.get("message") {
                    // Check if there are tool calls
                    if let Some(tool_calls) = message.get("tool_calls").and_then(|tc| tc.as_array()) {
                        println!("{}", "Executing tools...".cyan());
                        
                        // Execute each tool call
                        for tool_call in tool_calls {
                            if let (Some(id), Some(function)) = (
                                tool_call.get("id").and_then(|i| i.as_str()),
                                tool_call.get("function")
                            ) {
                                if let (Some(name), Some(arguments_str)) = (
                                    function.get("name").and_then(|n| n.as_str()),
                                    function.get("arguments").and_then(|a| a.as_str())
                                ) {
                                    println!("{}", format!("Calling tool: {}", name).yellow());
                                    
                                    // Parse arguments
                                    if let Ok(arguments) = serde_json::from_str::<serde_json::Value>(arguments_str) {
                                        // Execute tool via MCP
                                        if let Some(ref client) = mcp_client {
                                            let tool_call = mcp::McpToolCall {
                                                name: name.to_string(),
                                                arguments,
                                            };
                                            
                                            match client.call_tool(&tool_call).await {
                                                Ok(result) => {
                                                    println!("{}", "Tool executed successfully!".green());
                                                    // Format result for display
                                                    if let Some(content) = result.content.first() {
                                                        println!("{}", content.text);
                                                    }
                                                }
                                                Err(e) => {
                                                    eprintln!("{}", format!("Tool execution error: {}", e).red());
                                                }
                                            }
                                        }
                                    }
                                }
                            }
                        }
                        
                        "Tools executed successfully".to_string()
                    } else if let Some(content) = message.get("content").and_then(|c| c.as_str()) {
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
        "  -s, --search               Force web search".dimmed()
    );
    eprintln!(
        "{}",
        "      --no-search            Disable web search".dimmed()
    );
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
}

async fn make_api_request(
    api_key: &str,
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
        .post("https://openrouter.ai/api/v1/chat/completions")
        .json(&request_body)
        .send()
        .await
}

async fn process_streaming_response(
    response: reqwest::Response,
    timeout_secs: u64,
    reasoning_exclude: bool,
    verbose: bool,
) -> Result<String, Box<dyn std::error::Error>> {
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

                            return Ok(assistant_response);
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

    Ok(assistant_response)
}
