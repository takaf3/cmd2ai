# CLAUDE.md

This file provides guidance to Claude Code (claude.ai/code) when working with code in this repository.

## Project Overview

cmd2ai is a Rust CLI tool that pipes terminal commands to AI models via the OpenRouter API, providing AI-powered command-line assistance with web search capabilities, syntax-highlighted code output, and MCP (Model Context Protocol) tool integration.

## Development Commands

### Building and Running
```bash
# Debug build (use --bin ai since there are multiple binaries)
cargo build --bin ai

# Release build (optimized)
cargo build --release

# Run directly with cargo
cargo run --bin ai -- "your prompt here"

# Run with MCP tools (auto-detection is now default)
cargo run --bin ai -- "What time is it?"

# Run without MCP tools
cargo run --bin ai -- --no-tools "What is 2+2?"

# Initialize MCP config file
cargo run --bin ai -- --config-init
```

### Installation
```bash
# Build and install to ~/.local/bin (includes ZSH widget)
make install

# Install to custom location
make install PREFIX=/usr/local

# Uninstall
make uninstall

# Development build
make dev

# Run tests (currently no tests exist)
make test
```

### Code Quality
```bash
# Format code
cargo fmt
make fmt

# Run clippy linter
cargo clippy
make lint

# Check for compilation errors
cargo check
make check
```

## Architecture & Key Components

### High-Level Architecture

The application follows a pipeline architecture for processing AI requests:

```
User Input → CLI Args → Config Loading → MCP Server Detection → API Request → Stream Processing → Output
                           ↓                    ↓
                    Session Management    Tool Discovery & Execution
```

**Key Architectural Decisions:**

1. **Streaming vs Non-Streaming**: The app automatically switches between streaming (for regular responses) and non-streaming (when MCP tools are available) modes. This is critical because tool calls require the complete response to parse properly.

2. **Two-Stage MCP Tool Selection**:
   - Stage 1: Keyword-based server selection (happens before AI sees anything)
   - Stage 2: AI-based tool selection (AI decides which specific tools to call)
   This provides both efficiency (only connecting to relevant servers) and intelligence (AI chooses appropriate tools).

3. **Configuration Hierarchy**: 
   - Local `.cmd2ai.json` overrides global `~/.config/cmd2ai/cmd2ai.json`
   - Command-line args override all config files
   - Environment variables provide defaults

### Core Components

**Main Flow (`src/main.rs`)**:
- Handles CLI argument parsing and special commands (--clear, --config-init)
- Manages MCP client lifecycle and server connections
- Orchestrates streaming vs non-streaming API calls
- Processes tool calls in a loop until completion

**Config System (`src/config.rs`)**:
- `Config`: Runtime configuration from env vars and CLI args
- `McpConfig`: MCP server definitions loaded from JSON files
- Automatic server detection based on keywords (now default behavior)
- Priority: CLI args > Local config > Global config > Env vars
- API endpoint normalization (auto-appends `/chat/completions`)

**MCP Client (`src/mcp/`)**:
- `client.rs`: Manages server processes and JSON-RPC communication
- `tools.rs`: Formats tools for LLM and parses tool calls from responses
- `types.rs`: Type definitions for MCP protocol
- Critical: Each server runs as a child process with stdio communication

**Streaming Handler (`src/highlight.rs`)**:
- `CodeBuffer`: Stateful processor for detecting and highlighting code blocks during streaming
- Handles partial code blocks across SSE chunks
- Applies syntax highlighting in real-time using syntect

**Session Management (`src/session.rs`)**:
- Maintains conversation history in `~/.ai_sessions/`
- Auto-continues conversations within 1-hour window
- Trims history to stay within token limits

### Critical Implementation Details

**Web Search via MCP Tools**:
- Web search is handled through MCP tools (like Gemini)
- Configure MCP servers in config file for web search capabilities
- The AI intelligently decides when to use search tools based on the query

**Custom API Endpoints**:
- Support for any OpenAI-compatible API endpoint
- Configurable via `--api-endpoint` CLI arg or `AI_API_ENDPOINT` env var
- Automatically appends `/chat/completions` to base URLs ending with `/v1`

**MCP Configuration (`config.example.json`)**:
- `auto_activate_keywords`: Must match with sufficient score (default 0.3 threshold)
- `enabled`: Server-level flag to disable without removing configuration
- Environment variables use `${VAR_NAME}` syntax for expansion

**Tool Call Processing**:
When tools are available, the main loop in `main.rs`:
1. Sends non-streaming request with tool definitions
2. Parses response for tool calls
3. Executes each tool via MCP client
4. Sends results back to AI
5. Repeats until no more tool calls

**Session Files**:
- Location: `~/.ai_sessions/session_*.json`
- Format: JSON with messages array and metadata
- Auto-cleanup: Files older than 30 days are deleted
- Token limit: Automatically trims older messages to stay under limits

### Configuration

Configuration can be set via JSON config files, environment variables, or command-line arguments. The priority order is:
**CLI args > Environment variables > JSON config > Defaults**

#### Config File Locations (priority order)
1. `.cmd2ai.json` (local project config)
2. `~/.config/cmd2ai/cmd2ai.json` (global user config)

#### Complete Configuration Structure
```json
{
  "api": {
    "endpoint": "https://openrouter.ai/api/v1",  // Custom API endpoint
    "stream_timeout": 30                          // Request timeout in seconds
  },
  "model": {
    "default_model": "openai/gpt-4o-mini",       // Default AI model
    "system_prompt": "Custom instructions"        // System prompt
  },
  "session": {
    "verbose": false                              // Enable debug logging
  },
  "reasoning": {
    "enabled": false,                             // Enable reasoning tokens
    "effort": "low",                              // "high", "medium", or "low"
    "max_tokens": 1000,                          // Max reasoning tokens
    "exclude": false                              // Hide reasoning output
  },
  "mcp": {
    "disable_tools": false,                       // Disable all MCP tools
    "settings": {
      "auto_detect": true,                        // Auto-detect tools
      "timeout": 30                                // MCP timeout
    },
    "servers": [                                  // MCP server configs
      {
        "name": "filesystem",
        "command": "npx",
        "args": ["-y", "@modelcontextprotocol/server-filesystem", "/tmp"],
        "auto_activate_keywords": ["file", "read", "write"],
        "description": "File system operations",
        "env": {"CUSTOM_VAR": "${ENV_VAR}"},     // Env var expansion
        "enabled": true
      }
    ],
    "tool_selection": {
      "max_servers": 3,                          // Max servers to activate
      "min_match_score": 0.3,                    // Min keyword match score
      "prompt_before_activation": false          // Prompt before using tools
    }
  }
}
```

#### Environment Variable Overrides

Required:
- `OPENROUTER_API_KEY` - API authentication (no JSON config for security)

Optional (override JSON config):
- `AI_API_ENDPOINT` - Custom API base URL
- `AI_MODEL` - Default model
- `AI_SYSTEM_PROMPT` - System instructions  
- `AI_VERBOSE` - Set to "true" for debug logging
- `AI_STREAM_TIMEOUT` - Timeout in seconds
- `AI_REASONING_ENABLED` - Enable reasoning
- `AI_REASONING_EFFORT` - Reasoning effort level
- `AI_REASONING_MAX_TOKENS` - Max reasoning tokens
- `AI_REASONING_EXCLUDE` - Hide reasoning output
- `AI_DISABLE_TOOLS` - Disable MCP tools

### Configuration Migration

A migration script is provided to convert existing environment variables to JSON configuration:

```bash
# Show current env vars and generate config (dry run)
./migrate_config.sh --dry-run

# Write config to default location (~/.config/cmd2ai/cmd2ai.json)
./migrate_config.sh

# Write to current directory (for project-specific config)
./migrate_config.sh --output .cmd2ai.json

# Merge with existing config
./migrate_config.sh --merge

# Force overwrite without prompting
./migrate_config.sh --force
```

The migration script will:
- Detect all AI_* environment variables
- Convert them to appropriate JSON config structure
- Preserve existing MCP server configurations when using --merge
- Keep sensitive data (API keys) as environment variables

### Common Debugging

```bash
# Enable verbose logging to see MCP server detection
AI_VERBOSE=true ai "your query"

# Test MCP server connection directly
ai --mcp-server "test:echo:hello" "test"

# Check which config file is being loaded
AI_VERBOSE=true ai "test" 2>&1 | grep "Available servers"

# Use custom API endpoint
ai --api-endpoint "http://localhost:11434/v1" "Hello world"
```