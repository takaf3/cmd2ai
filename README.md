# cmd2ai

A fast command-line tool that pipes your terminal commands to AI models via the OpenRouter API.

## Features

- ✅ Streaming AI responses using Server-Sent Events (SSE)
- ✅ Markdown rendering with terminal colors
- ✅ Automatic web search detection
- ✅ Manual web search control with flags
- ✅ Support for custom models and system prompts
- ✅ Clean citation display for web search results

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

Force web search:
```bash
./ai --search "What's the latest news?"
```

Disable web search:
```bash
./ai --no-search "What is 2+2?"
```

## Environment Variables

- `OPENROUTER_API_KEY` - Required. Your OpenRouter API key
- `AI_MODEL` - Optional. AI model to use (default: "openai/gpt-4.1-mini")
- `AI_SYSTEM_PROMPT` - Optional. System prompt to prepend to messages
- `AI_WEB_SEARCH_MAX_RESULTS` - Optional. Maximum web search results (default: 5, range: 1-10)
- `AI_VERBOSE` - Optional. Enable debug logging when set to "true"

## Security Considerations

- **API Key Protection**: The application uses environment variables for API key storage, which is best practice. However, be aware that enabling verbose HTTP logging (not currently implemented) could potentially expose the Authorization header containing your API key.

## Command-Line Options

- `-s, --search` - Force web search
- `-n, --no-search` - Disable web search
- `-h, --help` - Print help information

## Web Search Detection

The tool automatically detects when web search might be beneficial based on keywords:
- Web search triggers: "latest", "news", "current", "weather", "price", etc.
- No-search keywords: "hello", "code", "implement", "debug", etc.
- Informational queries are evaluated based on context

## Dependencies

This Rust implementation uses:
- `clap` - Command-line argument parsing
- `tokio` - Async runtime
- `reqwest` - HTTP client with streaming support
- `serde` - JSON serialization/deserialization
- `colored` - Terminal colors
- `pulldown-cmark` - Markdown parsing
- `chrono` - Date/time handling

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

## Performance

The Rust version offers improved performance compared to the TypeScript version:
- Faster startup time (no Node.js/tsx overhead)
- Lower memory usage
- Native binary execution