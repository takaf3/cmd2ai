# Bash MCP Server

A secure, locked-down MCP server that exposes only `ls` and `cat` commands for safe file system operations.

## Features

- **Locked-down commands**: Only `ls` and `cat` are allowed
- **Base directory sandbox**: All operations are restricted to a specified base directory
- **Path traversal protection**: Prevents access to files outside the base directory
- **File size limits**: Prevents reading files larger than 10MB
- **No shell execution**: Uses `execFile` directly, avoiding shell injection vulnerabilities

## Tools

### `ls_dir`

Lists files in a directory using `ls -la`.

**Parameters:**
- `path` (string, required): Path to the directory relative to the base directory

**Example:**
```json
{
  "name": "ls_dir",
  "arguments": {
    "path": "Documents"
  }
}
```

### `cat_file`

Reads and displays the contents of a file.

**Parameters:**
- `path` (string, required): Path to the file relative to the base directory

**Limitations:**
- Maximum file size: 10MB
- Only regular files can be read (not directories or special files)

**Example:**
```json
{
  "name": "cat_file",
  "arguments": {
    "path": ".zshrc"
  }
}
```

## Installation

This server requires [Bun](https://bun.sh/) to run:

```bash
# Install Bun (if not already installed)
curl -fsSL https://bun.sh/install | bash
```

## Usage

### Standalone Testing

Run the server directly:

```bash
bun samples/mcp-servers/bash-mcp/server.ts /path/to/base/directory
```

The server will accept JSON-RPC requests on stdin and send responses on stdout.

### With cmd2ai (Command Line)

Use the `--mcp-server` flag:

```bash
./ai \
  --mcp-server "bash:bun:samples/mcp-servers/bash-mcp/server.ts,/Users/takafumi" \
  "List files in my home directory and show the contents of .zshrc"
```

### With cmd2ai (Configuration File)

Add to your `~/.config/cmd2ai/cmd2ai.yaml` or `.cmd2ai.yaml`:

```yaml
mcp:
  servers:
    - name: bash
      command: bun
      args: ["samples/mcp-servers/bash-mcp/server.ts", "/Users/takafumi"]
      description: Locked-down bash (ls, cat) MCP server
      auto_activate_keywords:
        - ls
        - list
        - dir
        - directory
        - cat
        - read
        - file
        - show
        - display
      enabled: true
```

Or use an absolute path:

```yaml
mcp:
  servers:
    - name: bash
      command: bun
      args: ["/absolute/path/to/cmd2ai/samples/mcp-servers/bash-mcp/server.ts", "/Users/takafumi"]
      description: Locked-down bash (ls, cat) MCP server
      auto_activate_keywords: [ls, list, dir, directory, cat, read, file]
      enabled: true
```

## Security Considerations

### Base Directory

The base directory is set via:
1. Command-line argument (first argument after script path)
2. Environment variable `BASH_MCP_BASE_DIR`
3. Current working directory (fallback)

**Important**: Always specify a base directory explicitly. Using the current working directory as a fallback is less secure.

### Path Traversal Protection

The server implements multiple layers of protection:

1. **Path normalization**: Resolves `.` and `..` components
2. **Absolute path resolution**: Converts relative paths to absolute paths
3. **Base directory check**: Ensures resolved paths are within the base directory
4. **Path length limits**: Rejects paths longer than 4096 characters

### Command Restrictions

- Only `/bin/ls` and file reading operations are allowed
- No shell execution (uses `execFile` with `shell: false`)
- No arbitrary command execution
- File operations are limited to reading (no write/delete operations)

### File Size Limits

- Maximum file size for `cat_file`: 10MB
- Maximum output size for `ls_dir`: 10MB

## Example Queries

Once configured, you can use natural language queries:

```bash
# List files in a directory
./ai "List all files in my Documents folder"

# Read a file
./ai "Show me the contents of my .zshrc file"

# Combined operations
./ai "List my home directory and show me what's in .gitconfig"
```

## Troubleshooting

### "Path traversal detected" error

This means the requested path would escape the base directory. Check:
- The path is relative to the base directory
- The path doesn't contain `../` sequences
- The base directory is set correctly

### "Not a directory" or "Not a file" error

- For `ls_dir`: Ensure the path points to a directory
- For `cat_file`: Ensure the path points to a regular file (not a directory or symlink)

### "File too large" error

The file exceeds the 10MB limit. Consider using a different tool or increasing the limit in the code.

## Development

To modify the server:

1. Edit `server.ts`
2. Test locally:
   ```bash
   echo '{"jsonrpc":"2.0","id":1,"method":"initialize","params":{"protocolVersion":"2024-11-05","capabilities":{},"clientInfo":{"name":"test","version":"1.0.0"}}}' | bun server.ts /tmp
   ```

## License

Same as cmd2ai project.

