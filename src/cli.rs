use clap::Parser;

#[derive(Parser, Debug)]
#[command(name = "ai")]
#[command(about = "AI command-line tool using OpenRouter API", long_about = None)]
pub struct Args {
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

    #[arg(
        long = "mcp-server",
        help = "Connect to MCP server (format: name:command:arg1,arg2,...)"
    )]
    pub mcp_servers: Vec<String>,

    #[arg(
        long = "use-tools",
        help = "Enable MCP tool usage in AI responses (deprecated, tools are now on by default)"
    )]
    pub use_tools: bool,

    #[arg(
        long = "auto-tools",
        help = "Automatically detect and use appropriate MCP tools (deprecated, this is now the default)"
    )]
    pub auto_tools: bool,

    #[arg(long = "no-tools", help = "Disable MCP tools for this query")]
    pub no_tools: bool,

    #[arg(
        long = "config-init",
        help = "Initialize a config file with example MCP servers"
    )]
    pub config_init: bool,

    #[arg(
        long = "api-endpoint",
        help = "Custom API base URL (e.g., http://localhost:11434/v1)"
    )]
    pub api_endpoint: Option<String>,

    #[arg(help = "Command to send to AI")]
    pub command: Vec<String>,
}
