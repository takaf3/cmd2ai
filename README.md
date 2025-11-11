# cmd2ai

A fast command-line tool that pipes your terminal commands to AI models via the OpenRouter API.

## Features

- ✅ Streaming AI responses using Server-Sent Events (SSE)
- ✅ Syntax highlighting for code blocks
- ✅ Conversation memory with automatic continuation
- ✅ Support for custom models and system prompts
- ✅ Reasoning token support for enhanced AI model decision making
- ✅ Local tool integration for extended AI capabilities
- ✅ Comprehensive JSON/YAML configuration with environment variable overrides
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

### Tools

Disable tools for a specific query:
```bash
./ai --no-tools "What is 2+2?"
```

### Configuration

cmd2ai supports comprehensive configuration through YAML files with environment variable overrides for debugging. YAML format allows inline comments for better documentation.

#### Quick Setup

1. Initialize a config file with examples:
```bash
./ai --config-init
```

2. Migrate existing environment variables to YAML config:
```bash
# Migrate to global config (~/.config/cmd2ai/cmd2ai.yaml)
./migrate_config.sh

# Or migrate to project-specific config
./migrate_config.sh --output .cmd2ai.yaml
```

#### Configuration File Structure

Config file locations (checked in priority order):
1. `.cmd2ai.yaml` or `.cmd2ai.yml` (project-specific config)
2. `~/.config/cmd2ai/cmd2ai.yaml` or `.cmd2ai.yml` (global user config)
3. `.cmd2ai.json` (backward compatibility)

Complete configuration example:
```yaml
# cmd2ai Configuration File
# YAML format supports comments for better documentation!

# API Configuration
api:
  endpoint: https://openrouter.ai/api/v1  # Custom API endpoint
  stream_timeout: 30                       # Request timeout in seconds

# Model Configuration
model:
  default_model: openai/gpt-5            # Default AI model
  system_prompt: You are a helpful assistant  # System instructions

# Session Configuration
session:
  verbose: false                          # Enable debug logging

# Reasoning Configuration
reasoning:
  enabled: false                          # Enable reasoning tokens
  effort: low                             # high, medium, or low
  max_tokens: 1000                        # Max reasoning tokens
  exclude: false                          # Hide reasoning output

# Global Tools Configuration
tools:
  enabled: true                           # Enable/disable all tools

# Local Tools Configuration
local_tools:
  enabled: true                           # Enable local tools
  base_dir: ${HOME}                       # Base directory for file operations
  max_file_size_mb: 10                    # Max file size for read_file (MB)
  tools:                                  # Per-tool configuration (optional)
    - name: echo
      enabled: true
    - name: time_now
      enabled: true
    - name: read_file
      enabled: true
    - name: list_dir
      enabled: true
```

#### Priority Order

Settings are resolved in this order (highest to lowest priority):
1. **Command-line arguments** (e.g., `--api-endpoint`, `--no-tools`)
2. **Environment variables** (e.g., `AI_MODEL`, `AI_VERBOSE`)
3. **YAML/JSON configuration files**
4. **Built-in defaults**

This allows you to:
- Set your preferred defaults in YAML config
- Override temporarily with environment variables for debugging
- Override for specific commands with CLI arguments

#### Migration Tool

The `migrate_config.sh` script helps convert existing environment variables to YAML:

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
- Converts them to well-formatted YAML with comments
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
- `AI_TOOLS_ENABLED` - Enable/disable all tools ("true", "1", or "yes")

**Note:** All settings except the API key can be configured in YAML files. Environment variables override YAML config values, which is useful for temporary changes or debugging. The system also supports JSON files for backward compatibility.

## Security Considerations

- **API Key Protection**: The application uses environment variables for API key storage, which is best practice. However, be aware that enabling verbose HTTP logging (not currently implemented) could potentially expose the Authorization header containing your API key.

## Command-Line Options

- `-n, --new` - Start a new conversation
- `-c, --continue` - Continue previous conversation even if expired
- `--clear` - Clear all conversation history
- `--api-endpoint` - Custom API base URL (e.g., http://localhost:11434/v1)
- `--no-tools` - Disable all tools for this query
- `--config-init` - Initialize a config file with example local tools
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

## Web Search via Custom Tools

Web search can be added via custom local tools configured in your config file. Create a custom tool that interfaces with a web search API or service.

## Local Tools

cmd2ai includes built-in local tools that run directly in the application (no external processes). These tools are enabled by default when configured.

### Available Local Tools

- **`read_file`** - Read and return the contents of a file. Limited to files within the base directory and under the size limit.

### Configuration

Local tools are configured in your YAML config file:

```yaml
# Global tools toggle (affects both local and MCP tools)
tools:
  enabled: true

# Local tools configuration
local_tools:
  enabled: true                    # Enable local tools
  base_dir: ${HOME}               # Base directory for file operations (defaults to $HOME)
  max_file_size_mb: 10            # Maximum file size for read_file (default: 10MB)
  
  # Per-tool configuration (optional)
  # If a tool is not listed here, it defaults to enabled
  tools:
    - name: read_file
      enabled: true
```

### Creating Custom Tools

You can create your own custom tools directly in the config file without modifying code. Custom tools can be either **script-based** (inline or from file) or **command-based** (external executables).

#### Script-Based Tools

Script tools run code using an interpreter (Python, Node.js, Bash, etc.):

```yaml
local_tools:
  tools:
    # Inline Python script
    - name: upper
      enabled: true
      type: script
      description: "Convert text to uppercase"
      interpreter: python3
      script: |
        import sys, json
        data = json.load(sys.stdin)
        text = data.get("text", "")
        print(text.upper())
      input_schema:
        type: object
        properties:
          text:
            type: string
            description: "Text to convert to uppercase"
        required: [text]
        additionalProperties: false
      timeout_secs: 10
      max_output_bytes: 1048576  # 1MB
    
    # Script from file (relative to base_dir)
    - name: my_script_tool
      enabled: true
      type: script
      description: "Run a custom script from file"
      interpreter: python3
      script_path: scripts/my_tool.py
      input_schema:
        type: object
        properties:
          input:
            type: string
        required: [input]
      timeout_secs: 30
      working_dir: scripts  # Optional: change working directory
      env:  # Optional: environment variables (supports ${VAR} expansion)
        CUSTOM_VAR: ${HOME}/custom
```

#### Command-Based Tools

Command tools execute external programs. You can use argument templating with `{{key}}` syntax to inject tool arguments into command-line arguments:

```yaml
local_tools:
  tools:
    - name: word_count
      enabled: true
      type: command
      description: "Count words in text using wc command"
      command: wc
      args: ["-w"]
      input_schema:
        type: object
        properties:
          text:
            type: string
            description: "Text to count words in"
        required: [text]
        additionalProperties: false
      timeout_secs: 10
      max_output_bytes: 262144  # 256KB
    
    # Example with argument templating
    - name: list_directory
      enabled: true
      type: command
      description: "List files in a directory using ls"
      command: ls
      args: ["-la", "{{path}}"]  # {{path}} will be replaced with the 'path' argument
      input_schema:
        type: object
        properties:
          path:
            type: string
            description: "Path to the directory to list (relative to base_dir)"
        required: [path]
        additionalProperties: false
      timeout_secs: 10
      max_output_bytes: 262144
```

#### Tool Configuration Fields

**Required fields:**
- `name` - Unique tool name
- `type` - Either `"script"` or `"command"`
- `description` - Tool description shown to the AI
- `input_schema` - JSON Schema defining tool parameters
- `enabled` - Enable/disable the tool

**For script tools:**
- `interpreter` - Interpreter command (e.g., `python3`, `node`, `bash`)
- `script` OR `script_path` - Inline script content or path to script file

**For command tools:**
- `command` - Command to execute
- `args` - Optional command arguments (array of strings, supports `{{key}}` templating)

**Optional fields (both types):**
- `timeout_secs` - Execution timeout in seconds (default: 30)
- `max_output_bytes` - Maximum output size in bytes (default: 1MB)
- `working_dir` - Working directory relative to `base_dir`
- `env` - Environment variables (supports `${VAR}` expansion)

**Optional fields (command tools only):**
- `stdin_json` - Whether to send tool arguments as JSON via stdin (default: `true`). Set to `false` if the command doesn't read from stdin and you're using argument templating.

#### Argument Templating for Command Tools

Command tools support argument templating using `{{key}}` syntax. This allows you to inject tool arguments directly into command-line arguments:

```yaml
- name: list_directory
  type: command
  command: ls
  args: ["-la", "{{path}}"]  # {{path}} is replaced with the 'path' argument value
  stdin_json: false  # Disable stdin since ls doesn't read from stdin
  input_schema:
    type: object
    properties:
      path:
        type: string
    required: [path]
```

When the tool is called with `{"path": "Documents"}`, the command executed will be: `ls -la Documents`

**Note**: If `stdin_json` is `false`, the tool arguments are only available via templated args. If `true` (default), arguments are sent both via stdin (as JSON) and can be templated into args.

#### How Custom Tools Work

1. **Input**: 
   - For scripts: Tool arguments are sent as JSON on stdin (always)
   - For commands: Tool arguments can be templated into `args` using `{{key}}` syntax, and optionally sent as JSON on stdin (controlled by `stdin_json`)
2. **Output**: Tool output (stdout) is captured and returned as the tool result
3. **Validation**: Arguments are validated against the `input_schema` before execution
4. **Security**: All paths are restricted to `base_dir`, timeouts and output limits are enforced

#### Example: Reading JSON from stdin

Your script should read JSON from stdin:

```python
import sys, json
data = json.load(sys.stdin)
# Process data
result = process(data)
print(result)  # Output becomes tool result
```

### Security Considerations

- **Base Directory**: All file operations are restricted to the configured `base_dir` (defaults to `$HOME`). Path traversal attacks are prevented.
- **File Size Limits**: The `read_file` tool enforces a maximum file size (default 10MB) to prevent reading huge files.
- **Path Validation**: All paths are validated and normalized to ensure they stay within the base directory.
- **Execution Limits**: Custom tools have configurable timeouts and output size limits to prevent runaway processes.
- **Sandboxing**: Script paths and working directories are restricted to `base_dir`.

#### Security with Templated Command Arguments

When using argument templating in command-based tools (e.g., `args: ["-la", "{{path}}"]`), cmd2ai implements several security measures to prevent argument injection and path traversal attacks:

**Automatic Path Validation:**
- Arguments with names matching `*path*` (case-insensitive) are automatically treated as paths
- Path arguments are validated and canonicalized to ensure they stay within `base_dir`
- Absolute paths are rejected by default (unless explicitly allowed)
- Path traversal attempts (e.g., `../../../etc/passwd`) are blocked

**Option Injection Prevention:**
- Values starting with `-` are rejected for path arguments (prevents injecting command-line options)
- The `--` separator is automatically inserted before templated path arguments to prevent option parsing
- This ensures that even if an attacker tries to inject `--help` or similar, it's treated as a filename

**Explicit Validation Policies:**
You can configure explicit validation policies for templated arguments:

```yaml
local_tools:
  tools:
    - name: list_directory
      type: command
      command: ls
      args: ["-la", "{{path}}"]
      # Security settings (all have secure defaults)
      restrict_to_base_dir: true  # Restrict paths to base_dir (default: true)
      insert_double_dash: true     # Insert "--" before templated args (default: auto-detect)
      template_validations:
        path:
          kind: path               # Explicitly mark as path
          allow_absolute: false    # Reject absolute paths (default: false)
          deny_patterns: ["\\.\\./"]  # Optional: deny specific patterns
```

**When to Disable Security Features:**
- Setting `restrict_to_base_dir: false` disables path validation (not recommended)
- Setting `insert_double_dash: false` disables option injection prevention (not recommended)
- Only disable these features if you fully understand the security implications and trust all tool call arguments

**Best Practices:**
1. Always use relative paths in tool arguments (they'll be resolved relative to `base_dir`)
2. Use explicit `template_validations` for non-path arguments that need pattern matching
3. Keep `base_dir` restricted to a safe directory (default `$HOME` is reasonable)
4. Review tool configurations before enabling them in production
5. Use verbose mode (`AI_VERBOSE=true`) to audit tool calls during development

### Disabling Tools

You can disable tools at multiple levels:

```bash
# Disable all tools
ai --no-tools "What is 2+2?"
```

Or via config:

```yaml
# Disable all tools globally
tools:
  enabled: false

# Disable only local tools
local_tools:
  enabled: false
```

### Debugging Local Tools

When troubleshooting tool execution issues, enable verbose logging to see detailed information about tool calls:

```bash
# Enable verbose mode
AI_VERBOSE=true ai "List files in Documents"
```

Or set it in your config:

```yaml
session:
  verbose: true
```

With verbose mode enabled, you'll see detailed debug output including:

- **Tool Registration**: Which tools are being registered and their configuration
- **Tool Selection**: Which tool the AI selected and the arguments it's using
- **Argument Validation**: Whether arguments passed validation against the schema
- **Path Resolution**: How user-provided paths are resolved relative to `base_dir`
- **Command Execution**: The exact command/script being run, working directory, timeout, and environment variables
- **Execution Results**: Exit codes, execution duration, output size, and any stderr output

**Example verbose output:**

```
[tools] Available tools: read_file, list_directory (base_dir=/Users/takafumi)
[tools] Selected tool: 'list_directory' with args: {"path":"Documents"}
[tools] Validating arguments for 'list_directory': {"path":"Documents"}
[tools] Validation passed for 'list_directory'
[tools] Calling tool 'list_directory' with args: {"path":"Documents"}
[tools] run: ls -la "Documents" (cwd=/Users/takafumi, timeout=10s)
[tools] done: exit_code=0, duration=0.05s, output_size=1234 bytes
```

This is especially useful when debugging:
- Why a tool isn't being called (check tool registration logs)
- Why arguments aren't working (check validation logs)
- Why paths aren't resolving correctly (check path resolution logs)
- Why commands fail (check execution logs for exact command and stderr)

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

See [CLAUDE.md](CLAUDE.md) for detailed development documentation.

## Changelog

### Version 0.2.2 (Security Release)

**Security Fixes:**
- **Fixed argument injection vulnerability in templated command arguments**: Path arguments in command-based tools are now validated and restricted to `base_dir` by default
- **Added option injection prevention**: Values starting with `-` are rejected for path arguments, and `--` separator is automatically inserted before templated path arguments
- **Enhanced path validation**: All templated path arguments are canonicalized and validated to prevent path traversal attacks
- **Configurable validation policies**: Added `template_validations` configuration for fine-grained control over argument validation

**New Features:**
- Added `restrict_to_base_dir` configuration option (default: `true`) for command tools
- Added `insert_double_dash` configuration option (default: auto-detect) for option injection prevention
- Added `template_validations` configuration for explicit validation policies per argument

**Documentation:**
- Added comprehensive security documentation for templated command arguments
- Updated configuration examples with secure defaults

### Version 0.2.1

- Initial release with local tools support