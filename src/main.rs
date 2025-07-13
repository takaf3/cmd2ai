use clap::Parser;
use colored::*;
use futures::StreamExt;
use reqwest::header::{HeaderMap, HeaderValue, AUTHORIZATION, CONTENT_TYPE};
use serde::{Deserialize, Serialize};
use std::env;
use std::fs;
use std::io::{self, Write};
use std::path::{Path, PathBuf};
use std::process;
use syntect::easy::HighlightLines;
use syntect::highlighting::{Style, ThemeSet};
use syntect::parsing::SyntaxSet;
use syntect::util::{as_24_bit_terminal_escaped, LinesWithEndings};
use tokio::time::{timeout, Duration};
use uuid::Uuid;

const WEB_SEARCH_KEYWORDS: &[&str] = &[
    "latest",
    "recent",
    "current",
    "today",
    "yesterday",
    "news",
    "update",
    "price",
    "stock",
    "weather",
    "score",
    "result",
    "released",
    "announced",
    "trending",
    "happening",
    "now",
    "breaking",
    "2024",
    "2025",
    "this week",
    "this month",
    "real-time",
    "live",
    "status",
    "outage",
    "down",
];

const INFO_KEYWORDS: &[&str] = &[
    "what is",
    "who is",
    "where is",
    "when is",
    "how to",
    "tell me about",
    "explain",
    "define",
    "information about",
];

const NO_SEARCH_KEYWORDS: &[&str] = &[
    "hi",
    "hello",
    "hey",
    "thanks",
    "thank you",
    "bye",
    "goodbye",
    "please",
    "help me write",
    "code",
    "implement",
    "fix",
    "debug",
    "create",
    "make",
    "build",
];

const SESSION_EXPIRY_MINUTES: i64 = 30;
const MAX_CONVERSATION_PAIRS: usize = 3; // Keep last 3 exchanges (6 messages)

#[derive(Parser, Debug)]
#[command(name = "ai")]
#[command(about = "AI command-line tool using OpenRouter API", long_about = None)]
struct Args {
    #[arg(short = 's', long = "search", help = "Force web search")]
    force_search: bool,

    #[arg(long = "no-search", help = "Disable web search")]
    no_search: bool,

    #[arg(short = 'n', long = "new", help = "Start a new conversation")]
    new_conversation: bool,

    #[arg(
        short = 'c',
        long = "continue",
        help = "Continue previous conversation even if expired"
    )]
    force_continue: bool,

    #[arg(long = "clear", help = "Clear all conversation history")]
    clear_history: bool,

    #[arg(help = "Command to send to AI")]
    command: Vec<String>,
}

#[derive(Serialize, Deserialize, Clone)]
struct Message {
    role: String,
    content: String,
}

#[derive(Serialize, Deserialize, Clone)]
struct Session {
    session_id: String,
    last_updated: chrono::DateTime<chrono::Local>,
    messages: Vec<Message>,
}

#[derive(Serialize)]
struct WebPlugin {
    id: String,
    max_results: u32,
    search_prompt: String,
}

#[derive(Serialize)]
struct RequestBody {
    model: String,
    messages: Vec<Message>,
    stream: bool,
    #[serde(skip_serializing_if = "Option::is_none")]
    plugins: Option<Vec<WebPlugin>>,
}

#[derive(Deserialize)]
struct Citation {
    url: String,
    title: String,
    #[allow(dead_code)]
    content: Option<String>,
}

#[derive(Deserialize)]
struct Annotation {
    #[serde(rename = "type")]
    annotation_type: String,
    url_citation: Option<Citation>,
}

#[derive(Deserialize)]
struct Delta {
    content: Option<String>,
    annotations: Option<Vec<Annotation>>,
}

#[derive(Deserialize)]
struct Choice {
    delta: Option<Delta>,
}

#[derive(Deserialize)]
struct StreamResponse {
    choices: Option<Vec<Choice>>,
}

struct CodeBuffer {
    buffer: String,
    in_code_block: bool,
    code_block_content: String,
    code_block_lang: Option<String>,
    syntax_set: SyntaxSet,
    theme_set: ThemeSet,
}

impl CodeBuffer {
    fn new() -> Self {
        Self {
            buffer: String::new(),
            in_code_block: false,
            code_block_content: String::new(),
            code_block_lang: None,
            syntax_set: SyntaxSet::load_defaults_newlines(),
            theme_set: ThemeSet::load_defaults(),
        }
    }

    fn highlight_code(&self, code: &str, lang: Option<&str>) -> String {
        let theme = &self.theme_set.themes["Solarized (dark)"];

        let syntax = if let Some(lang) = lang {
            self.syntax_set
                .find_syntax_by_token(lang)
                .or_else(|| self.syntax_set.find_syntax_by_extension(lang))
                .unwrap_or_else(|| self.syntax_set.find_syntax_plain_text())
        } else {
            self.syntax_set.find_syntax_plain_text()
        };

        let mut highlighter = HighlightLines::new(syntax, theme);
        let mut output = String::new();

        for line in LinesWithEndings::from(code) {
            let ranges: Vec<(Style, &str)> =
                highlighter.highlight_line(line, &self.syntax_set).unwrap();
            let escaped = as_24_bit_terminal_escaped(&ranges[..], false);
            output.push_str(&escaped);
        }

        output
    }

    fn append(&mut self, content: &str) -> String {
        self.buffer.push_str(content);
        let mut output = String::new();

        while !self.buffer.is_empty() {
            if !self.in_code_block {
                // Look for code block start
                if let Some(code_start) = self.buffer.find("```") {
                    // Output everything before the code block
                    output.push_str(&self.buffer[..code_start]);

                    // Extract the code block marker and language
                    self.buffer = self.buffer[code_start + 3..].to_string();

                    // Check if we have a complete first line with language
                    if let Some(newline_pos) = self.buffer.find('\n') {
                        let lang_line = self.buffer[..newline_pos].trim();
                        self.code_block_lang = if lang_line.is_empty() {
                            None
                        } else {
                            Some(lang_line.to_string())
                        };

                        self.buffer = self.buffer[newline_pos + 1..].to_string();
                        self.in_code_block = true;
                        self.code_block_content.clear();

                        // Output code block header
                        output.push_str(&format!(
                            "{}[{}]{}\n",
                            "┌─".dimmed(),
                            self.code_block_lang.as_deref().unwrap_or("code").cyan(),
                            "─────────────────────────────────────────────────".dimmed()
                        ));
                    } else {
                        // Incomplete first line, wait for more content
                        self.buffer = format!("```{}", self.buffer);
                        break;
                    }
                } else {
                    // No code block found, output everything and clear buffer
                    output.push_str(&self.buffer);
                    self.buffer.clear();
                }
            } else {
                // In code block, look for end marker
                if let Some(code_end) = self.buffer.find("```") {
                    // Add content before the end marker to code block
                    self.code_block_content.push_str(&self.buffer[..code_end]);

                    // Highlight and output the code
                    let highlighted = self
                        .highlight_code(&self.code_block_content, self.code_block_lang.as_deref());
                    output.push_str(&highlighted);

                    // Output code block footer
                    output.push_str(&format!(
                        "{}\n",
                        "└──────────────────────────────────────────────────────────".dimmed()
                    ));

                    // Reset state
                    self.buffer = self.buffer[code_end + 3..].to_string();
                    self.in_code_block = false;
                    self.code_block_content.clear();
                    self.code_block_lang = None;
                } else {
                    // Still in code block, accumulate content
                    self.code_block_content.push_str(&self.buffer);
                    self.buffer.clear();
                    break;
                }
            }
        }

        output
    }

    fn flush(&mut self) -> String {
        let mut output = String::new();

        if self.in_code_block {
            // Unterminated code block
            if !self.code_block_content.is_empty() {
                let highlighted =
                    self.highlight_code(&self.code_block_content, self.code_block_lang.as_deref());
                output.push_str(&highlighted);
                output.push_str(&format!(
                    "{}\n",
                    "└──────────────────────────────────────────────────────────".dimmed()
                ));
            }
        } else if !self.buffer.is_empty() {
            output.push_str(&self.buffer);
        }

        self.buffer.clear();
        self.code_block_content.clear();
        self.in_code_block = false;
        self.code_block_lang = None;

        output
    }
}

fn get_cache_dir() -> PathBuf {
    let home = env::var("HOME").expect("HOME environment variable not set");
    let cache_dir = Path::new(&home).join(".cache").join("cmd2ai");
    if !cache_dir.exists() {
        fs::create_dir_all(&cache_dir).expect("Failed to create cache directory");
    }
    cache_dir
}

fn find_recent_session() -> Option<Session> {
    let cache_dir = get_cache_dir();
    let now = chrono::Local::now();

    // Read all session files and find the most recent valid one
    if let Ok(entries) = fs::read_dir(&cache_dir) {
        let mut sessions: Vec<(PathBuf, Session)> = entries
            .filter_map(|entry| entry.ok())
            .filter_map(|entry| {
                let path = entry.path();
                if path.extension()? == "json"
                    && path.file_name()?.to_str()?.starts_with("session-")
                {
                    let content = fs::read_to_string(&path).ok()?;
                    let session: Session = serde_json::from_str(&content).ok()?;
                    Some((path, session))
                } else {
                    None
                }
            })
            .collect();

        // Sort by last_updated (most recent first)
        sessions.sort_by(|a, b| b.1.last_updated.cmp(&a.1.last_updated));

        // Return the most recent session if it's not expired
        if let Some((path, session)) = sessions.first() {
            let age_minutes = (now - session.last_updated).num_minutes();
            if age_minutes < SESSION_EXPIRY_MINUTES {
                return Some(session.clone());
            } else {
                // Clean up expired session
                let _ = fs::remove_file(path);
            }
        }
    }

    None
}

fn save_session(session: &Session) -> Result<(), Box<dyn std::error::Error>> {
    let cache_dir = get_cache_dir();
    let session_file = cache_dir.join(format!("session-{}.json", session.session_id));
    let content = serde_json::to_string_pretty(session)?;
    fs::write(session_file, content)?;
    Ok(())
}

fn clear_all_sessions() -> Result<(), Box<dyn std::error::Error>> {
    let cache_dir = get_cache_dir();
    if let Ok(entries) = fs::read_dir(&cache_dir) {
        for entry in entries.filter_map(|e| e.ok()) {
            let path = entry.path();
            if path.extension() == Some(std::ffi::OsStr::new("json"))
                && path
                    .file_name()
                    .unwrap()
                    .to_str()
                    .unwrap()
                    .starts_with("session-")
            {
                fs::remove_file(path)?;
            }
        }
    }
    Ok(())
}

fn trim_conversation_history(messages: &mut Vec<Message>) {
    // Keep system message (if exists) + last N conversation pairs
    let mut system_messages: Vec<Message> = messages
        .iter()
        .filter(|m| m.role == "system")
        .cloned()
        .collect();

    let conversation_messages: Vec<Message> = messages
        .iter()
        .filter(|m| m.role != "system")
        .cloned()
        .collect();

    // Keep only the last MAX_CONVERSATION_PAIRS exchanges
    let keep_count = MAX_CONVERSATION_PAIRS * 2; // Each pair has user + assistant
    let trimmed: Vec<Message> = conversation_messages
        .into_iter()
        .rev()
        .take(keep_count)
        .collect::<Vec<_>>()
        .into_iter()
        .rev()
        .collect();

    messages.clear();
    messages.append(&mut system_messages);
    messages.extend(trimmed);
}

fn should_use_web_search(command: &str, force_search: bool, no_search: bool) -> bool {
    if force_search {
        return true;
    }
    if no_search {
        return false;
    }

    let lower_command = command.to_lowercase();

    if NO_SEARCH_KEYWORDS
        .iter()
        .any(|&keyword| lower_command.contains(keyword))
        && !WEB_SEARCH_KEYWORDS
            .iter()
            .any(|&keyword| lower_command.contains(keyword))
    {
        return false;
    }

    if WEB_SEARCH_KEYWORDS
        .iter()
        .any(|&keyword| lower_command.contains(keyword))
    {
        return true;
    }

    if INFO_KEYWORDS
        .iter()
        .any(|&keyword| lower_command.starts_with(keyword))
    {
        return lower_command.contains("company")
            || lower_command.contains("person")
            || lower_command.contains("event")
            || lower_command.contains("place")
            || lower_command.contains("product");
    }

    false
}

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
        eprintln!("{}", "Usage: ai [OPTIONS] <command>".red());
        eprintln!("{}", "  -s, --search         Force web search".dimmed());
        eprintln!("{}", "      --no-search      Disable web search".dimmed());
        eprintln!(
            "{}",
            "  -n, --new            Start a new conversation".dimmed()
        );
        eprintln!(
            "{}",
            "  -c, --continue       Continue previous conversation even if expired".dimmed()
        );
        eprintln!(
            "{}",
            "      --clear          Clear all conversation history".dimmed()
        );
        process::exit(1);
    }

    let command = args.command.join(" ");

    let api_key = env::var("OPENROUTER_API_KEY").unwrap_or_else(|_| {
        eprintln!(
            "{}",
            "Error: OPENROUTER_API_KEY environment variable not set".red()
        );
        process::exit(1);
    });

    // Security: Never log the API key or enable verbose HTTP logging that could expose headers

    let model = env::var("AI_MODEL").unwrap_or_else(|_| "openai/gpt-4.1-mini".to_string());

    let use_web_search = should_use_web_search(&command, args.force_search, args.no_search);

    let final_model = if use_web_search && !model.contains(":online") {
        format!("{}:online", model)
    } else {
        model
    };

    let system_prompt = env::var("AI_SYSTEM_PROMPT").ok();
    let current_date = chrono::Local::now().format("%A, %B %d, %Y").to_string();

    // Load or create session
    let mut session = if args.new_conversation {
        // Create new session
        Session {
            session_id: Uuid::new_v4().to_string(),
            last_updated: chrono::Local::now(),
            messages: vec![],
        }
    } else {
        // Try to find existing session
        let existing_session = find_recent_session();

        if args.force_continue && existing_session.is_some() {
            // Force continue even if expired
            existing_session.unwrap()
        } else {
            // Use existing if found and not expired, otherwise create new
            existing_session.unwrap_or_else(|| Session {
                session_id: Uuid::new_v4().to_string(),
                last_updated: chrono::Local::now(),
                messages: vec![],
            })
        }
    };

    // Build messages array
    let mut messages = session.messages.clone();

    // Add system message if this is a new conversation or no system message exists
    if messages.is_empty() || messages.first().map(|m| &m.role) != Some(&"system".to_string()) {
        let date_prompt = format!("Today's date is {}.", current_date);
        let system_content = if let Some(prompt) = system_prompt {
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
        let max_results = env::var("AI_WEB_SEARCH_MAX_RESULTS")
            .ok()
            .and_then(|s| s.parse::<u32>().ok())
            .unwrap_or(5);

        let search_date = chrono::Local::now().format("%B %d, %Y").to_string();

        Some(vec![WebPlugin {
            id: "web".to_string(),
            max_results,
            search_prompt: format!(
                "A web search was conducted on {}. Use the following web search results to answer the user's question.\n\nIMPORTANT: Do NOT include URLs or citations in your answer. Just provide a clean, natural response based on the information found. The sources will be displayed separately.",
                search_date
            ),
        }])
    } else {
        None
    };

    let request_body = RequestBody {
        model: final_model.clone(),
        messages: messages.clone(),
        stream: true,
        plugins,
    };

    if env::var("AI_VERBOSE").unwrap_or_default() == "true" {
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
                format!(
                    "[AI] Max search results: {}",
                    env::var("AI_WEB_SEARCH_MAX_RESULTS").unwrap_or_else(|_| "5".to_string())
                )
                .dimmed()
            );
        }
    }

    let mut headers = HeaderMap::new();
    headers.insert(
        AUTHORIZATION,
        HeaderValue::from_str(&format!("Bearer {}", api_key))?,
    );
    headers.insert(CONTENT_TYPE, HeaderValue::from_static("application/json"));

    // Build client with explicit configuration to prevent logging sensitive data
    let client = reqwest::Client::builder()
        .default_headers(headers)
        .build()?;
    let response = client
        .post("https://openrouter.ai/api/v1/chat/completions")
        .json(&request_body)
        .send()
        .await?;

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

    let mut stream = response.bytes_stream();
    let mut buffer = String::new();
    let mut citations: Vec<Citation> = vec![];
    let mut code_buffer = CodeBuffer::new();
    let mut last_flush = std::time::Instant::now();
    let flush_interval = std::time::Duration::from_millis(50);
    let mut incomplete_line = String::new();
    let mut assistant_response = String::new(); // Accumulate assistant's response
    let timeout_secs = env::var("AI_STREAM_TIMEOUT")
        .ok()
        .and_then(|s| s.parse::<u64>().ok())
        .unwrap_or(30);
    let chunk_timeout = Duration::from_secs(timeout_secs);

    loop {
        match timeout(chunk_timeout, stream.next()).await {
            Ok(Some(chunk)) => {
                let chunk = chunk?;
                let text = String::from_utf8_lossy(&chunk);

                // Append new data to incomplete line from previous chunk
                incomplete_line.push_str(&text);
            }
            Ok(None) => {
                // Stream ended normally
                break;
            }
            Err(_) => {
                // Timeout occurred
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

                // Flush any buffered content before exiting
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
            // Add complete lines to buffer
            buffer.push_str(&incomplete_line[..=last_newline_pos]);
            // Keep incomplete part for next iteration
            incomplete_line = incomplete_line[last_newline_pos + 1..].to_string();
        } else {
            // No complete line yet, continue accumulating
            continue;
        }

        // Process complete lines
        while let Some(line_end) = buffer.find('\n') {
            let line = buffer[..line_end].to_string();
            buffer = buffer[line_end + 1..].to_string();

            // Handle SSE format - skip empty lines and comments
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

                            // Save session before returning
                            if !assistant_response.is_empty() {
                                session.messages = messages;
                                session.messages.push(Message {
                                    role: "assistant".to_string(),
                                    content: assistant_response,
                                });
                                session.last_updated = chrono::Local::now();

                                if let Err(e) = save_session(&session) {
                                    if env::var("AI_VERBOSE").unwrap_or_default() == "true" {
                                        eprintln!(
                                            "{}",
                                            format!("[AI] Warning: Failed to save session: {}", e)
                                                .dimmed()
                                        );
                                    }
                                }
                            }

                            return Ok(());
                        }

                        // Parse JSON data with better error handling
                        match serde_json::from_str::<StreamResponse>(value) {
                            Ok(parsed) => {
                                if let Some(choices) = parsed.choices {
                                    for choice in choices {
                                        if let Some(delta) = choice.delta {
                                            // Process content
                                            if let Some(content) = delta.content {
                                                // Accumulate response for session
                                                assistant_response.push_str(&content);

                                                let formatted = code_buffer.append(&content);
                                                if !formatted.is_empty() {
                                                    print!("{}", formatted);

                                                    // Batch flushes for better performance
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
                                if env::var("AI_VERBOSE").unwrap_or_default() == "true" {
                                    eprintln!(
                                        "{}",
                                        format!("[AI] JSON parse error: {}", e).dimmed()
                                    );
                                }
                            }
                        }
                    }
                    "event" | "id" | "retry" => {
                        // Handle other SSE fields if needed in the future
                        if env::var("AI_VERBOSE").unwrap_or_default() == "true" {
                            eprintln!("{}", format!("[AI] SSE {}: {}", field, value).dimmed());
                        }
                    }
                    _ => {
                        // Unknown field
                        if env::var("AI_VERBOSE").unwrap_or_default() == "true" {
                            eprintln!("{}", format!("[AI] Unknown SSE field: {}", field).dimmed());
                        }
                    }
                }
            }
        }
    }

    // Process any remaining incomplete line
    if !incomplete_line.is_empty()
        && incomplete_line.trim() != ""
        && env::var("AI_VERBOSE").unwrap_or_default() == "true"
    {
        eprintln!(
            "{}",
            format!(
                "[AI] Warning: Incomplete SSE line at stream end: {}",
                incomplete_line
            )
            .dimmed()
        );
    }

    // Handle case where stream ends without [DONE]
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

    // Save session with assistant's response
    if !assistant_response.is_empty() {
        session.messages = messages;
        session.messages.push(Message {
            role: "assistant".to_string(),
            content: assistant_response,
        });
        session.last_updated = chrono::Local::now();

        if let Err(e) = save_session(&session) {
            if env::var("AI_VERBOSE").unwrap_or_default() == "true" {
                eprintln!(
                    "{}",
                    format!("[AI] Warning: Failed to save session: {}", e).dimmed()
                );
            }
        }
    }

    Ok(())
}
