use serde::{Deserialize, Serialize};
use serde_json::Value;

#[derive(Serialize)]
pub struct RequestBody {
    pub model: String,
    pub messages: Vec<crate::models::Message>,
    pub stream: bool,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub reasoning: Option<crate::models::Reasoning>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub tools: Option<Vec<Value>>,
}

#[derive(Deserialize)]
pub struct Citation {
    pub url: String,
    pub title: String,
    #[allow(dead_code)]
    pub content: Option<String>,
}

#[derive(Deserialize)]
pub struct Annotation {
    #[serde(rename = "type")]
    pub annotation_type: String,
    pub url_citation: Option<Citation>,
}

#[derive(Deserialize)]
pub struct Delta {
    pub content: Option<String>,
    pub annotations: Option<Vec<Annotation>>,
    pub reasoning: Option<String>,
    #[allow(dead_code)]
    pub tool_calls: Option<Vec<crate::models::ToolCall>>,
}

#[derive(Deserialize)]
pub struct Choice {
    pub delta: Option<Delta>,
}

#[derive(Deserialize)]
pub struct StreamResponse {
    pub choices: Option<Vec<Choice>>,
}

