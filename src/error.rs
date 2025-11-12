use std::fmt;

#[derive(Debug)]
pub enum Cmd2AiError {
    ApiError {
        status: u16,
        message: String,
    },
    #[allow(dead_code)]
    ConfigError(String),
    #[allow(dead_code)]
    ToolError(String),
    #[allow(dead_code)]
    SessionError(String),
    NetworkError(reqwest::Error),
    Timeout,
    IoError(std::io::Error),
    JsonError(serde_json::Error),
    YamlError(serde_yaml::Error),
    Other(String),
}

impl fmt::Display for Cmd2AiError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Cmd2AiError::ApiError { status, message } => {
                write!(f, "API error (status {}): {}", status, message)
            }
            Cmd2AiError::ConfigError(msg) => write!(f, "Configuration error: {}", msg),
            Cmd2AiError::ToolError(msg) => write!(f, "Tool error: {}", msg),
            Cmd2AiError::SessionError(msg) => write!(f, "Session error: {}", msg),
            Cmd2AiError::NetworkError(e) => write!(f, "Network error: {}", e),
            Cmd2AiError::Timeout => write!(f, "Request timeout"),
            Cmd2AiError::IoError(e) => write!(f, "IO error: {}", e),
            Cmd2AiError::JsonError(e) => write!(f, "JSON error: {}", e),
            Cmd2AiError::YamlError(e) => write!(f, "YAML error: {}", e),
            Cmd2AiError::Other(msg) => write!(f, "{}", msg),
        }
    }
}

impl std::error::Error for Cmd2AiError {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        match self {
            Cmd2AiError::NetworkError(e) => Some(e),
            Cmd2AiError::IoError(e) => Some(e),
            Cmd2AiError::JsonError(e) => Some(e),
            Cmd2AiError::YamlError(e) => Some(e),
            _ => None,
        }
    }
}

impl From<reqwest::Error> for Cmd2AiError {
    fn from(err: reqwest::Error) -> Self {
        Cmd2AiError::NetworkError(err)
    }
}

impl From<std::io::Error> for Cmd2AiError {
    fn from(err: std::io::Error) -> Self {
        Cmd2AiError::IoError(err)
    }
}

impl From<serde_json::Error> for Cmd2AiError {
    fn from(err: serde_json::Error) -> Self {
        Cmd2AiError::JsonError(err)
    }
}

impl From<serde_yaml::Error> for Cmd2AiError {
    fn from(err: serde_yaml::Error) -> Self {
        Cmd2AiError::YamlError(err)
    }
}

impl From<anyhow::Error> for Cmd2AiError {
    fn from(err: anyhow::Error) -> Self {
        Cmd2AiError::Other(err.to_string())
    }
}

impl From<String> for Cmd2AiError {
    fn from(msg: String) -> Self {
        Cmd2AiError::Other(msg)
    }
}

impl From<&str> for Cmd2AiError {
    fn from(msg: &str) -> Self {
        Cmd2AiError::Other(msg.to_string())
    }
}

pub type Result<T> = std::result::Result<T, Cmd2AiError>;

