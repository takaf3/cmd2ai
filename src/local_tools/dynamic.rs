use crate::config::LocalToolConfig;
use serde_json::Value;
use std::future::Future;
use std::pin::Pin;

use super::executor::execute_dynamic_tool;
use super::registry::{LocalSettings, LocalTool};

/// Convert a LocalToolConfig with type field into a LocalTool
pub fn create_dynamic_tool(
    tool_config: &LocalToolConfig,
    settings: &LocalSettings,
) -> Result<LocalTool, String> {
    // Validate required fields
    let tool_type = tool_config
        .r#type
        .as_deref()
        .ok_or_else(|| format!("Tool '{}' is missing 'type' field", tool_config.name))?;

    if tool_type != "script" && tool_type != "command" {
        return Err(format!(
            "Tool '{}' has invalid type '{}' (must be 'script' or 'command')",
            tool_config.name, tool_type
        ));
    }

    let description = tool_config
        .description
        .clone()
        .ok_or_else(|| format!("Tool '{}' is missing 'description' field", tool_config.name))?;

    let input_schema = tool_config.input_schema.clone().ok_or_else(|| {
        format!(
            "Tool '{}' is missing 'input_schema' field",
            tool_config.name
        )
    })?;

    // Validate schema-specific requirements
    if tool_type == "script" {
        if tool_config.interpreter.is_none() {
            return Err(format!(
                "Tool '{}' (type: script) requires 'interpreter' field",
                tool_config.name
            ));
        }
        if tool_config.script.is_none() && tool_config.script_path.is_none() {
            return Err(format!(
                "Tool '{}' (type: script) requires either 'script' or 'script_path' field",
                tool_config.name
            ));
        }
    } else if tool_type == "command" {
        if tool_config.command.is_none() {
            return Err(format!(
                "Tool '{}' (type: command) requires 'command' field",
                tool_config.name
            ));
        }
    }

    // Create a handler that calls the executor
    let tool_config_clone = tool_config.clone();
    let settings_clone = settings.clone();

    let handler: Box<
        dyn for<'a> Fn(
                &'a Value,
                &'a LocalSettings,
            )
                -> Pin<Box<dyn Future<Output = Result<String, String>> + Send + 'a>>
            + Send
            + Sync,
    > = Box::new(
        move |args: &Value,
              _settings: &LocalSettings|
              -> Pin<Box<dyn Future<Output = Result<String, String>> + Send + '_>> {
            let args = args.clone();
            let tool_config = tool_config_clone.clone();
            let settings = settings_clone.clone();
            Box::pin(async move { execute_dynamic_tool(&tool_config, &args, &settings).await })
        },
    );

    Ok(LocalTool {
        name: tool_config.name.clone(),
        description,
        input_schema,
        handler,
    })
}
