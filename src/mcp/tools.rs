use super::types::McpTool;
use serde_json::{json, Value};

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
