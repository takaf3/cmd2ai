mod cli;
mod config;
mod highlight;
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
                content: system_content,
            },
        );
    }

    // Add user message
    messages.push(Message {
        role: "user".to_string(),
        content: command.clone(),
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

    let request_body = RequestBody {
        model: final_model.clone(),
        messages: messages.clone(),
        stream: true,
        plugins,
        reasoning: config.reasoning,
    };

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

    // Process streaming response
    let assistant_response = process_streaming_response(
        response,
        config.stream_timeout,
        args.reasoning_exclude,
        config.verbose,
    )
    .await?;

    // Save session with assistant's response
    if !assistant_response.is_empty() {
        session.messages = messages;
        session.messages.push(Message {
            role: "assistant".to_string(),
            content: assistant_response,
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
                                // Check how many trailing newlines we have
                                let trailing_newlines = reasoning_buffer
                                    .chars()
                                    .rev()
                                    .take_while(|&c| c == '\n')
                                    .count();

                                // If we have more than one trailing newline, we need to handle the extra space
                                if trailing_newlines > 1 {
                                    // Use backspace to remove extra lines
                                    print!("\x1b[{}A", trailing_newlines - 1);
                                }

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
                                                        println!("{}", format!("{}[REASONING]{}", "┌─".dimmed(), "──────────────────────────────────────────────".dimmed()).cyan());
                                                        reasoning_displayed = true;
                                                    }

                                                    print!("{}", reasoning);
                                                    if last_flush.elapsed() > flush_interval {
                                                        io::stdout().flush()?;
                                                        last_flush = std::time::Instant::now();
                                                    }
                                                }
                                            }

                                            // Process content
                                            if let Some(content) = delta.content {
                                                if reasoning_displayed && !reasoning_exclude {
                                                    println!();
                                                    println!("{}", "└──────────────────────────────────────────────────────────".dimmed());
                                                    reasoning_displayed = false;
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
