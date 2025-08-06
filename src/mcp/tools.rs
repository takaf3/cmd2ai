use serde_json::{json, Value};
use super::types::{McpTool, McpToolCall, McpToolResult};

pub fn format_tools_for_llm(tools: &[McpTool]) -> Vec<Value> {
    tools
        .iter()
        .map(|tool| {
            json!({
                "type": "function",
                "function": {
                    "name": tool.name,
                    "description": tool.description.as_ref().unwrap_or(&String::new()),
                    "parameters": tool.input_schema,
                }
            })
        })
        .collect()
}

pub fn parse_llm_tool_call(tool_call: &Value) -> Option<McpToolCall> {
    let function = tool_call.get("function")?;
    let name = function.get("name")?.as_str()?.to_string();
    let arguments_str = function.get("arguments")?.as_str()?;
    let arguments: Value = serde_json::from_str(arguments_str).ok()?;
    
    Some(McpToolCall {
        name,
        arguments,
    })
}

pub fn format_tool_result_for_llm(result: &McpToolResult, tool_call_id: &str) -> Value {
    let content = result.content
        .iter()
        .map(|c| c.text.clone())
        .collect::<Vec<_>>()
        .join("\n");
    
    json!({
        "role": "tool",
        "tool_call_id": tool_call_id,
        "content": content,
    })
}