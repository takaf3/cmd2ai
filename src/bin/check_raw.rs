use colored::*;
use futures::StreamExt;
use reqwest::header::{HeaderMap, HeaderValue, AUTHORIZATION, CONTENT_TYPE};
use serde_json::{json, Value};
use std::env;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Get command line args
    let args: Vec<String> = env::args().collect();
    if args.len() < 2 {
        eprintln!("Usage: {} <prompt> [--reasoning]", args[0]);
        std::process::exit(1);
    }

    let prompt = &args[1];
    let use_reasoning = args.len() > 2 && args[2] == "--reasoning";

    // Get API key
    let api_key = env::var("OPENROUTER_API_KEY").unwrap_or_else(|_| {
        eprintln!("Error: OPENROUTER_API_KEY environment variable not set");
        std::process::exit(1);
    });

    // Get model
    let model = env::var("AI_MODEL").unwrap_or_else(|_| "openai/gpt-4o-mini".to_string());

    println!("{}", format!("Using model: {}", model).green());
    println!("{}", format!("Prompt: {}", prompt).cyan());
    println!(
        "{}",
        format!("Reasoning: {}", if use_reasoning { "Enabled" } else { "Disabled" }).yellow()
    );
    println!("{}", "-".repeat(80).dimmed());

    // Build request body
    let mut request_body = json!({
        "model": model,
        "messages": [
            {
                "role": "system",
                "content": format!("Today's date is {}.", chrono::Local::now().format("%A, %B %d, %Y"))
            },
            {
                "role": "user",
                "content": prompt
            }
        ],
        "stream": true
    });

    // Add reasoning if requested
    if use_reasoning {
        request_body["reasoning"] = json!({
            "effort": "high",
            "enabled": true
        });
    }

    println!("{}", "Request payload:".bold());
    println!("{}", serde_json::to_string_pretty(&request_body)?);
    println!("{}", "-".repeat(80).dimmed());

    // Make request
    let mut headers = HeaderMap::new();
    headers.insert(
        AUTHORIZATION,
        HeaderValue::from_str(&format!("Bearer {}", api_key))?,
    );
    headers.insert(CONTENT_TYPE, HeaderValue::from_static("application/json"));

    let client = reqwest::Client::builder()
        .default_headers(headers)
        .build()?;

    let response = client
        .post("https://openrouter.ai/api/v1/chat/completions")
        .json(&request_body)
        .send()
        .await?;

    if !response.status().is_success() {
        eprintln!("{}", format!("Error: HTTP {}", response.status()).red());
        eprintln!("{}", response.text().await?);
        std::process::exit(1);
    }

    println!("{}", "Raw SSE stream:".bold());
    println!("{}", "-".repeat(80).dimmed());

    // Process streaming response
    let mut stream = response.bytes_stream();
    let mut buffer = String::new();

    while let Some(chunk) = stream.next().await {
        let chunk = chunk?;
        let text = String::from_utf8_lossy(&chunk);
        buffer.push_str(&text);

        // Process complete lines
        while let Some(newline_pos) = buffer.find('\n') {
            let line = buffer[..newline_pos].to_string();
            buffer = buffer[newline_pos + 1..].to_string();

            if line.is_empty() {
                println!();
                continue;
            }

            println!("{}", line.dimmed());

            // Parse data lines
            if line.starts_with("data: ") && line != "data: [DONE]" {
                let data_str = &line[6..];
                match serde_json::from_str::<Value>(data_str) {
                    Ok(data) => {
                        println!("\n{}", "Parsed JSON:".green());
                        println!("{}", serde_json::to_string_pretty(&data)?);
                        
                        // Highlight specific fields
                        if let Some(choices) = data["choices"].as_array() {
                            for choice in choices {
                                if let Some(delta) = choice["delta"].as_object() {
                                    if let Some(content) = delta["content"].as_str() {
                                        println!("{}: {}", "Content".yellow(), content);
                                    }
                                    if let Some(reasoning) = delta["reasoning"].as_str() {
                                        println!("{}: {}", "Reasoning".cyan(), reasoning);
                                    }
                                }
                            }
                        }
                        println!();
                    }
                    Err(e) => {
                        eprintln!("{}", format!("JSON parse error: {}", e).red());
                    }
                }
            }
        }
    }

    println!("{}", "-".repeat(80).dimmed());
    println!("{}", "Stream ended".green());

    Ok(())
}