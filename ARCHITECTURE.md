# cmd2ai Architecture

This document describes the architecture and module structure of the cmd2ai codebase.

## Overview

cmd2ai is a Rust CLI tool that pipes terminal commands to AI models via the OpenRouter API, providing AI-powered command-line assistance with syntax-highlighted code output and local tool integration.

## Module Structure

```
src/
├── main.rs              # CLI entry point (minimal - ~250 lines)
├── lib.rs               # Public library exports
│
├── api/                  # API interaction layer
│   ├── mod.rs
│   ├── client.rs        # HTTP client for API requests
│   ├── streaming.rs     # Streaming response processing
│   ├── response.rs      # Non-streaming response helpers
│   └── models.rs        # API request/response types
│
├── orchestrator.rs       # Main execution orchestration
│
├── cli.rs                # CLI argument definitions
│
├── config.rs             # Configuration management
│
├── error.rs              # Unified error types
│
├── models.rs             # Data models (Message, Session, Reasoning, ToolCall)
│
├── session.rs           # Session management
│
├── ui/                   # User interface/output
│   ├── mod.rs
│   ├── highlight.rs     # Code syntax highlighting
│   └── output.rs        # Output formatting helpers
│
└── local_tools/          # Local tool execution
    ├── mod.rs
    ├── registry.rs      # Tool registry and validation
    ├── tools.rs         # Tool formatting and execution
    ├── executor.rs      # Command/script execution
    ├── dynamic.rs       # Dynamic tool creation
    └── paths.rs         # Safe path resolution
```

## Data Flow

### Main Execution Flow

1. **CLI Entry** (`main.rs`)
   - Parse CLI arguments
   - Handle special commands (`--clear`, `--config-init`)
   - Load configuration
   - Create session
   - Build message history
   - Call orchestrator

2. **Orchestration** (`orchestrator.rs`)
   - Determine streaming vs non-streaming mode
   - Collect and format tools
   - Make API request
   - Process response (streaming or non-streaming)
   - Handle tool execution loops
   - Return final response

3. **API Layer** (`api/`)
   - **client.rs**: HTTP request handling
   - **streaming.rs**: Process SSE streams, handle reasoning, citations
   - **response.rs**: Parse non-streaming responses, extract tool calls
   - **models.rs**: API request/response type definitions

4. **Tool Execution** (`local_tools/`)
   - **registry.rs**: Tool registration and discovery
   - **tools.rs**: Format tools for LLM, execute tool calls
   - **executor.rs**: Execute custom script and command-based tools
   - **dynamic.rs**: Create dynamic tools from configuration
   - **paths.rs**: Safe path resolution (prevents path traversal)

5. **UI Layer** (`ui/`)
   - **highlight.rs**: Syntax highlighting for code blocks
   - **output.rs**: Formatted output helpers (tool results, errors, reasoning)

6. **Session Management** (`session.rs`)
   - Load/create sessions
   - Trim conversation history
   - Save session state

## Key Design Decisions

### Streaming vs Non-Streaming

- **Streaming**: Used when no tools are available. Provides real-time output.
- **Non-Streaming**: Used when tools are available. Required because OpenRouter's streaming API doesn't properly stream tool call arguments.

### Tool Execution Loop

When tools are called:
1. Parse tool calls from response
2. Execute each tool locally
3. Add tool results to conversation
4. Make follow-up API request (with streaming enabled for final answer)
5. Display final response

### Error Handling

Unified error type (`Cmd2AiError`) with conversions from common error types:
- `reqwest::Error` → `NetworkError`
- `std::io::Error` → `IoError`
- `serde_json::Error` → `JsonError`
- `serde_yaml::Error` → `YamlError`

### Configuration Hierarchy

Priority order: CLI args > Environment variables > Local config > Global config > Defaults

Config files are searched in order:
1. `.cmd2ai.yaml` / `.cmd2ai.yml` (local project)
2. `~/.config/cmd2ai/cmd2ai.yaml` / `.cmd2ai.yml` (global)
3. `.cmd2ai.json` (backward compatibility)

## Security Considerations

### Path Resolution

- All file operations are restricted to `base_dir`
- Path traversal attacks are prevented via `safe_resolve_path()`
- Absolute paths are rejected unless explicitly allowed

### Tool Argument Validation

- Template arguments (`{{key}}`) are validated
- Path arguments are automatically restricted to `base_dir`
- Option injection prevention for path arguments
- Configurable validation policies via `template_validations`

## Testing Strategy

- Unit tests for individual modules
- Integration tests for full CLI flow
- Security tests for path validation
- Mock API responses for testing without network calls

## Future Improvements

- Connection pooling for HTTP client
- Caching for syntax highlighting themes
- Parallel tool execution (currently sequential)
- Additional built-in tools
- Session storage abstraction for testability

