use clap::Parser;

#[derive(Parser, Debug)]
#[command(name = "ai")]
#[command(about = "AI command-line tool using OpenRouter API", long_about = None)]
pub struct Args {
    #[arg(short = 's', long = "search", help = "Force web search")]
    pub force_search: bool,

    #[arg(long = "no-search", help = "Disable web search")]
    pub no_search: bool,

    #[arg(short = 'n', long = "new", help = "Start a new conversation")]
    pub new_conversation: bool,

    #[arg(
        short = 'c',
        long = "continue",
        help = "Continue previous conversation even if expired"
    )]
    pub force_continue: bool,

    #[arg(long = "clear", help = "Clear all conversation history")]
    pub clear_history: bool,

    #[arg(
        long = "reasoning-effort",
        help = "Set reasoning effort level (high, medium, low)"
    )]
    pub reasoning_effort: Option<String>,

    #[arg(
        long = "reasoning-max-tokens",
        help = "Set maximum tokens for reasoning"
    )]
    pub reasoning_max_tokens: Option<u32>,

    #[arg(
        long = "reasoning-exclude",
        help = "Use reasoning but exclude from response"
    )]
    pub reasoning_exclude: bool,

    #[arg(
        long = "reasoning-enabled",
        help = "Enable reasoning with default parameters"
    )]
    pub reasoning_enabled: bool,

    #[arg(help = "Command to send to AI")]
    pub command: Vec<String>,
}
