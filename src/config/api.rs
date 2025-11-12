use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct ApiConfig {
    #[serde(default)]
    pub endpoint: Option<String>,
    #[serde(default)]
    pub stream_timeout: Option<u64>,
}

impl Default for ApiConfig {
    fn default() -> Self {
        Self {
            endpoint: None,
            stream_timeout: None,
        }
    }
}

