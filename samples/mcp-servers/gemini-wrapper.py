#!/usr/bin/env python3
"""
MCP Server wrapper for Gemini AI tool
This wrapper exposes the gemini command as an MCP-compatible tool
"""

import json
import sys
import subprocess
from typing import Dict, Any, List

def send_response(id: Any, result: Any = None, error: Any = None):
    """Send a JSON-RPC response"""
    response = {"jsonrpc": "2.0", "id": id}
    if error is not None:
        response["error"] = error
    else:
        response["result"] = result
    
    print(json.dumps(response))
    sys.stdout.flush()

def handle_initialize(request_id: Any):
    """Handle the initialize request"""
    result = {
        "protocolVersion": "2024-11-05",
        "capabilities": {
            "tools": {}
        },
        "serverInfo": {
            "name": "gemini-mcp-server",
            "version": "1.0.0"
        }
    }
    send_response(request_id, result)

def handle_list_tools(request_id: Any):
    """List available tools"""
    tools = [
        {
            "name": "gemini_search",
            "description": "Search the web using Google Gemini AI for current information, news, weather, and real-time data",
            "inputSchema": {
                "type": "object",
                "properties": {
                    "query": {
                        "type": "string",
                        "description": "The search query (e.g., 'current weather in Tokyo', 'latest news about AI', 'stock price of AAPL')"
                    }
                },
                "required": ["query"]
            }
        }
    ]
    send_response(request_id, {"tools": tools})

def handle_call_tool(request_id: Any, tool_name: str, arguments: Dict[str, Any]):
    """Execute a tool call"""
    try:
        if tool_name == "gemini_search":
            query = arguments.get("query", "")
            # Always prepend "WebSearch:" for gemini to know it should search the web
            prompt = f"WebSearch: {query}"
        else:
            send_response(request_id, error={
                "code": -32602,
                "message": f"Unknown tool: {tool_name}"
            })
            return
        
        # Call gemini command
        result = subprocess.run(
            ["gemini", "-p", prompt],
            capture_output=True,
            text=True,
            timeout=60  # Increased timeout for retries
        )
        
        if result.returncode == 0:
            content = [{
                "type": "text",
                "text": result.stdout.strip()
            }]
            send_response(request_id, {"content": content})
        else:
            send_response(request_id, error={
                "code": -32603,
                "message": f"Gemini command failed: {result.stderr}"
            })
            
    except subprocess.TimeoutExpired:
        send_response(request_id, error={
            "code": -32603,
            "message": "Gemini command timed out"
        })
    except Exception as e:
        send_response(request_id, error={
            "code": -32603,
            "message": f"Error executing gemini: {str(e)}"
        })

def main():
    """Main loop for the MCP server"""
    while True:
        try:
            line = sys.stdin.readline()
            if not line:
                break
            
            request = json.loads(line)
            method = request.get("method")
            request_id = request.get("id")
            params = request.get("params", {})
            
            if method == "initialize":
                handle_initialize(request_id)
            elif method == "tools/list":
                handle_list_tools(request_id)
            elif method == "tools/call":
                tool_name = params.get("name")
                arguments = params.get("arguments", {})
                handle_call_tool(request_id, tool_name, arguments)
            else:
                send_response(request_id, error={
                    "code": -32601,
                    "message": f"Method not found: {method}"
                })
                
        except json.JSONDecodeError:
            # Invalid JSON, ignore
            continue
        except KeyboardInterrupt:
            break
        except Exception as e:
            sys.stderr.write(f"Error: {str(e)}\n")
            sys.stderr.flush()

if __name__ == "__main__":
    main()