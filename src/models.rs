use serde::{Deserialize, Serialize};

#[derive(Serialize, Deserialize, Clone)]
pub struct Message {
    pub role: String,
    pub content: String,
}

#[derive(Serialize, Deserialize, Clone)]
pub struct Session {
    pub session_id: String,
    pub last_updated: chrono::DateTime<chrono::Local>,
    pub messages: Vec<Message>,
}

#[derive(Serialize)]
pub struct WebPlugin {
    pub id: String,
    pub max_results: u32,
    pub search_prompt: String,
}

#[derive(Serialize)]
pub struct Reasoning {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub effort: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub max_tokens: Option<u32>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub exclude: Option<bool>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub enabled: Option<bool>,
}

#[derive(Serialize)]
pub struct RequestBody {
    pub model: String,
    pub messages: Vec<Message>,
    pub stream: bool,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub plugins: Option<Vec<WebPlugin>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub reasoning: Option<Reasoning>,
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
}

#[derive(Deserialize)]
pub struct Choice {
    pub delta: Option<Delta>,
}

#[derive(Deserialize)]
pub struct StreamResponse {
    pub choices: Option<Vec<Choice>>,
}
