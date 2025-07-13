# CLAUDE.md

This file provides guidance to Claude Code (claude.ai/code) when working with code in this repository.

## Project Overview

cmd2ai is a Rust CLI tool that pipes terminal commands to AI models via the OpenRouter API, providing AI-powered command-line assistance with web search capabilities and markdown rendering.

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
The entire application logic is contained in `src/main.rs` (455 lines), implementing:

1. **Command-line Interface** - Using clap with structured CLI arguments
2. **Streaming Response Handler** - Server-Sent Events (SSE) processing with custom markdown buffer
3. **Web Search Intelligence** - Automatic detection based on keywords with manual override flags
4. **Markdown Rendering** - Real-time markdown parsing with syntax highlighting during streaming

### Key Implementation Details

**Streaming Markdown Buffer**: The `MarkdownBuffer` struct handles incremental markdown parsing, crucial for maintaining formatting during SSE streaming. When modifying, ensure:
- Buffer state is preserved between chunks
- Code blocks are properly tracked
- Incomplete markdown elements are handled gracefully

**Web Search Detection**: The `should_search()` function implements smart keyword detection. Search keywords include: "latest", "news", "current", "weather", "update", "price", "stock", "today". No-search keywords include development terms like "code", "function", "implement".

**API Integration**: Uses OpenRouter API with automatic model suffix handling (appends ":online" for web search). The streaming response parser handles SSE format with proper error recovery.

### Environment Configuration
Required:
- `OPENROUTER_API_KEY` - API authentication

Optional:
- `AI_MODEL` - Default: "openai/gpt-4o-mini"
- `AI_SYSTEM_PROMPT` - Custom system instructions
- `AI_WEB_SEARCH_MAX_RESULTS` - Range: 1-10, default: 5
- `AI_VERBOSE` - Set to "true" for debug logging

### Wrapper Scripts
- `ai` - Bash wrapper that auto-builds if binary is missing
- `ai-widget.zsh` - ZSH integration for capital letter command interception

## Development Patterns

When modifying this codebase:
1. Maintain the single-file architecture unless complexity demands refactoring
2. Preserve streaming behavior - avoid buffering entire responses
3. Test markdown rendering with various edge cases (nested code blocks, incomplete elements)
4. Ensure color output works across different terminal emulators
5. Keep dependencies minimal - each addition should be justified

## Testing Approach

Currently no automated tests exist. When adding features:
1. Test streaming with slow connections using throttled API responses
2. Verify markdown rendering with complex documents
3. Test web search detection logic with various prompts
4. Ensure proper error handling for API failures and network issues