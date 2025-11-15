mod executor;
mod generator;

pub use executor::CodeExecutor;
pub use generator::generate_typescript_files;

use crate::config::CodeModeConfig;
use crate::error::{Cmd2AiError, Result};
use std::path::PathBuf;

pub fn resolve_bun_path(config: &CodeModeConfig) -> Result<String> {
    if let Some(ref bun_path) = config.bun_path {
        if PathBuf::from(bun_path).exists() {
            return Ok(bun_path.clone());
        }
        return Err(Cmd2AiError::Other(format!(
            "Configured bun path does not exist: {}",
            bun_path
        )));
    }

    which::which("bun")
        .map(|path| path.to_string_lossy().to_string())
        .map_err(|_| {
            Cmd2AiError::Other(
                "bun not found in PATH. Please install bun or set AI_CODE_MODE_BUN_PATH."
                    .to_string(),
            )
        })
}

pub fn create_workspace(session_id: &str, config: &CodeModeConfig) -> Result<PathBuf> {
    let base_dir = if let Some(ref dir) = config.workspace_dir {
        PathBuf::from(dir)
    } else {
        dirs::cache_dir()
            .unwrap_or_else(|| PathBuf::from("."))
            .join("cmd2ai")
            .join("code_mode")
    };

    std::fs::create_dir_all(&base_dir)?;
    let workspace = base_dir.join(session_id);
    std::fs::create_dir_all(&workspace)?;

    Ok(workspace)
}
