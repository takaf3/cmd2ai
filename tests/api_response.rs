use cmd2ai::api::response::{extract_content, extract_reasoning, parse_tool_calls};
use serde_json::json;

#[test]
fn test_extract_content_with_content() {
    let response = json!({
        "choices": [{
            "message": {
                "content": "Hello, world!",
                "role": "assistant"
            }
        }]
    });

    let content = extract_content(&response).unwrap();
    assert_eq!(content, Some("Hello, world!".to_string()));
}

#[test]
fn test_extract_content_without_content() {
    let response = json!({
        "choices": [{
            "message": {
                "role": "assistant"
            }
        }]
    });

    let content = extract_content(&response).unwrap();
    assert_eq!(content, None);
}

#[test]
fn test_extract_content_empty_choices() {
    let response = json!({
        "choices": []
    });

    let result = extract_content(&response);
    assert!(result.is_err());
}

#[test]
fn test_parse_tool_calls_with_tools() {
    let response = json!({
        "choices": [{
            "message": {
                "role": "assistant",
                "tool_calls": [
                    {
                        "id": "call_123",
                        "type": "function",
                        "function": {
                            "name": "read_file",
                            "arguments": "{\"path\": \"test.txt\"}"
                        }
                    }
                ]
            }
        }]
    });

    let tool_calls = parse_tool_calls(&response).unwrap();
    assert!(tool_calls.is_some());
    let calls = tool_calls.unwrap();
    assert_eq!(calls.len(), 1);
    assert_eq!(calls[0]["id"], "call_123");
}

#[test]
fn test_parse_tool_calls_without_tools() {
    let response = json!({
        "choices": [{
            "message": {
                "role": "assistant",
                "content": "No tools needed"
            }
        }]
    });

    let tool_calls = parse_tool_calls(&response).unwrap();
    assert!(tool_calls.is_none());
}

#[test]
fn test_parse_tool_calls_empty_array() {
    let response = json!({
        "choices": [{
            "message": {
                "role": "assistant",
                "tool_calls": []
            }
        }]
    });

    let tool_calls = parse_tool_calls(&response).unwrap();
    assert!(tool_calls.is_none());
}

#[test]
fn test_extract_reasoning_with_reasoning() {
    let response = json!({
        "choices": [{
            "message": {
                "role": "assistant",
                "reasoning_content": "Let me think about this..."
            }
        }]
    });

    let reasoning = extract_reasoning(&response).unwrap();
    assert_eq!(reasoning, Some("Let me think about this...".to_string()));
}

#[test]
fn test_extract_reasoning_without_reasoning() {
    let response = json!({
        "choices": [{
            "message": {
                "role": "assistant",
                "content": "No reasoning"
            }
        }]
    });

    let reasoning = extract_reasoning(&response).unwrap();
    assert_eq!(reasoning, None);
}

