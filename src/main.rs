use clap::Parser;
use colored::*;
use futures::StreamExt;
use pulldown_cmark::{Event, Options, Parser as MarkdownParser, Tag, TagEnd};
use reqwest::header::{HeaderMap, HeaderValue, AUTHORIZATION, CONTENT_TYPE};
use serde::{Deserialize, Serialize};
use std::env;
use std::io::{self, Write};
use std::process;

const WEB_SEARCH_KEYWORDS: &[&str] = &[
    "latest", "recent", "current", "today", "yesterday", "news",
    "update", "price", "stock", "weather", "score", "result",
    "released", "announced", "trending", "happening", "now",
    "breaking", "2024", "2025", "this week", "this month",
    "real-time", "live", "status", "outage", "down"
];

const INFO_KEYWORDS: &[&str] = &[
    "what is", "who is", "where is", "when is", "how to",
    "tell me about", "explain", "define", "information about"
];

const NO_SEARCH_KEYWORDS: &[&str] = &[
    "hi", "hello", "hey", "thanks", "thank you", "bye", 
    "goodbye", "please", "help me write", "code", "implement",
    "fix", "debug", "create", "make", "build"
];

#[derive(Parser, Debug)]
#[command(name = "ai")]
#[command(about = "AI command-line tool using OpenRouter API", long_about = None)]
struct Args {
    #[arg(short = 's', long = "search", help = "Force web search")]
    force_search: bool,

    #[arg(short = 'n', long = "no-search", help = "Disable web search")]
    no_search: bool,

    #[arg(help = "Command to send to AI")]
    command: Vec<String>,
}

#[derive(Serialize)]
struct Message {
    role: String,
    content: String,
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

struct MarkdownBuffer {
    buffer: String,
    in_code_block: bool,
}

impl MarkdownBuffer {
    fn new() -> Self {
        Self {
            buffer: String::new(),
            in_code_block: false,
        }
    }

    fn append(&mut self, content: &str) -> String {
        self.buffer.push_str(content);
        
        let mut output = String::new();
        let mut last_processed_index = 0;
        
        let chars: Vec<char> = self.buffer.chars().collect();
        let mut i = 0;
        
        while i < chars.len() {
            if i + 2 < chars.len() && &chars[i..i+3] == &['`', '`', '`'] {
                if !self.in_code_block {
                    if i + 3 < chars.len() {
                        let remaining: String = chars[i+3..].iter().collect();
                        if remaining.contains('\n') {
                            self.in_code_block = true;
                            if i > last_processed_index {
                                let to_process: String = chars[last_processed_index..i].iter().collect();
                                output.push_str(&render_markdown(&to_process));
                            }
                            last_processed_index = i;
                        }
                    }
                } else {
                    if i + 3 < chars.len() && chars[i + 3] == '\n' {
                        self.in_code_block = false;
                        let code_content: String = chars[last_processed_index..i+4].iter().collect();
                        output.push_str(&render_markdown(&code_content));
                        last_processed_index = i + 4;
                        i += 3;
                    }
                }
            }
            i += 1;
        }
        
        if !self.in_code_block {
            let unprocessed: String = chars[last_processed_index..].iter().collect();
            let lines: Vec<&str> = unprocessed.split('\n').collect();
            
            for i in 0..lines.len() - 1 {
                output.push_str(&render_markdown(&format!("{}\n", lines[i])));
            }
            
            self.buffer = lines[lines.len() - 1].to_string();
        } else {
            self.buffer = chars[last_processed_index..].iter().collect();
        }
        
        output
    }

    fn flush(&mut self) -> String {
        if !self.buffer.is_empty() {
            let output = render_markdown(&self.buffer);
            self.buffer.clear();
            output
        } else {
            String::new()
        }
    }
}

fn render_markdown(text: &str) -> String {
    let mut options = Options::empty();
    options.insert(Options::ENABLE_STRIKETHROUGH);
    
    let parser = MarkdownParser::new_ext(text, options);
    let mut output = String::new();
    let mut in_code_block = false;
    let mut in_heading = false;
    let mut heading_level = 0;
    let mut in_link = false;
    let mut link_url = String::new();
    
    for event in parser {
        match event {
            Event::Start(tag) => match tag {
                Tag::Heading { level, .. } => {
                    in_heading = true;
                    heading_level = level as u8;
                }
                Tag::CodeBlock(_) => {
                    in_code_block = true;
                    output.push_str(&format!("{}", "```".yellow()));
                }
                Tag::Link { dest_url, .. } => {
                    in_link = true;
                    link_url = dest_url.to_string();
                }
                Tag::Emphasis => output.push_str(&format!("{}", "".italic())),
                Tag::Strong => output.push_str(&format!("{}", "".bold())),
                Tag::Strikethrough => output.push_str(&format!("{}", "".strikethrough())),
                Tag::BlockQuote(_) => output.push_str(&format!("{}", "> ".dimmed())),
                _ => {}
            },
            Event::End(tag) => match tag {
                TagEnd::Heading(_) => {
                    in_heading = false;
                }
                TagEnd::CodeBlock => {
                    in_code_block = false;
                    output.push_str(&format!("{}\n", "```".yellow()));
                }
                TagEnd::Link => {
                    if !link_url.is_empty() {
                        output.push_str(&format!(" ({})", link_url.blue().underline()));
                    }
                    in_link = false;
                    link_url.clear();
                }
                _ => {}
            },
            Event::Text(text) => {
                if in_code_block {
                    output.push_str(&format!("{}", text.yellow()));
                } else if in_heading {
                    let colored_text = match heading_level {
                        1 => text.magenta().underline().bold().to_string(),
                        _ => text.green().bold().to_string(),
                    };
                    output.push_str(&colored_text);
                } else if in_link {
                    output.push_str(&text.blue().underline().to_string());
                } else {
                    output.push_str(&text);
                }
            }
            Event::Code(code) => {
                output.push_str(&format!("{}", code.yellow()));
            }
            Event::SoftBreak | Event::HardBreak => {
                output.push('\n');
            }
            _ => {}
        }
    }
    
    output
}

fn should_use_web_search(command: &str, force_search: bool, no_search: bool) -> bool {
    if force_search {
        return true;
    }
    if no_search {
        return false;
    }
    
    let lower_command = command.to_lowercase();
    
    if NO_SEARCH_KEYWORDS.iter().any(|&keyword| lower_command.contains(keyword)) {
        if !WEB_SEARCH_KEYWORDS.iter().any(|&keyword| lower_command.contains(keyword)) {
            return false;
        }
    }
    
    if WEB_SEARCH_KEYWORDS.iter().any(|&keyword| lower_command.contains(keyword)) {
        return true;
    }
    
    if INFO_KEYWORDS.iter().any(|&keyword| lower_command.starts_with(keyword)) {
        return lower_command.contains("company") || 
               lower_command.contains("person") || 
               lower_command.contains("event") || 
               lower_command.contains("place") || 
               lower_command.contains("product");
    }
    
    false
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let args = Args::parse();
    
    if args.command.is_empty() {
        eprintln!("{}", "Usage: ai [--search|-s] [--no-search|-n] <command>".red());
        eprintln!("{}", "  --search, -s     Force web search".dimmed());
        eprintln!("{}", "  --no-search, -n  Disable web search".dimmed());
        process::exit(1);
    }
    
    let command = args.command.join(" ");
    
    let api_key = env::var("OPENROUTER_API_KEY").unwrap_or_else(|_| {
        eprintln!("{}", "Error: OPENROUTER_API_KEY environment variable not set".red());
        process::exit(1);
    });
    
    let model = env::var("AI_MODEL").unwrap_or_else(|_| "openai/gpt-4.1-mini".to_string());
    
    let use_web_search = should_use_web_search(&command, args.force_search, args.no_search);
    
    let final_model = if use_web_search && !model.contains(":online") {
        format!("{}:online", model)
    } else {
        model
    };
    
    let system_prompt = env::var("AI_SYSTEM_PROMPT").ok();
    
    let current_date = chrono::Local::now().format("%A, %B %d, %Y").to_string();
    
    let mut messages = vec![];
    
    let date_prompt = format!("Today's date is {}.", current_date);
    let system_content = if let Some(prompt) = system_prompt {
        format!("{}\n\n{}", date_prompt, prompt)
    } else {
        date_prompt
    };
    
    messages.push(Message {
        role: "system".to_string(),
        content: system_content,
    });
    
    messages.push(Message {
        role: "user".to_string(),
        content: command,
    });
    
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
        messages,
        stream: true,
        plugins,
    };
    
    if env::var("AI_VERBOSE").unwrap_or_default() == "true" {
        eprintln!("{}", format!("[AI] Using model: {}", final_model).dimmed());
        eprintln!("{}", format!("[AI] Web search: {}", if use_web_search { "enabled" } else { "disabled" }).dimmed());
        if use_web_search {
            eprintln!("{}", format!("[AI] Max search results: {}", 
                env::var("AI_WEB_SEARCH_MAX_RESULTS").unwrap_or_else(|_| "5".to_string())).dimmed());
        }
    }
    
    let mut headers = HeaderMap::new();
    headers.insert(AUTHORIZATION, HeaderValue::from_str(&format!("Bearer {}", api_key))?);
    headers.insert(CONTENT_TYPE, HeaderValue::from_static("application/json"));
    
    let client = reqwest::Client::new();
    let response = client
        .post("https://openrouter.ai/api/v1/chat/completions")
        .headers(headers)
        .json(&request_body)
        .send()
        .await?;
    
    if !response.status().is_success() {
        let status = response.status();
        let error_text = response.text().await?;
        eprintln!("{} HTTP error! status: {}, message: {}", "Error:".red(), status, error_text);
        process::exit(1);
    }
    
    let mut stream = response.bytes_stream();
    let mut buffer = String::new();
    let mut citations: Vec<Citation> = vec![];
    let mut markdown_buffer = MarkdownBuffer::new();
    
    while let Some(chunk) = stream.next().await {
        let chunk = chunk?;
        let text = String::from_utf8_lossy(&chunk);
        buffer.push_str(&text);
        
        while let Some(line_end) = buffer.find('\n') {
            let line = buffer[..line_end].trim().to_string();
            buffer = buffer[line_end + 1..].to_string();
            
            if line.starts_with("data: ") {
                let data = &line[6..];
                if data == "[DONE]" {
                    let remaining = markdown_buffer.flush();
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
                    return Ok(());
                }
                
                if let Ok(parsed) = serde_json::from_str::<StreamResponse>(data) {
                    if let Some(choices) = parsed.choices {
                        for choice in choices {
                            if let Some(delta) = choice.delta {
                                if let Some(content) = delta.content {
                                    let formatted = markdown_buffer.append(&content);
                                    if !formatted.is_empty() {
                                        print!("{}", formatted);
                                        io::stdout().flush()?;
                                    }
                                }
                                
                                if let Some(annotations) = delta.annotations {
                                    for annotation in annotations {
                                        if annotation.annotation_type == "url_citation" {
                                            if let Some(citation) = annotation.url_citation {
                                                if !citations.iter().any(|c| c.url == citation.url) {
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
            }
        }
    }
    
    Ok(())
}