use super::registry::LocalToolRegistry;
use colored::Colorize;
use serde_json::{json, Value};

pub fn format_tools_for_llm(registry: &LocalToolRegistry) -> Vec<Value> {
    registry
        .list()
        .iter()
        .map(|tool| {
            json!({
                "type": "function",
                "function": {
                    "name": tool.name,
                    "description": tool.description,
                    "parameters": tool.input_schema,
                }
            })
        })
        .collect()
}

pub async fn call_local_tool(
    registry: &LocalToolRegistry,
    tool_name: &str,
    arguments: &Value,
) -> Result<String, String> {
    let settings = registry.settings();
    if settings.verbose {
        let args_str = serde_json::to_string(arguments)
            .unwrap_or_else(|_| "<invalid json>".to_string());
        let truncated = if args_str.len() > 200 {
            format!("{}...", &args_str[..200])
        } else {
            args_str
        };
        eprintln!(
            "{}",
            format!("[tools] Calling tool '{}' with args: {}", tool_name, truncated).dimmed()
        );
    }

    // Validate arguments first
    registry.validate_arguments(tool_name, arguments)?;

    // Get the tool
    let tool = registry
        .get(tool_name)
        .ok_or_else(|| format!("Tool '{}' not found", tool_name))?;

    // Call the handler (now async)
    let handler = &tool.handler;
    handler(arguments, registry.settings()).await
}
