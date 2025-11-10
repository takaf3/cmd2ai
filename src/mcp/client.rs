use jsonschema::JSONSchema;
use serde_json::{json, Value};
use std::collections::HashMap;
use std::io::{BufRead, BufReader, Write};
use std::process::{Child, Command, Stdio};
use std::sync::Arc;
use tokio::sync::RwLock;
use tokio::time::{timeout, Duration};

use super::types::{
    InitializeResult, McpTool, McpToolCall, McpToolResult, ResourceListResponse,
    ResourceReadResponse, ToolListResponse,
};

// MCP Protocol constants
const MCP_PROTOCOL_VERSION: &str = "2024-11-05";
const CLIENT_NAME: &str = "cmd2ai";
const CLIENT_VERSION: &str = "0.1.0";

pub struct McpClient {
    servers: Arc<RwLock<HashMap<String, McpServer>>>,
    tools: Arc<RwLock<HashMap<String, (String, McpTool)>>>, // tool_name -> (server_name, tool)
    verbose: bool,
}

struct McpServer {
    process: Child,
    next_id: u64,
}

impl McpClient {
    pub fn new(verbose: bool) -> Self {
        Self {
            servers: Arc::new(RwLock::new(HashMap::new())),
            tools: Arc::new(RwLock::new(HashMap::new())),
            verbose,
        }
    }

    pub async fn connect_server(
        &self,
        server_name: &str,
        command: &str,
        args: Vec<String>,
        env_vars: HashMap<String, String>,
    ) -> Result<(), Box<dyn std::error::Error>> {
        // Start the MCP server process
        let mut cmd = Command::new(command);
        cmd.args(args)
            .stdin(Stdio::piped())
            .stdout(Stdio::piped())
            .stderr(Stdio::null());

        // Set environment variables for the child process
        // Note: We don't log env var values in verbose mode for security
        for (key, value) in env_vars {
            if self.verbose {
                eprintln!("  Setting env var: {} (value hidden)", key);
            }
            cmd.env(key, value);
        }

        let process = cmd.spawn()?;

        let mut server = McpServer {
            process,
            next_id: 1,
        };

        // Initialize the connection
        let init_params = json!({
            "protocolVersion": MCP_PROTOCOL_VERSION,
            "capabilities": {
                "tools": {}
            },
            "clientInfo": {
                "name": CLIENT_NAME,
                "version": CLIENT_VERSION
            }
        });

        let response = self.send_request(&mut server, "initialize", Some(init_params))?;
        let init_result: InitializeResult = serde_json::from_value(response)?;

        if self.verbose {
            println!(
                "Connected to MCP server: {} v{}",
                init_result.server_info.name, init_result.server_info.version
            );
        }

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
        let server = servers.get_mut(server_name).ok_or("Server not found")?;

        let response = self.send_request(server, "tools/list", None)?;
        let tool_list: ToolListResponse = serde_json::from_value(response)?;

        drop(servers);

        let mut tools = self.tools.write().await;
        // Remove old tools from this server first
        tools.retain(|_, (srv_name, _)| srv_name != server_name);

        // Add new tools
        for tool in tool_list.tools {
            if self.verbose {
                println!(
                    "  - Tool: {} - {}",
                    tool.name,
                    tool.description.as_ref().unwrap_or(&String::new())
                );
            }
            tools.insert(tool.name.clone(), (server_name.to_string(), tool));
        }

        Ok(())
    }

    /// Refresh tools for all connected servers
    pub async fn refresh_tools(&self) -> Result<(), Box<dyn std::error::Error>> {
        let server_names: Vec<String> = {
            let servers = self.servers.read().await;
            servers.keys().cloned().collect()
        };

        for server_name in server_names {
            if let Err(e) = self.discover_tools(&server_name).await {
                if self.verbose {
                    eprintln!(
                        "Warning: Failed to refresh tools for server '{}': {}",
                        server_name, e
                    );
                }
                // Continue with other servers even if one fails
            }
        }

        Ok(())
    }

    pub async fn get_tool(&self, tool_name: &str) -> Option<McpTool> {
        let tools = self.tools.read().await;
        tools.get(tool_name).map(|(_, tool)| tool.clone())
    }

    pub async fn list_tools(&self) -> Vec<McpTool> {
        let tools = self.tools.read().await;
        tools.values().map(|(_, tool)| tool.clone()).collect()
    }

    pub async fn call_tool(
        &self,
        tool_call: &McpToolCall,
        timeout_secs: u64,
    ) -> Result<McpToolResult, Box<dyn std::error::Error>> {
        // Validate arguments against schema before calling
        if let Some(tool) = self.get_tool(&tool_call.name).await {
            if let Err(validation_errors) =
                self.validate_tool_arguments(&tool, &tool_call.arguments)
            {
                return Err(format!(
                    "Tool '{}' argument validation failed: {}",
                    tool_call.name, validation_errors
                )
                .into());
            }
        }

        let tools = self.tools.read().await;
        let (server_name, _tool) = tools
            .get(&tool_call.name)
            .ok_or_else(|| format!("Tool '{}' not found", tool_call.name))?;
        let server_name = server_name.clone();
        drop(tools);

        let timeout_duration = Duration::from_secs(timeout_secs);

        // Wrap the tool call in a timeout
        match timeout(
            timeout_duration,
            self.call_tool_internal(tool_call, &server_name),
        )
        .await
        {
            Ok(result) => result,
            Err(_) => Err(format!(
                "Tool '{}' execution timed out after {} seconds",
                tool_call.name, timeout_secs
            )
            .into()),
        }
    }

    fn validate_tool_arguments(&self, tool: &McpTool, arguments: &Value) -> Result<(), String> {
        // Compile the JSON schema
        let schema = match JSONSchema::compile(&tool.input_schema) {
            Ok(s) => s,
            Err(e) => return Err(format!("Invalid tool schema: {}", e)),
        };

        // Validate arguments against schema
        if let Err(errors) = schema.validate(arguments) {
            let error_messages: Vec<String> = errors
                .map(|e| format!("{}: {}", e.instance_path, e.to_string()))
                .collect();
            return Err(error_messages.join("; "));
        }

        Ok(())
    }

    async fn call_tool_internal(
        &self,
        tool_call: &McpToolCall,
        server_name: &str,
    ) -> Result<McpToolResult, Box<dyn std::error::Error>> {
        let mut servers = self.servers.write().await;
        let server = servers.get_mut(server_name).ok_or("Server not found")?;

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

    /// List resources from a specific server
    /// Note: Resource API support is groundwork for future implementation
    #[allow(dead_code)]
    pub async fn list_resources(
        &self,
        server_name: &str,
    ) -> Result<ResourceListResponse, Box<dyn std::error::Error>> {
        let mut servers = self.servers.write().await;
        let server = servers.get_mut(server_name).ok_or("Server not found")?;

        let response = self.send_request(server, "resources/list", None)?;
        let resource_list: ResourceListResponse = serde_json::from_value(response)?;
        Ok(resource_list)
    }

    /// Read a resource from a specific server
    /// Note: Resource API support is groundwork for future implementation
    #[allow(dead_code)]
    pub async fn read_resource(
        &self,
        server_name: &str,
        uri: &str,
    ) -> Result<ResourceReadResponse, Box<dyn std::error::Error>> {
        let mut servers = self.servers.write().await;
        let server = servers.get_mut(server_name).ok_or("Server not found")?;

        let params = json!({
            "uri": uri,
        });

        let response = self.send_request(server, "resources/read", Some(params))?;
        let resource_read: ResourceReadResponse = serde_json::from_value(response)?;
        Ok(resource_read)
    }
}

impl Drop for McpClient {
    fn drop(&mut self) {
        // Cleanup will be handled by shutdown method
    }
}
