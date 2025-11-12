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
cargo build --release --bin ai

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

# Run a specific test
cargo test test_name

# Run all tests
cargo test

# Run tests with output
cargo test -- --nocapture
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

# Run tests
make test
```

**ZSH Widget**: The installation includes `ai-widget.zsh` which intercepts commands starting with capital letters and routes them to the AI. The widget:
- Intercepts Enter key presses and checks if the command starts with a capital letter
- Excludes environment variable assignments (e.g., `VAR=value command`)
- Wraps the command in `ai "command"` and executes it
- Install by adding to `~/.zshrc`: `source ~/.config/zsh/functions/ai-widget.zsh`

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

The source is organized into several modules:
- **`api/`**: API client, streaming response handling, and request/response models
- **`cli.rs`**: Command-line argument parsing using clap
- **`config/`**: Configuration management (YAML/JSON files, env vars, defaults)
- **`error.rs`**: Centralized error types and handling
- **`local_tools/`**: Local tool execution system with security sandboxing
  - `builtins/`: Built-in tools (read_file, list_dir, etc.)
  - `executor.rs`: Script and command execution with security validation
  - `paths.rs`: Path validation and canonicalization utilities
  - `dynamic.rs`: Dynamic tool creation from config
  - `registry.rs`: Tool discovery and management
- **`models/`**: Data structures for messages, sessions, tools, and reasoning
- **`orchestrator.rs`**: Main execution flow orchestrating API calls and tool execution
- **`session/`**: Session storage, filesystem operations, and conversation history
- **`ui/`**: Output formatting, syntax highlighting, and terminal display

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

**Orchestrator (`src/orchestrator.rs`)**:
- Main execution flow coordinating all components
- Manages the request-response cycle with tool call iterations
- Decides between streaming and non-streaming modes based on tool availability
- Collects and formats tools from registry for LLM consumption

**Config System (`src/config/mod.rs`)**:
- `Config`: Runtime configuration from env vars and CLI args
- `LocalToolsConfig`: Local tool definitions loaded from YAML/JSON files
- Supports regex patterns for environment variable expansion (`${VAR_NAME}`)
- API endpoint normalization (auto-appends `/chat/completions`)
- Hierarchical config resolution: CLI args > Env vars > Local config > Global config > Defaults

**API Module (`src/api/`)**:
- `client.rs`: HTTP client for making API requests
- `streaming.rs`: Server-Sent Events (SSE) stream processing
- `response.rs`: Response parsing, tool call extraction, and content extraction
- `models.rs`: API request/response data structures

**Local Tools (`src/local_tools/`)**:
- `registry.rs`: Manages tool registration and discovery
- `tools.rs`: Formats tools for LLM and executes tool calls
- `executor.rs`: Executes custom script and command-based tools with security validation
- `dynamic.rs`: Creates dynamic tools from configuration
- `paths.rs`: Safe path resolution utilities (prevents path traversal attacks)
- `builtins/`: Built-in tools like read_file with sandboxing

**UI Module (`src/ui/`)**:
- `highlight.rs`: `CodeBuffer` - Stateful processor for detecting and highlighting code blocks during streaming
- `output.rs`: Display functions for content, reasoning, tool results, and errors
- Handles partial code blocks across SSE chunks
- Applies syntax highlighting in real-time using syntect

**Session Management (`src/session/`)**:
- `storage.rs`: Session data structures and serialization
- `filesystem.rs`: File-based session persistence
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

**Security for Command Tools with Templated Arguments**:
- Argument templating (`{{key}}` syntax) is validated to prevent injection attacks
- Path arguments (auto-detected by name pattern `*path*`) are validated and restricted to `base_dir`
- Option injection prevention: values starting with `-` are rejected for path arguments
- Automatic `--` insertion before templated path arguments prevents option parsing
- Configurable validation policies via `template_validations` map in tool config
- See `src/local_tools/executor.rs::template_args()` for implementation details

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

**Note**: Run `ai --config-init` to create a `.cmd2ai.yaml` file with examples copied from `config.example.yaml`.

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

### Key Dependencies

The project uses these main crates:
- **tokio**: Async runtime for non-blocking I/O and concurrent execution
- **reqwest**: HTTP client with streaming support for SSE responses
- **serde/serde_json/serde_yaml**: (De)serialization for config files and API messages
- **clap**: Command-line argument parsing with derive macros
- **syntect**: Real-time syntax highlighting for code blocks in output
- **colored**: Terminal color formatting for UI elements
- **jsonschema**: JSON Schema validation for tool input parameters
- **anyhow**: Simplified error handling with context
- **regex**: Pattern matching for env var expansion and template validation
- **chrono**: Timestamp handling for session expiration
- **uuid**: Unique session identifiers
- **dirs**: Cross-platform path resolution (config, cache directories)

### Testing

The project includes unit tests in the `tests/` directory covering:
- **Path validation and security** (`tests/local_tools_template.rs`): Tests for path traversal prevention, option injection blocking, and template argument validation
- **API response parsing** (`tests/api_response.rs`): Tests for parsing streaming responses and tool calls
- **Session storage** (`tests/session_store.rs`): Tests for session persistence and history management
- **Tool execution** (`tests/tools.rs`): Tests for local tool execution and error handling

Key testing utilities:
- Uses `tempfile` crate for creating temporary directories in tests
- Tests validate security features like path canonicalization and base directory restrictions
- Integration tests cover the full tool execution flow from config to execution

### Common Debugging

```bash
# Enable verbose logging
AI_VERBOSE=true ai "your query"

# Check which config file is being loaded
AI_VERBOSE=true ai "test" 2>&1 | grep "Available tools"

# Use custom API endpoint
ai --api-endpoint "http://localhost:11434/v1" "Hello world"

# See detailed tool execution logs
AI_VERBOSE=true ai "Read the file test.txt"
```