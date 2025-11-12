use crate::api::models::{Citation, StreamResponse};
use crate::error::{Cmd2AiError, Result};
use crate::ui::highlight::CodeBuffer;
use colored::*;
use futures::StreamExt;
use std::io::{self, Write};
use tokio::time::{timeout, Duration};

pub struct StreamingResult {
    pub content: String,
}

pub async fn process_streaming_response(
    response: reqwest::Response,
    timeout_secs: u64,
    reasoning_exclude: bool,
    verbose: bool,
) -> Result<StreamingResult> {
    let mut stream = response.bytes_stream();
    let mut buffer = String::new();
    let mut citations: Vec<Citation> = vec![];
    let mut code_buffer = CodeBuffer::new();
    let mut reasoning_code_buffer = CodeBuffer::new();
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
                let chunk = chunk.map_err(|e| Cmd2AiError::NetworkError(e))?;
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
                return Err(Cmd2AiError::Timeout);
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
                                // Close reasoning block with CodeBuffer
                                // Avoid double newline if reasoning_buffer already ends with one
                                let sep = if reasoning_buffer.ends_with('\n') { "" } else { "\n" };
                                let reasoning_end = format!("{}\n```", sep);
                                let formatted = reasoning_code_buffer.append(&reasoning_end);
                                if !formatted.is_empty() {
                                    print!("{}", formatted);
                                }
                                let remaining = reasoning_code_buffer.flush();
                                if !remaining.is_empty() {
                                    print!("{}", remaining.trim_end());
                                }
                                println!();
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
                                                        // Start reasoning block with CodeBuffer
                                                        println!();
                                                        let reasoning_start = "```REASONING\n";
                                                        let formatted =
                                                            reasoning_code_buffer.append(reasoning_start);
                                                        if !formatted.is_empty() {
                                                            print!("{}", formatted);
                                                        }
                                                        reasoning_displayed = true;
                                                    }

                                                    // Clean up markdown formatting for display
                                                    let display_reasoning = reasoning
                                                        .replace("**", "")
                                                        .trim_end()
                                                        .to_string();

                                                    if !display_reasoning.is_empty() {
                                                        // Append reasoning content to CodeBuffer
                                                        let formatted =
                                                            reasoning_code_buffer.append(&display_reasoning);
                                                        if !formatted.is_empty() {
                                                            print!("{}", formatted);
                                                        }
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
                                                if reasoning_displayed
                                                    && !reasoning_exclude
                                                    && !content.trim().is_empty()
                                                {
                                                    // Close reasoning block with CodeBuffer
                                                    // Avoid double newline if reasoning_buffer already ends with one
                                                    let sep =
                                                        if reasoning_buffer.ends_with('\n') { "" } else { "\n" };
                                                    let reasoning_end = format!("{}\n```", sep);
                                                    let formatted =
                                                        reasoning_code_buffer.append(&reasoning_end);
                                                    if !formatted.is_empty() {
                                                        print!("{}", formatted);
                                                    }
                                                    let remaining = reasoning_code_buffer.flush();
                                                    if !remaining.is_empty() {
                                                        print!("{}", remaining.trim_end());
                                                    }
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
                                                    if annotation.annotation_type == "url_citation" {
                                                        if let Some(citation) = annotation.url_citation {
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
        // Close reasoning block with CodeBuffer
        // Avoid double newline if reasoning_buffer already ends with one
        let sep = if reasoning_buffer.ends_with('\n') { "" } else { "\n" };
        let reasoning_end = format!("{}\n```", sep);
        let formatted = reasoning_code_buffer.append(&reasoning_end);
        if !formatted.is_empty() {
            print!("{}", formatted);
        }
        let remaining = reasoning_code_buffer.flush();
        if !remaining.is_empty() {
            print!("{}", remaining.trim_end());
        }
        println!();
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

