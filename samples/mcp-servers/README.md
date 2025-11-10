# MCP Server Examples

This directory contains example MCP (Model Context Protocol) server implementations that can be used with cmd2ai.

## Gemini Wrapper

`gemini-wrapper.py` - A Python wrapper that exposes the `gemini` CLI tool as an MCP server.

### Features
- Provides `gemini_search` tool for web search and real-time information
- Automatically prepends "WebSearch:" to queries for proper gemini operation
- Handles timeout and error cases gracefully

### Usage

1. Make sure you have the `gemini` CLI tool installed and configured
2. Update your cmd2ai config (`~/.config/cmd2ai/cmd2ai.yaml`) to include:

```yaml
mcp:
  servers:
    - name: gemini
      command: python3
      args: ["/path/to/gemini-wrapper.py"]
      description: Google Gemini AI for web search and research
      enabled: true
```

3. Use with cmd2ai:
```bash
ai "What's the current weather in Tokyo?"
ai "Search for latest news about AI"
```

## Bash MCP Server

`bash-mcp/server.ts` - A secure, locked-down Node (bun) MCP server that exposes only `ls` and `cat` commands.

### Features
- **Locked-down commands**: Only `ls` and `cat` are allowed
- **Base directory sandbox**: All operations restricted to a specified directory
- **Path traversal protection**: Prevents access outside the base directory
- **File size limits**: Prevents reading files larger than 10MB
- **No shell execution**: Uses direct file operations, avoiding shell injection

### Tools
- `ls_dir` - List files in a directory using `ls -la`
- `cat_file` - Read file contents (limited to 10MB files)

### Usage

1. Install [Bun](https://bun.sh/) if not already installed
2. Add to your cmd2ai config (`~/.config/cmd2ai/cmd2ai.yaml`):

```yaml
mcp:
  servers:
    - name: bash
      command: bun
      args: ["samples/mcp-servers/bash-mcp/server.ts", "/Users/takafumi"]
      description: Locked-down bash (ls, cat) MCP server
      auto_activate_keywords: [ls, list, dir, directory, cat, read, file]
      enabled: true
```

3. Use with cmd2ai:
```bash
ai "List files in my Documents folder"
ai "Show me the contents of .zshrc"
ai "List my home directory and show .gitconfig"
```

See `bash-mcp/README.md` for detailed documentation and security considerations.

## Creating Your Own MCP Server

MCP servers communicate via JSON-RPC 2.0 over stdio. To create your own:

1. Handle the `initialize` method to establish connection
2. Implement `tools/list` to advertise available tools
3. Implement `tools/call` to execute tool functions
4. Return results in MCP format with proper error handling

See `gemini-wrapper.py` (Python) or `bash-mcp/server.ts` (Node/bun) for complete example implementations.