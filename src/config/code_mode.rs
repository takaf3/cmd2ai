use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct CodeModeConfig {
    #[serde(default = "default_code_mode_enabled")]
    pub enabled: bool,

    #[serde(default)]
    pub workspace_dir: Option<String>,

    #[serde(default)]
    pub bun_path: Option<String>,

    #[serde(default = "default_code_mode_timeout")]
    pub timeout_secs: Option<u64>,

    #[serde(default = "default_code_mode_max_output_bytes")]
    pub max_output_bytes: Option<u64>,
}

impl Default for CodeModeConfig {
    fn default() -> Self {
        Self {
            enabled: default_code_mode_enabled(),
            workspace_dir: None,
            bun_path: None,
            timeout_secs: default_code_mode_timeout(),
            max_output_bytes: default_code_mode_max_output_bytes(),
        }
    }
}

pub fn default_code_mode_enabled() -> bool {
    true
}

pub fn default_code_mode_timeout() -> Option<u64> {
    Some(60)
}

pub fn default_code_mode_max_output_bytes() -> Option<u64> {
    Some(10_485_760)
}
