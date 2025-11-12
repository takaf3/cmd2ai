use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct ReasoningConfig {
    #[serde(default)]
    pub enabled: Option<bool>,
    #[serde(default)]
    pub effort: Option<String>,
    #[serde(default)]
    pub max_tokens: Option<u32>,
    #[serde(default)]
    pub exclude: Option<bool>,
}

impl Default for ReasoningConfig {
    fn default() -> Self {
        Self {
            enabled: None,
            effort: None,
            max_tokens: None,
            exclude: None,
        }
    }
}

