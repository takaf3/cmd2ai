#!/usr/bin/env bun
/**
 * MCP Server for locked-down bash commands (ls, cat)
 * This server exposes only ls and cat commands in a secure, sandboxed way
 */

import { execFile } from "node:child_process";
import { promisify } from "node:util";
import { resolve, normalize } from "node:path";
import { stat } from "node:fs/promises";
import { readFileSync } from "node:fs";

const pexecFile = promisify(execFile);

// Get base directory from command line argument or environment variable
const BASE_DIR = resolve(process.argv[2] || process.env.BASH_MCP_BASE_DIR || process.cwd());

// Validate base directory exists
try {
  const baseStat = await stat(BASE_DIR);
  if (!baseStat.isDirectory()) {
    console.error(`Error: Base path is not a directory: ${BASE_DIR}`);
    process.exit(1);
  }
} catch (error) {
  console.error(`Error: Base directory does not exist: ${BASE_DIR}`);
  process.exit(1);
}

/**
 * Safely resolve a user-provided path within the base directory
 * Prevents path traversal attacks
 */
function safeResolve(userPath: string): string {
  // Basic validation: reject empty, very long, or non-string paths
  if (!userPath || typeof userPath !== "string" || userPath.length > 4096) {
    throw new Error("Invalid path: path must be a non-empty string under 4096 characters");
  }

  // Normalize the path (resolves . and ..)
  const normalized = normalize(userPath);
  
  // Resolve against base directory
  const resolved = resolve(BASE_DIR, normalized);
  
  // Ensure the resolved path is within the base directory
  // Normalize base directory to avoid issues with trailing slashes
  const normalizedBase = BASE_DIR.endsWith("/") && BASE_DIR !== "/" 
    ? BASE_DIR.slice(0, -1) 
    : BASE_DIR;
  const normalizedResolved = resolved.endsWith("/") && resolved !== "/"
    ? resolved.slice(0, -1)
    : resolved;
  
  if (!normalizedResolved.startsWith(normalizedBase) && normalizedResolved !== normalizedBase) {
    throw new Error(`Path traversal detected: ${userPath} escapes base directory`);
  }
  
  return resolved;
}

/**
 * Run ls -la on a directory
 */
async function runLs(path: string): Promise<string> {
  const absPath = safeResolve(path);
  
  // Verify it's actually a directory
  const stats = await stat(absPath);
  if (!stats.isDirectory()) {
    throw new Error(`Not a directory: ${path}`);
  }
  
  // Execute ls -la using execFile (no shell, safer)
  const { stdout } = await pexecFile("/bin/ls", ["-la", absPath], {
    shell: false,
    maxBuffer: 10 * 1024 * 1024, // 10MB max output
  });
  
  return stdout;
}

/**
 * Run cat on a file
 */
async function runCat(path: string): Promise<string> {
  const absPath = safeResolve(path);
  
  // Verify it's actually a file
  const stats = await stat(absPath);
  if (!stats.isFile()) {
    throw new Error(`Not a file: ${path}`);
  }
  
  // Check file size (prevent reading huge files)
  if (stats.size > 10 * 1024 * 1024) { // 10MB limit
    throw new Error(`File too large: ${path} (${stats.size} bytes, max 10MB)`);
  }
  
  // Read file directly (safer than execFile for reading)
  const content = readFileSync(absPath, "utf-8");
  return content;
}

// Tool definitions
const tools = [
  {
    name: "ls_dir",
    description: "List files in a directory using ls -la. Shows detailed file information including permissions, size, and modification time.",
    inputSchema: {
      type: "object",
      properties: {
        path: {
          type: "string",
          description: "Path to the directory to list (relative to base directory)",
        },
      },
      required: ["path"],
      additionalProperties: false,
    },
  },
  {
    name: "cat_file",
    description: "Read and display the contents of a file. Limited to files under 10MB.",
    inputSchema: {
      type: "object",
      properties: {
        path: {
          type: "string",
          description: "Path to the file to read (relative to base directory)",
        },
      },
      required: ["path"],
      additionalProperties: false,
    },
  },
];

/**
 * Send a JSON-RPC response
 */
function sendResponse(id: any, result?: any, error?: any): void {
  const response: any = { jsonrpc: "2.0", id };
  if (error !== undefined) {
    response.error = error;
  } else {
    response.result = result;
  }
  
  console.log(JSON.stringify(response));
  // Flush stdout to ensure immediate delivery
  if (process.stdout.isTTY) {
    process.stdout.flush?.();
  }
}

/**
 * Handle initialize request
 */
function handleInitialize(requestId: any): void {
  const result = {
    protocolVersion: "2024-11-05",
    capabilities: {
      tools: {},
    },
    serverInfo: {
      name: "bash-mcp-server",
      version: "1.0.0",
    },
  };
  sendResponse(requestId, result);
}

/**
 * Handle initialized notification (no response needed)
 */
function handleInitialized(): void {
  // Notifications don't require a response
}

/**
 * Handle tools/list request
 */
function handleListTools(requestId: any): void {
  sendResponse(requestId, { tools });
}

/**
 * Handle tools/call request
 */
async function handleCallTool(requestId: any, toolName: string, arguments_: any): Promise<void> {
  try {
    let content: string;
    
    if (toolName === "ls_dir") {
      const path = arguments_?.path;
      if (!path || typeof path !== "string") {
        sendResponse(requestId, undefined, {
          code: -32602,
          message: "Invalid arguments: path is required and must be a string",
        });
        return;
      }
      content = await runLs(path);
    } else if (toolName === "cat_file") {
      const path = arguments_?.path;
      if (!path || typeof path !== "string") {
        sendResponse(requestId, undefined, {
          code: -32602,
          message: "Invalid arguments: path is required and must be a string",
        });
        return;
      }
      content = await runCat(path);
    } else {
      sendResponse(requestId, undefined, {
        code: -32602,
        message: `Unknown tool: ${toolName}`,
      });
      return;
    }
    
    sendResponse(requestId, {
      content: [
        {
          type: "text",
          text: content,
        },
      ],
    });
  } catch (error: any) {
    sendResponse(requestId, undefined, {
      code: -32603,
      message: error.message || String(error),
    });
  }
}

/**
 * Main loop: read JSON-RPC requests from stdin
 */
async function main(): Promise<void> {
  const stdin = process.stdin;
  stdin.setEncoding("utf8");
  
  let buffer = "";
  
  stdin.on("data", async (chunk: string) => {
    buffer += chunk;
    
    // Process complete lines (JSON-RPC messages are newline-delimited)
    const lines = buffer.split("\n");
    buffer = lines.pop() || ""; // Keep incomplete line in buffer
    
    for (const line of lines) {
      if (!line.trim()) continue;
      
      try {
        const request = JSON.parse(line);
        const method = request.method;
        const requestId = request.id;
        const params = request.params || {};
        
        if (method === "initialize") {
          handleInitialize(requestId);
        } else if (method === "notifications/initialized") {
          handleInitialized();
        } else if (method === "tools/list") {
          handleListTools(requestId);
        } else if (method === "tools/call") {
          await handleCallTool(requestId, params.name, params.arguments);
        } else if (method === "shutdown") {
          process.exit(0);
        } else {
          sendResponse(requestId, undefined, {
            code: -32601,
            message: `Method not found: ${method}`,
          });
        }
      } catch (error: any) {
        // Invalid JSON, ignore or log error
        if (error.name !== "SyntaxError") {
          console.error(`Error processing request: ${error.message}`);
        }
      }
    }
  });
  
  stdin.on("end", () => {
    process.exit(0);
  });
  
  // Handle errors
  stdin.on("error", (error) => {
    console.error(`Stdin error: ${error.message}`);
    process.exit(1);
  });
}

// Start the server
main().catch((error) => {
  console.error(`Fatal error: ${error.message}`);
  process.exit(1);
});

