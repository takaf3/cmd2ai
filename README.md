# cmd2ai

A fast command-line tool that pipes your terminal commands to AI models via the OpenRouter API.

## Features

- ✅ Streaming AI responses using Server-Sent Events (SSE)
- ✅ Syntax highlighting for code blocks
- ✅ Conversation memory with automatic continuation
- ✅ Automatic web search detection
- ✅ Manual web search control with flags
- ✅ Support for custom models and system prompts
- ✅ Clean citation display for web search results
- ✅ Reasoning token support for enhanced AI model decision making
- ✅ MCP (Model Context Protocol) tool integration for extended AI capabilities

## Prerequisites

- Rust 1.70 or higher
- An OpenRouter API key

## Installation

### Using Make (Recommended)

1. Clone the repository:
```bash
git clone https://github.com/takaf3/cmd2ai.git
cd cmd2ai
```

2. Build and install:
```bash
make install
```

This will:
- Build the optimized release binary
- Install the `ai` command to `~/.local/bin`
- Install the ZSH widget to `~/.config/zsh/functions`

To install to a custom location:
```bash
make install PREFIX=/usr/local
```

3. Set your OpenRouter API key:
```bash
export OPENROUTER_API_KEY="your-api-key-here"
```

4. For ZSH users, add to your `~/.zshrc`:
```bash
source ~/.config/zsh/functions/ai-widget.zsh
```

To uninstall:
```bash
make uninstall
```

### Manual Installation

1. Clone the repository:
```bash
git clone https://github.com/takaf3/cmd2ai.git
cd cmd2ai
```

2. Build the project:
```bash
cargo build --release
```

3. Set your OpenRouter API key:
```bash
export OPENROUTER_API_KEY="your-api-key-here"
```

## Usage

Basic usage:
```bash
./ai "your prompt here"
```

### Conversation Memory

The tool automatically maintains conversation context for follow-up questions:

```bash
./ai "What is the capital of France?"
# Output: The capital of France is Paris.

./ai "Tell me more about it"
# The AI remembers the previous context and provides details about Paris
```

Start a new conversation:
```bash
./ai --new "Different topic here"
```

Continue an expired conversation:
```bash
./ai --continue "Follow up on our last discussion"
```

Clear all conversation history:
```bash
./ai --clear
```

### Web Search

Force web search:
```bash
./ai --search "What's the latest news?"
```

Disable web search:
```bash
./ai --no-search "What is 2+2?"
```

### MCP Tool Integration

Connect to MCP servers and enable tool usage:
```bash
# Connect to filesystem MCP server
./ai --mcp-server "fs:npx:-y,@modelcontextprotocol/server-filesystem,/tmp" --use-tools "List files in /tmp"

# Connect to multiple MCP servers
./ai --mcp-server "fs:npx:-y,@modelcontextprotocol/server-filesystem,/home" \
     --mcp-server "time:npx:-y,@modelcontextprotocol/server-time" \
     --use-tools "What time is it and what files are in my home directory?"
```

MCP server format: `name:command:arg1,arg2,...`
- `name`: Server identifier (e.g., "fs", "time")
- `command`: Command to launch the server (e.g., "npx")
- `args`: Comma-separated arguments (optional)

### Reasoning Tokens

For models that support it, you can enable reasoning tokens to see the AI's step-by-step thinking process:

Enable reasoning with default parameters:
```bash
./ai --reasoning-enabled "How would you build the world's tallest skyscraper?"
```

Set specific reasoning effort level:
```bash
./ai --reasoning-effort high "Explain quantum computing in detail"
```

Set maximum tokens for reasoning:
```bash
./ai --reasoning-max-tokens 2000 "What's the most efficient sorting algorithm?"
```

Use reasoning internally but exclude from output:
```bash
./ai --reasoning-effort medium --reasoning-exclude "Solve this complex problem"
```

Combine multiple reasoning options:
```bash
./ai --reasoning-effort high --reasoning-max-tokens 3000 "Design a distributed system"
```

#### Using Environment Variables

For convenience, especially with the ZSH widget, you can set reasoning options via environment variables:

```bash
# Enable reasoning by default
export AI_REASONING_ENABLED=true

# Set default reasoning effort level
export AI_REASONING_EFFORT=high

# Set default max tokens for reasoning
export AI_REASONING_MAX_TOKENS=2000

# Exclude reasoning output by default
export AI_REASONING_EXCLUDE=true
```

Command-line arguments always take precedence over environment variables, allowing you to override defaults on a per-command basis.

## Environment Variables

- `OPENROUTER_API_KEY` - Required. Your OpenRouter API key
- `AI_MODEL` - Optional. AI model to use (default: "openai/gpt-4.1-mini")
- `AI_SYSTEM_PROMPT` - Optional. System prompt to prepend to messages
- `AI_WEB_SEARCH_MAX_RESULTS` - Optional. Maximum web search results (default: 5, range: 1-10)
- `AI_STREAM_TIMEOUT` - Optional. Timeout in seconds for streaming responses (default: 30)
- `AI_VERBOSE` - Optional. Enable debug logging when set to "true"
- `AI_REASONING_ENABLED` - Optional. Enable reasoning tokens ("true", "1", or "yes")
- `AI_REASONING_EFFORT` - Optional. Set reasoning effort level ("high", "medium", or "low")
- `AI_REASONING_MAX_TOKENS` - Optional. Maximum tokens for reasoning (numeric value)
- `AI_REASONING_EXCLUDE` - Optional. Use reasoning but exclude from output ("true", "1", or "yes")

## Security Considerations

- **API Key Protection**: The application uses environment variables for API key storage, which is best practice. However, be aware that enabling verbose HTTP logging (not currently implemented) could potentially expose the Authorization header containing your API key.

## Command-Line Options

- `-s, --search` - Force web search
- `--no-search` - Disable web search
- `-n, --new` - Start a new conversation
- `-c, --continue` - Continue previous conversation even if expired
- `--clear` - Clear all conversation history
- `--mcp-server` - Connect to MCP server (format: name:command:arg1,arg2,...)
- `--use-tools` - Enable MCP tool usage in AI responses
- `--reasoning-effort` - Set reasoning effort level (high, medium, low)
- `--reasoning-max-tokens` - Set maximum tokens for reasoning
- `--reasoning-exclude` - Use reasoning but exclude from response
- `--reasoning-enabled` - Enable reasoning with default parameters
- `-h, --help` - Print help information

## Web Search Detection

The tool automatically detects when web search might be beneficial based on keywords:
- Web search triggers: "latest", "news", "current", "weather", "price", etc.
- No-search keywords: "hello", "code", "implement", "debug", etc.
- Informational queries are evaluated based on context

## Conversation Memory

Conversations are automatically saved and can be continued within 30 minutes:

- Sessions are stored in `~/.cache/cmd2ai/` as JSON files
- Each session maintains the last 3 exchanges (6 messages) for context
- Sessions automatically expire after 30 minutes of inactivity
- Expired sessions are cleaned up automatically

## Dependencies

This Rust implementation uses:
- `clap` - Command-line argument parsing
- `tokio` - Async runtime
- `reqwest` - HTTP client with streaming support
- `serde` - JSON serialization/deserialization
- `colored` - Terminal colors
- `syntect` - Syntax highlighting for code blocks
- `chrono` - Date/time handling
- `uuid` - Unique session identifiers

## Development

The project includes a Makefile with several helpful commands:

```bash
make          # Build release binary (default)
make install  # Build and install binary and ZSH widget
make uninstall # Remove installed files
make clean    # Clean build artifacts
make dev      # Build debug binary
make test     # Run tests
make fmt      # Format code
make lint     # Run clippy linter
make check    # Check compilation
```