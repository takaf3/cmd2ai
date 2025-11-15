use crate::config::CodeModeConfig;
use crate::error::{Cmd2AiError, Result};
use colored::*;
use std::path::PathBuf;
use std::process::Stdio;
use std::time::{Duration, Instant};
use tokio::io::AsyncReadExt;
use tokio::process::Command as TokioCommand;
use tokio::time::timeout;

#[derive(Debug, Clone)]
pub struct ExecutionResult {
    pub stdout: String,
    pub stderr: String,
    pub exit_code: i32,
}

pub struct CodeExecutor {
    bun_path: String,
    workspace: PathBuf,
    timeout: Duration,
    max_output_bytes: u64,
    verbose: bool,
}

impl CodeExecutor {
    pub fn new(
        bun_path: String,
        workspace: PathBuf,
        config: &CodeModeConfig,
        verbose: bool,
    ) -> Self {
        let timeout_secs = config.timeout_secs.unwrap_or(60);
        let max_output_bytes = config.max_output_bytes.unwrap_or(10_485_760);

        Self {
            bun_path,
            workspace,
            timeout: Duration::from_secs(timeout_secs),
            max_output_bytes,
            verbose,
        }
    }

    pub fn write_code(&self, code: &str) -> Result<()> {
        let main_ts = self.workspace.join("main.ts");
        std::fs::write(&main_ts, code)?;
        if self.verbose {
            eprintln!(
                "{}",
                format!("[code_mode] Wrote TypeScript code to {}", main_ts.display()).dimmed()
            );
        }
        Ok(())
    }

    pub async fn execute(&self) -> Result<ExecutionResult> {
        if self.verbose {
            eprintln!(
                "{}",
                format!(
                    "[code_mode] Executing: {} run {}",
                    self.bun_path,
                    self.workspace.join("main.ts").display()
                )
                .dimmed()
            );
        }

        let mut cmd = TokioCommand::new(&self.bun_path);
        cmd.arg("run")
            .arg(self.workspace.join("main.ts"))
            .current_dir(&self.workspace)
            .stdout(Stdio::piped())
            .stderr(Stdio::piped());

        let mut child = cmd
            .spawn()
            .map_err(|e| Cmd2AiError::Other(format!("Failed to spawn bun process: {}", e)))?;

        let mut stdout_pipe = child
            .stdout
            .take()
            .ok_or_else(|| Cmd2AiError::Other("Failed to capture stdout".to_string()))?;
        let mut stderr_pipe = child
            .stderr
            .take()
            .ok_or_else(|| Cmd2AiError::Other("Failed to capture stderr".to_string()))?;

        let max_output_bytes = self.max_output_bytes;
        let stdout_handle = tokio::spawn(async move {
            let mut output = Vec::new();
            let mut buffer = vec![0u8; 8192];
            let mut total_bytes = 0u64;

            loop {
                match stdout_pipe.read(&mut buffer).await {
                    Ok(0) => break,
                    Ok(n) => {
                        total_bytes += n as u64;
                        if total_bytes > max_output_bytes {
                            return Err(Cmd2AiError::Other(format!(
                                "Output exceeded maximum size of {} bytes",
                                max_output_bytes
                            )));
                        }
                        output.extend_from_slice(&buffer[..n]);
                    }
                    Err(e) => return Err(Cmd2AiError::IoError(e)),
                }
            }

            Ok(String::from_utf8_lossy(&output).to_string())
        });

        let max_output_bytes_stderr = self.max_output_bytes;
        let stderr_handle = tokio::spawn(async move {
            let mut output = Vec::new();
            let mut buffer = vec![0u8; 8192];
            let mut total_bytes = 0u64;

            loop {
                match stderr_pipe.read(&mut buffer).await {
                    Ok(0) => break,
                    Ok(n) => {
                        total_bytes += n as u64;
                        if total_bytes > max_output_bytes_stderr {
                            return Err(Cmd2AiError::Other(format!(
                                "Stderr exceeded maximum size of {} bytes",
                                max_output_bytes_stderr
                            )));
                        }
                        output.extend_from_slice(&buffer[..n]);
                    }
                    Err(e) => return Err(Cmd2AiError::IoError(e)),
                }
            }

            Ok(String::from_utf8_lossy(&output).to_string())
        });

        let start = Instant::now();
        let wait_result = timeout(self.timeout, child.wait()).await;
        let timed_out = wait_result.is_err();

        if timed_out {
            let _ = child.kill().await;
        }

        let exit_code = match wait_result {
            Ok(Ok(status)) => status.code().unwrap_or(-1),
            Ok(Err(e)) => {
                return Err(Cmd2AiError::Other(format!(
                    "Failed to wait for bun process: {}",
                    e
                )))
            }
            Err(_) => -1,
        };

        let stdout = stdout_handle
            .await
            .map_err(|e| Cmd2AiError::Other(format!("Failed to read stdout: {}", e)))??;
        let stderr = stderr_handle
            .await
            .map_err(|e| Cmd2AiError::Other(format!("Failed to read stderr: {}", e)))??;

        if timed_out {
            if self.verbose {
                eprintln!(
                    "{}",
                    format!(
                        "[code_mode] Bun execution timed out after {:.2}s",
                        self.timeout.as_secs_f64()
                    )
                    .red()
                );
            }
            return Err(Cmd2AiError::Timeout);
        }

        if self.verbose {
            let elapsed = start.elapsed().as_secs_f64();
            eprintln!(
                "{}",
                format!(
                    "[code_mode] Execution completed in {:.2}s (exit code: {})",
                    elapsed, exit_code
                )
                .dimmed()
            );
            if !stderr.is_empty() {
                eprintln!("{}", format!("[code_mode] Stderr: {}", stderr).dimmed());
            }
        }

        Ok(ExecutionResult {
            stdout,
            stderr,
            exit_code,
        })
    }
}
