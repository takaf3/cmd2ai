# CLAUDE.md

This file provides guidance to Claude Code (claude.ai/code) when working with code in this repository.

## Project Overview

cmd2ai is a Rust CLI tool that pipes terminal commands to AI models via the OpenRouter API, providing AI-powered command-line assistance with syntax-highlighted code output and local tool integration.

## Development Commands

### Building and Running
```bash
# Debug build (use --bin ai since there are multiple binaries)
cargo build --bin ai

# Release build (optimized)
cargo build --release

# Run directly with cargo
cargo run --bin ai -- "your prompt here"

# Run with local tools (enabled by default)
cargo run --bin ai -- "What time is it?"

# Run without tools
cargo run --bin ai -- --no-tools "What is 2+2?"

# Initialize config file
cargo run --bin ai -- --config-init

# Debug streaming with raw SSE output (check-raw binary)
cargo run --bin check-raw -- "your prompt" [--reasoning]
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

**ZSH Widget**: The installation includes `ai-widget.zsh` which intercepts commands starting with capital letters and routes them to the AI. Add to `~/.zshrc`: `source ~/.config/zsh/functions/ai-widget.zsh`

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

### Project Structure

The codebase contains two binary targets:
- **`ai`** (`src/main.rs`): Main CLI application with full features
- **`check-raw`** (`src/bin/check_raw.rs`): Debugging tool that displays raw SSE streams from the API

### High-Level Architecture

The application follows a pipeline architecture for processing AI requests:

```
User Input → CLI Args → Config Loading → API Request → Stream Processing → Output
                           ↓                    ↓
                    Session Management    Tool Discovery & Execution
```

**Key Architectural Decisions:**

1. **Streaming vs Non-Streaming**: The app automatically switches between streaming (for regular responses) and non-streaming (when local tools are available) modes. This is critical because tool calls require the complete response to parse properly.

2. **Local Tool Integration**:
   - Tools are discovered from configuration
   - AI decides which specific tools to call based on the query
   - Tools execute locally within the application

3. **Configuration Hierarchy**:
   - Priority: CLI args > Env vars > Local config > Global config > Defaults
   - Supports both YAML (`.cmd2ai.yaml`) and JSON (`.cmd2ai.json`) formats
   - YAML is preferred for its inline comment support

### Core Components

**Main Flow (`src/main.rs`)**:
- Handles CLI argument parsing and special commands (--clear, --config-init)
- Manages local tool registry and execution
- Orchestrates streaming vs non-streaming API calls
- Processes tool calls in a loop until completion

**Config System (`src/config.rs`)**:
- `Config`: Runtime configuration from env vars and CLI args
- `LocalToolsConfig`: Local tool definitions loaded from YAML/JSON files
- Supports regex patterns for environment variable expansion (`${VAR_NAME}`)
- API endpoint normalization (auto-appends `/chat/completions`)

**Local Tools (`src/local_tools/`)**:
- `registry.rs`: Manages tool registration and discovery
- `tools.rs`: Formats tools for LLM and executes tool calls
- `executor.rs`: Executes custom script and command-based tools
- `dynamic.rs`: Creates dynamic tools from configuration

**Streaming Handler (`src/highlight.rs`)**:
- `CodeBuffer`: Stateful processor for detecting and highlighting code blocks during streaming
- Handles partial code blocks across SSE chunks
- Applies syntax highlighting in real-time using syntect

**Session Management (`src/session.rs`)**:
- Maintains conversation history in `~/.cache/cmd2ai/`
- Auto-continues conversations within 30-minute window
- Keeps last 3 exchanges (6 messages) to stay within token limits

### Critical Implementation Details

**Custom API Endpoints**:
- Support for any OpenAI-compatible API endpoint
- Configurable via `--api-endpoint` CLI arg or `AI_API_ENDPOINT` env var
- Automatically appends `/chat/completions` to base URLs ending with `/v1`

**Local Tool Configuration**:
- Tools are defined in YAML/JSON config files
- Supports built-in tools (echo, time_now, read_file, list_dir) and custom tools
- Custom tools can be script-based or command-based
- Environment variables use `${VAR_NAME}` syntax for expansion

**Tool Call Processing**:
When tools are available, the main loop in `main.rs`:
1. Sends non-streaming request with tool definitions
2. Parses response for tool calls
3. Executes each tool via local tool registry
4. Sends results back to AI
5. Repeats until no more tool calls

**Session Files**:
- Location: `~/.cache/cmd2ai/session-*.json`
- Format: JSON with messages array and metadata
- Auto-cleanup: Expired sessions (>30 minutes) are deleted when encountered
- Token management: Keeps last 3 exchanges (6 messages) automatically

### Configuration

Configuration can be set via YAML/JSON config files, environment variables, or command-line arguments. The priority order is:
**CLI args > Environment variables > Local config > Global config > Defaults**

#### Config File Locations (priority order)
1. `.cmd2ai.yaml` or `.cmd2ai.yml` (local project config)
2. `~/.config/cmd2ai/cmd2ai.yaml` or `.cmd2ai.yml` (global user config)
3. `.cmd2ai.json` (backward compatibility)

#### Complete Configuration Structure
```yaml
# cmd2ai Configuration
# YAML format supports inline comments!

# API Configuration
api:
  endpoint: https://openrouter.ai/api/v1  # Custom API endpoint
  stream_timeout: 30                       # Request timeout in seconds

# Model Configuration  
model:
  default_model: openai/gpt-5            # Default AI model
  system_prompt: Custom instructions       # System prompt

# Session Configuration
session:
  verbose: false                          # Enable debug logging

# Reasoning Configuration
reasoning:
  enabled: false                          # Enable reasoning tokens
  effort: low                             # high, medium, or low
  max_tokens: 1000                        # Max reasoning tokens
  exclude: false                          # Hide reasoning output

# Local Tools Configuration
local_tools:
  enabled: true                          # Enable local tools
  base_dir: ${HOME}                      # Base directory for file operations
  max_file_size_mb: 10                   # Max file size for read_file
  tools:                                 # Per-tool configuration
    - name: echo
      enabled: true
    - name: read_file
      enabled: true
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
- `AI_TOOLS_ENABLED` - Enable/disable all tools

### Configuration Migration

A migration script (`migrate_config.sh`) converts environment variables to YAML configuration:

```bash
# Show current env vars and generate config (dry run)
./migrate_config.sh --dry-run

# Write config to default location (~/.config/cmd2ai/cmd2ai.yaml)
./migrate_config.sh

# Write to current directory (for project-specific config)
./migrate_config.sh --output .cmd2ai.yaml

# Merge with existing config
./migrate_config.sh --merge

# Force overwrite without prompting
./migrate_config.sh --force
```

The migration script will:
- Detect all AI_* environment variables
- Convert them to appropriate YAML config structure with comments
- Preserve existing local tool configurations when using --merge
- Keep sensitive data (API keys) as environment variables

### Common Debugging

```bash
# Enable verbose logging
AI_VERBOSE=true ai "your query"

# Check which config file is being loaded
AI_VERBOSE=true ai "test" 2>&1 | grep "Available tools"

# Use custom API endpoint
ai --api-endpoint "http://localhost:11434/v1" "Hello world"
```