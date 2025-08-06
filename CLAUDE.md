# CLAUDE.md

This file provides guidance to Claude Code (claude.ai/code) when working with code in this repository.

## Project Overview

cmd2ai is a Rust CLI tool that pipes terminal commands to AI models via the OpenRouter API, providing AI-powered command-line assistance with web search capabilities and syntax-highlighted code output.

## Development Commands

### Building the Project
```bash
# Debug build
cargo build

# Release build (optimized)
cargo build --release

# Run directly
cargo run -- "your prompt here"

# Run with web search
cargo run -- -s "latest news about Rust"
```

### Installation
```bash
# Build and install (default: ~/.local/bin)
make install

# Install to custom location
make install PREFIX=/usr/local

# Uninstall
make uninstall
```

### MCP Tool Usage
```bash
# Connect to MCP server and use tools
ai --mcp-server "name:command:arg1,arg2" --use-tools "Your request"

# Example with filesystem MCP server
ai --mcp-server "fs:npx:-y,@modelcontextprotocol/server-filesystem,/tmp" --use-tools "List files in /tmp"

# Example with Arc browser control
ai --mcp-server "arc:node:/path/to/arc-control/server/index.js" --use-tools "Open github.com"

# Multiple MCP servers
ai --mcp-server "fs:npx:-y,@modelcontextprotocol/server-filesystem,." \
   --mcp-server "time:npx:-y,@modelcontextprotocol/server-time" \
   --use-tools "What time is it and what files are here?"
```

### Code Quality
```bash
# Format code
cargo fmt

# Run clippy linter
cargo clippy

# Check for compilation errors
cargo check
```

## Architecture & Key Components

### Core Application Structure
The application implements:

1. **Command-line Interface** - Using clap with structured CLI arguments
2. **Streaming Response Handler** - Server-Sent Events (SSE) processing with custom code buffer
3. **Web Search Intelligence** - Automatic detection based on keywords with manual override flags
4. **Syntax Highlighting** - Real-time syntax highlighting for code blocks using syntect library
5. **Reasoning Token Support** - Display AI model's step-by-step reasoning process
6. **MCP Client** - Model Context Protocol client for tool integration

### Key Implementation Details

**Streaming Code Buffer**: The `CodeBuffer` struct handles incremental code block detection and syntax highlighting during SSE streaming. When modifying, ensure:
- Buffer state is preserved between chunks
- Code blocks are properly tracked with language detection
- Incomplete code blocks are handled gracefully
- Syntax highlighting themes are appropriate for terminal display

**Web Search Detection**: The `should_search()` function implements smart keyword detection. Search keywords include: "latest", "news", "current", "weather", "update", "price", "stock", "today". No-search keywords include development terms like "code", "function", "implement".

**API Integration**: Uses OpenRouter API with automatic model suffix handling (appends ":online" for web search). The streaming response parser handles SSE format with proper error recovery.

**Reasoning Token Support**: The `Reasoning` struct provides configuration for AI reasoning tokens:
- `effort`: Controls reasoning depth (high/medium/low)
- `max_tokens`: Sets specific token limit for reasoning
- `exclude`: Allows using reasoning internally without displaying it
- `enabled`: Enables reasoning with default parameters
Reasoning tokens are displayed in a distinct formatted block during streaming.

**MCP Client**: The `McpClient` in `src/mcp/` implements:
- JSON-RPC 2.0 protocol communication with MCP servers via stdio
- Dynamic tool discovery via `tools/list` requests
- Tool execution through `tools/call` with parameter validation
- Support for multiple concurrent MCP server connections
- Automatic server lifecycle management (initialization and shutdown)
- Non-streaming mode when tools are available for proper tool call handling
- Automatic parsing and execution of tool calls from AI model responses

### Environment Configuration
Required:
- `OPENROUTER_API_KEY` - API authentication

Optional:
- `AI_MODEL` - Default: "openai/gpt-4o-mini"
- `AI_SYSTEM_PROMPT` - Custom system instructions
- `AI_WEB_SEARCH_MAX_RESULTS` - Range: 1-10, default: 5
- `AI_VERBOSE` - Set to "true" for debug logging
- `AI_STREAM_TIMEOUT` - Timeout in seconds for streaming responses, default: 30
- `AI_REASONING_ENABLED` - Enable reasoning tokens ("true", "1", or "yes")
- `AI_REASONING_EFFORT` - Set reasoning effort level ("high", "medium", or "low")
- `AI_REASONING_MAX_TOKENS` - Maximum tokens for reasoning (numeric value)
- `AI_REASONING_EXCLUDE` - Use reasoning but exclude from output ("true", "1", or "yes")

Command-line arguments always take precedence over environment variables.

### Wrapper Scripts
- `ai` - Bash wrapper that auto-builds if binary is missing
- `ai-widget.zsh` - ZSH integration for capital letter command interception

## Development Patterns

When modifying this codebase:
1. Main application logic is in `src/main.rs`, with modular components in separate files
2. Preserve streaming behavior except when tool usage requires full responses
3. Test syntax highlighting with various programming languages and edge cases
4. Ensure color output works across different terminal emulators
5. Keep dependencies minimal - each addition should be justified
6. The syntect library provides robust syntax highlighting but adds to binary size
7. MCP tool calls automatically switch to non-streaming mode for proper handling

## Testing Approach

Currently no automated tests exist. When adding features:
1. Test streaming with slow connections using throttled API responses
2. Verify markdown rendering with complex documents
3. Test web search detection logic with various prompts
4. Ensure proper error handling for API failures and network issues
5. Test MCP server connections with various server implementations
6. Verify tool discovery and execution with different MCP servers
7. Test tool call parsing and error handling in non-streaming mode