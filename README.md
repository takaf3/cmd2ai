# cmd2ai

A fast command-line tool that pipes your terminal commands to AI models via the OpenRouter API.

## Features

- ✅ Streaming AI responses using Server-Sent Events (SSE)
- ✅ Syntax highlighting for code blocks
- ✅ Conversation memory with automatic continuation
- ✅ Support for custom models and system prompts
- ✅ Reasoning token support for enhanced AI model decision making
- ✅ MCP (Model Context Protocol) tool integration for extended AI capabilities
- ✅ Automatic MCP server detection based on query content
- ✅ Web search via MCP tools (e.g., Gemini) instead of built-in :online suffix
- ✅ Comprehensive JSON configuration with environment variable overrides
- ✅ Configuration migration tool for easy setup

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

Disable MCP tools for a specific query:
```bash
./ai --no-tools "What is 2+2?"
```

Note: Web search is now handled through MCP tools. Configure appropriate MCP servers (like Gemini) in your config file for web search capabilities.

### MCP Tool Integration

Connect to MCP servers (tools are enabled by default):
```bash
# Connect to filesystem MCP server
./ai --mcp-server "fs:npx:-y,@modelcontextprotocol/server-filesystem,/tmp" "List files in /tmp"

# Connect to multiple MCP servers
./ai --mcp-server "fs:npx:-y,@modelcontextprotocol/server-filesystem,/home" \
     --mcp-server "time:npx:-y,@modelcontextprotocol/server-time" \
     "What time is it and what files are in my home directory?"
```

MCP server format: `name:command:arg1,arg2,...`
- `name`: Server identifier (e.g., "fs", "time")
- `command`: Command to launch the server (e.g., "npx")
- `args`: Comma-separated arguments (optional)

### Configuration

cmd2ai supports comprehensive configuration through JSON files with environment variable overrides for debugging.

#### Quick Setup

1. Initialize a config file with examples:
```bash
./ai --config-init
```

2. Migrate existing environment variables to JSON config:
```bash
# Migrate to global config (~/.config/cmd2ai/cmd2ai.json)
./migrate_config.sh

# Or migrate to project-specific config
./migrate_config.sh --output .cmd2ai.json
```

#### Configuration File Structure

Config file locations (checked in priority order):
1. `.cmd2ai.json` (project-specific config)
2. `~/.config/cmd2ai/cmd2ai.json` (global user config)

Complete configuration example:
```json
{
  "api": {
    "endpoint": "https://openrouter.ai/api/v1",
    "stream_timeout": 30
  },
  "model": {
    "default_model": "openai/gpt-4o-mini",
    "system_prompt": "You are a helpful assistant"
  },
  "session": {
    "verbose": false
  },
  "reasoning": {
    "enabled": false,
    "effort": "low",
    "max_tokens": 1000,
    "exclude": false
  },
  "mcp": {
    "disable_tools": false,
    "settings": {
      "auto_detect": true,
      "timeout": 30
    },
    "servers": [
      {
        "name": "filesystem",
        "command": "npx",
        "args": ["-y", "@modelcontextprotocol/server-filesystem", "/tmp"],
        "auto_activate_keywords": ["file", "read", "write"],
        "description": "File system operations",
        "env": {"CUSTOM_VAR": "${ENV_VAR}"},
        "enabled": true
      }
    ],
    "tool_selection": {
      "max_servers": 3,
      "min_match_score": 0.3
    }
  }
}
```

#### Priority Order

Settings are resolved in this order (highest to lowest priority):
1. **Command-line arguments** (e.g., `--api-endpoint`, `--no-tools`)
2. **Environment variables** (e.g., `AI_MODEL`, `AI_VERBOSE`)
3. **JSON configuration files**
4. **Built-in defaults**

This allows you to:
- Set your preferred defaults in JSON config
- Override temporarily with environment variables for debugging
- Override for specific commands with CLI arguments

#### Migration Tool

The `migrate_config.sh` script helps convert existing environment variables to JSON:

```bash
# Show what would be migrated (dry run)
./migrate_config.sh --dry-run

# Migrate and merge with existing config
./migrate_config.sh --merge

# Force overwrite without prompting
./migrate_config.sh --force
```

The migration tool:
- Detects all AI_* environment variables
- Preserves existing MCP server configurations when merging
- Creates the config directory if needed
- Keeps sensitive data (API keys) as environment variables

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

**Required:**
- `OPENROUTER_API_KEY` - Your OpenRouter API key (kept as env var for security)

**Optional (can be set in JSON config or as env var overrides):**
- `AI_API_ENDPOINT` - Custom API base URL
- `AI_MODEL` - AI model to use
- `AI_SYSTEM_PROMPT` - System prompt to prepend to messages
- `AI_STREAM_TIMEOUT` - Timeout in seconds for streaming responses
- `AI_VERBOSE` - Enable debug logging ("true")
- `AI_REASONING_ENABLED` - Enable reasoning tokens ("true", "1", or "yes")
- `AI_REASONING_EFFORT` - Set reasoning effort level ("high", "medium", or "low")
- `AI_REASONING_MAX_TOKENS` - Maximum tokens for reasoning
- `AI_REASONING_EXCLUDE` - Use reasoning but exclude from output ("true", "1", or "yes")
- `AI_DISABLE_TOOLS` - Disable MCP tools ("true", "1", or "yes")

**Note:** All settings except the API key can be configured in JSON files. Environment variables override JSON config values, which is useful for temporary changes or debugging.

## Security Considerations

- **API Key Protection**: The application uses environment variables for API key storage, which is best practice. However, be aware that enabling verbose HTTP logging (not currently implemented) could potentially expose the Authorization header containing your API key.

## Command-Line Options

- `-n, --new` - Start a new conversation
- `-c, --continue` - Continue previous conversation even if expired
- `--clear` - Clear all conversation history
- `--api-endpoint` - Custom API base URL (e.g., http://localhost:11434/v1)
- `--mcp-server` - Connect to MCP server (format: name:command:arg1,arg2,...)
- `--no-tools` - Disable MCP tools for this query
- `--config-init` - Initialize a config file with example MCP servers
- `--reasoning-effort` - Set reasoning effort level (high, medium, low)
- `--reasoning-max-tokens` - Set maximum tokens for reasoning
- `--reasoning-exclude` - Use reasoning but exclude from response
- `--reasoning-enabled` - Enable reasoning with default parameters
- `-h, --help` - Print help information

## Using Custom API Endpoints

You can use cmd2ai with any OpenAI-compatible API endpoint (including local LLMs, OpenAI, or other providers). You only need to provide the base URL up to `/v1`, and `/chat/completions` will be appended automatically.

Using command-line argument:
```bash
# Use a local LLM server (Ollama)
./ai --api-endpoint "http://localhost:11434/v1" "Hello world"

# Use OpenAI directly
./ai --api-endpoint "https://api.openai.com/v1" "Explain quantum computing"

# Use a local AI server
./ai --api-endpoint "http://localhost:8080/v1" "What is the weather?"
```

Using environment variable:
```bash
# Set custom endpoint for all commands
export AI_API_ENDPOINT="http://localhost:11434/v1"
./ai "What is the weather today?"
```

The endpoint URL is automatically normalized:
- If you provide just the base URL (e.g., `http://localhost:8080`), it appends `/v1/chat/completions`
- If you provide up to `/v1`, it appends `/chat/completions`
- If you provide the full path including `/chat/completions`, it uses it as-is

The priority order is:
1. Command-line argument (`--api-endpoint`)
2. Environment variable (`AI_API_ENDPOINT`)
3. Default OpenRouter endpoint

## Web Search via MCP Tools

Web search is now handled through MCP tools (like Gemini) instead of the built-in `:online` model suffix. 
Configure MCP servers in your config file to enable intelligent web search capabilities that can be automatically 
activated based on your query content.

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