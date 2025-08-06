use serde_json::{json, Value};
use std::collections::HashMap;
use std::io::{BufRead, BufReader, Write};
use std::process::{Child, Command, Stdio};
use std::sync::Arc;
use tokio::sync::RwLock;

use super::types::{
    InitializeResult, McpTool, McpToolCall, McpToolResult, ToolListResponse,
};

pub struct McpClient {
    servers: Arc<RwLock<HashMap<String, McpServer>>>,
    tools: Arc<RwLock<HashMap<String, (String, McpTool)>>>, // tool_name -> (server_name, tool)
}

struct McpServer {
    process: Child,
    next_id: u64,
}

impl McpClient {
    pub fn new() -> Self {
        Self {
            servers: Arc::new(RwLock::new(HashMap::new())),
            tools: Arc::new(RwLock::new(HashMap::new())),
        }
    }

    pub async fn connect_server(
        &self,
        server_name: &str,
        command: &str,
        args: Vec<String>,
    ) -> Result<(), Box<dyn std::error::Error>> {
        // Start the MCP server process
        let mut cmd = Command::new(command);
        let process = cmd
            .args(args)
            .stdin(Stdio::piped())
            .stdout(Stdio::piped())
            .stderr(Stdio::null())
            .spawn()?;

        let mut server = McpServer {
            process,
            next_id: 1,
        };

        // Initialize the connection
        let init_params = json!({
            "protocolVersion": "2024-11-05",
            "capabilities": {
                "tools": {}
            },
            "clientInfo": {
                "name": "cmd2ai",
                "version": "0.1.0"
            }
        });

        let response = self.send_request(&mut server, "initialize", Some(init_params))?;
        let init_result: InitializeResult = serde_json::from_value(response)?;
        
        println!(
            "Connected to MCP server: {} v{}",
            init_result.server_info.name, init_result.server_info.version
        );

        // Send initialized notification
        self.send_notification(&mut server, "notifications/initialized", None)?;

        // Store the server
        {
            let mut servers = self.servers.write().await;
            servers.insert(server_name.to_string(), server);
        }

        // Discover and register tools
        self.discover_tools(server_name).await?;

        Ok(())
    }

    fn send_request(
        &self,
        server: &mut McpServer,
        method: &str,
        params: Option<Value>,
    ) -> Result<Value, Box<dyn std::error::Error>> {
        let id = server.next_id;
        server.next_id += 1;

        let request = json!({
            "jsonrpc": "2.0",
            "id": id,
            "method": method,
            "params": params.unwrap_or(json!({}))
        });

        // Send request
        if let Some(stdin) = server.process.stdin.as_mut() {
            let request_str = serde_json::to_string(&request)?;
            writeln!(stdin, "{}", request_str)?;
            stdin.flush()?;
        }

        // Read response
        if let Some(stdout) = server.process.stdout.as_mut() {
            let reader = BufReader::new(stdout);
            for line in reader.lines() {
                let line = line?;
                if line.trim().is_empty() {
                    continue;
                }
                
                let response: Value = serde_json::from_str(&line)?;
                if response.get("id") == Some(&json!(id)) {
                    if let Some(result) = response.get("result") {
                        return Ok(result.clone());
                    } else if let Some(error) = response.get("error") {
                        return Err(format!("MCP error: {}", error).into());
                    }
                }
            }
        }

        Err("No response from MCP server".into())
    }

    fn send_notification(
        &self,
        server: &mut McpServer,
        method: &str,
        params: Option<Value>,
    ) -> Result<(), Box<dyn std::error::Error>> {
        let notification = json!({
            "jsonrpc": "2.0",
            "method": method,
            "params": params.unwrap_or(json!({}))
        });

        if let Some(stdin) = server.process.stdin.as_mut() {
            let notification_str = serde_json::to_string(&notification)?;
            writeln!(stdin, "{}", notification_str)?;
            stdin.flush()?;
        }

        Ok(())
    }

    async fn discover_tools(&self, server_name: &str) -> Result<(), Box<dyn std::error::Error>> {
        let mut servers = self.servers.write().await;
        let server = servers
            .get_mut(server_name)
            .ok_or("Server not found")?;

        let response = self.send_request(server, "tools/list", None)?;
        let tool_list: ToolListResponse = serde_json::from_value(response)?;

        drop(servers);

        let mut tools = self.tools.write().await;
        for tool in tool_list.tools {
            println!("  - Tool: {} - {}", tool.name, tool.description.as_ref().unwrap_or(&String::new()));
            tools.insert(
                tool.name.clone(),
                (server_name.to_string(), tool),
            );
        }

        Ok(())
    }

    pub async fn list_tools(&self) -> Vec<McpTool> {
        let tools = self.tools.read().await;
        tools.values().map(|(_, tool)| tool.clone()).collect()
    }

    pub async fn call_tool(
        &self,
        tool_call: &McpToolCall,
    ) -> Result<McpToolResult, Box<dyn std::error::Error>> {
        let tools = self.tools.read().await;
        let (server_name, _tool) = tools
            .get(&tool_call.name)
            .ok_or_else(|| format!("Tool '{}' not found", tool_call.name))?;
        let server_name = server_name.clone();
        drop(tools);

        let mut servers = self.servers.write().await;
        let server = servers
            .get_mut(&server_name)
            .ok_or("Server not found")?;

        let params = json!({
            "name": tool_call.name,
            "arguments": tool_call.arguments,
        });

        let response = self.send_request(server, "tools/call", Some(params))?;
        let result: McpToolResult = serde_json::from_value(response)?;
        Ok(result)
    }

    pub async fn shutdown(&self) -> Result<(), Box<dyn std::error::Error>> {
        let mut servers = self.servers.write().await;
        for (_name, mut server) in servers.drain() {
            // Send shutdown request
            let _ = self.send_request(&mut server, "shutdown", None);
            // Kill the process
            let _ = server.process.kill();
        }
        Ok(())
    }
}

impl Drop for McpClient {
    fn drop(&mut self) {
        // Cleanup will be handled by shutdown method
    }
}