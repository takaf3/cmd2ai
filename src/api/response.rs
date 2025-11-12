use crate::error::Result;
use serde_json::Value;

/// Parse a non-streaming API response and extract tool calls if present
pub fn parse_tool_calls(response_json: &Value) -> Result<Option<Vec<Value>>> {
    let choices = response_json
        .get("choices")
        .and_then(|c| c.as_array())
        .ok_or_else(|| crate::error::Cmd2AiError::Other("No choices in response".to_string()))?;

    let first_choice = choices
        .first()
        .ok_or_else(|| crate::error::Cmd2AiError::Other("Empty choices array".to_string()))?;

    let message = first_choice
        .get("message")
        .ok_or_else(|| crate::error::Cmd2AiError::Other("No message in response".to_string()))?;

    if let Some(tool_calls) = message.get("tool_calls").and_then(|tc| tc.as_array()) {
        if !tool_calls.is_empty() {
            return Ok(Some(tool_calls.clone()));
        }
    }

    Ok(None)
}

/// Extract content from a non-streaming response
pub fn extract_content(response_json: &Value) -> Result<Option<String>> {
    let choices = response_json
        .get("choices")
        .and_then(|c| c.as_array())
        .ok_or_else(|| crate::error::Cmd2AiError::Other("No choices in response".to_string()))?;

    let first_choice = choices
        .first()
        .ok_or_else(|| crate::error::Cmd2AiError::Other("Empty choices array".to_string()))?;

    let message = first_choice
        .get("message")
        .ok_or_else(|| crate::error::Cmd2AiError::Other("No message in response".to_string()))?;

    Ok(message
        .get("content")
        .and_then(|c| c.as_str())
        .map(|s| s.to_string()))
}

/// Extract reasoning content from a non-streaming response
pub fn extract_reasoning(response_json: &Value) -> Result<Option<String>> {
    let choices = response_json
        .get("choices")
        .and_then(|c| c.as_array())
        .ok_or_else(|| crate::error::Cmd2AiError::Other("No choices in response".to_string()))?;

    let first_choice = choices
        .first()
        .ok_or_else(|| crate::error::Cmd2AiError::Other("Empty choices array".to_string()))?;

    Ok(first_choice
        .get("message")
        .and_then(|m| m.get("reasoning_content"))
        .and_then(|r| r.as_str())
        .map(|s| s.to_string()))
}

