use crate::api::{make_api_request, process_streaming_response, RequestBody};
use crate::api::response::{extract_content, extract_reasoning, parse_tool_calls};
use crate::cli::Args;
use crate::config::Config;
use crate::error::{Cmd2AiError, Result};
use crate::local_tools::{call_local_tool, format_tools_for_llm, LocalToolRegistry};
use crate::models::Message;
use crate::ui::{display_content, display_reasoning, display_tool_error, display_tool_result};
use colored::*;
use serde_json::Value;

pub struct OrchestratorContext {
    pub config: Config,
    pub args: Args,
    pub local_tools_registry: Option<LocalToolRegistry>,
}

pub async fn run(context: OrchestratorContext, messages: &mut Vec<Message>) -> Result<String> {
    let final_model = context.config.model.clone();

    // Get available tools unless explicitly disabled
    let _local_tools_enabled = context.config.tools_enabled
        && context.config.local_tools_config.enabled
        && !context.args.no_tools;

    // Collect tools from local tools
    let mut all_tools = Vec::new();

    // Add local tools
    if let Some(ref registry) = context.local_tools_registry {
        let local_tools = format_tools_for_llm(registry);
        if !local_tools.is_empty() {
            if context.config.verbose {
                let tool_names: Vec<String> = registry.list().iter().map(|t| t.name.clone()).collect();
                eprintln!(
                    "{}",
                    format!(
                        "[tools] Available tools: {} (base_dir={})",
                        tool_names.join(", "),
                        registry.settings().base_dir.display()
                    )
                    .dimmed()
                );
            } else {
                println!(
                    "{}",
                    format!("Available local tools: {}", local_tools.len()).cyan()
                );
            }
            all_tools.extend(local_tools);
        }
    }

    let tools = if all_tools.is_empty() {
        None
    } else {
        Some(all_tools)
    };

    // Use non-streaming when tools are available for proper tool handling
    // OpenRouter's streaming API doesn't properly stream tool call arguments
    let use_streaming = tools.is_none();

    let request_body = RequestBody {
        model: final_model.clone(),
        messages: messages.to_vec(),
        stream: use_streaming,
        reasoning: context.config.reasoning.clone(),
        tools: tools.clone(),
    };

    // Debug: Print tools being sent
    if context.config.verbose && tools.is_some() {
        eprintln!(
            "{}",
            "[AI] Sending tools to model for function calling".dimmed()
        );
    }

    if context.config.verbose {
        eprintln!("{}", format!("[AI] Using model: {}", final_model).dimmed());
    }

    // Make API request
    if context.config.verbose {
        eprintln!("{}", "[AI] Making API request...".dimmed());
    }
    let response = make_api_request(&context.config.api_key, &context.config.api_endpoint, &request_body).await?;

    if context.config.verbose {
        eprintln!(
            "{}",
            format!("[AI] Response status: {}", response.status()).dimmed()
        );
    }

    if !response.status().is_success() {
        let status = response.status().as_u16();
        let error_text = response.text().await.unwrap_or_else(|_| "Unknown error".to_string());
        return Err(Cmd2AiError::ApiError {
            status,
            message: error_text,
        });
    }

    // Process response based on whether we're streaming or not
    let assistant_response = if use_streaming {
        // Streaming path - no tools available
        let streaming_result = process_streaming_response(
            response,
            context.config.stream_timeout,
            context.args.reasoning_exclude,
            context.config.verbose,
        )
        .await?;

        streaming_result.content
    } else {
        // Non-streaming path - handle tools properly
        let response_text = response.text().await?;
        if context.config.verbose {
            eprintln!(
                "{}",
                format!("[AI] Raw response: {}", response_text).dimmed()
            );
        }

        // Parse the response
        let response_json: Value = serde_json::from_str(&response_text)?;

        // Process the non-streaming response with tool handling
        process_non_streaming_response(
            &context,
            response_json,
            messages,
            &final_model,
        )
        .await?
    };

    Ok(assistant_response)
}

async fn process_non_streaming_response(
    context: &OrchestratorContext,
    response_json: Value,
    messages: &mut Vec<Message>,
    final_model: &str,
) -> Result<String> {
    // Check for reasoning content first
    if let Ok(Some(reasoning_content)) = extract_reasoning(&response_json) {
        if !context.args.reasoning_exclude && !reasoning_content.is_empty() {
            display_reasoning(&reasoning_content);
        }
    }

    // Check if there are tool calls
    if let Ok(Some(tool_calls)) = parse_tool_calls(&response_json) {
        if !tool_calls.is_empty() {
            if context.config.verbose {
                println!("{}", "Executing tools...".cyan());
            }

            let tool_results = execute_tool_calls(context, &tool_calls).await?;

            // If we executed tools, we need to send the results back and get a new response
            if !tool_results.is_empty() {
                // Add the assistant's message with tool calls to the conversation
                let first_choice = response_json
                    .get("choices")
                    .and_then(|c| c.as_array())
                    .and_then(|c| c.first())
                    .ok_or_else(|| Cmd2AiError::Other("No choices in response".to_string()))?;

                let message = first_choice
                    .get("message")
                    .ok_or_else(|| Cmd2AiError::Other("No message in response".to_string()))?;

                // Convert tool_calls array to proper ToolCall objects
                let tool_calls_typed: Vec<crate::models::ToolCall> = tool_calls
                    .iter()
                    .filter_map(|tc| serde_json::from_value(tc.clone()).ok())
                    .collect();

                messages.push(Message {
                    role: "assistant".to_string(),
                    content: message
                        .get("content")
                        .and_then(|c| c.as_str())
                        .map(|s| s.to_string()),
                    tool_calls: if tool_calls_typed.is_empty() {
                        None
                    } else {
                        Some(tool_calls_typed)
                    },
                    tool_call_id: None,
                });

                // Add tool results to the conversation
                for result in tool_results {
                    messages.push(result);
                }

                // Make another API call to get the final response - NOW WITH STREAMING!
                let followup_request = RequestBody {
                    model: final_model.to_string(),
                    messages: messages.to_vec(),
                    stream: true, // Enable streaming for the final answer
                    reasoning: context.config.reasoning.clone(),
                    tools: None, // Don't send tools again for the final response
                };

                if context.config.verbose {
                    eprintln!("{}", "[AI] Making follow-up request with tool results (streaming enabled)...".dimmed());
                }

                let followup_response = make_api_request(
                    &context.config.api_key,
                    &context.config.api_endpoint,
                    &followup_request,
                )
                .await?;

                if !followup_response.status().is_success() {
                    let status = followup_response.status().as_u16();
                    let error_text = followup_response.text().await.unwrap_or_else(|_| "Unknown error".to_string());
                    return Err(Cmd2AiError::ApiError {
                        status,
                        message: error_text,
                    });
                }

                // Process the follow-up STREAMING response for better UX
                let followup_result = process_streaming_response(
                    followup_response,
                    context.config.stream_timeout,
                    context.args.reasoning_exclude,
                    context.config.verbose,
                )
                .await?;

                // Return the final streamed response
                return Ok(followup_result.content);
            }
        }
    }

    // No tool calls - extract and display content
    if let Ok(Some(content)) = extract_content(&response_json) {
        if context.config.verbose {
            eprintln!(
                "{}",
                "[AI] tool_calls array is empty; using assistant message content.".dimmed()
            );
        }

        display_content(&content);
        Ok(content)
    } else {
        if context.config.verbose {
            eprintln!(
                "{}",
                "[AI] tool_calls array is empty and no content provided.".dimmed()
            );
        }
        Ok("No tool calls and no content in response".to_string())
    }
}

async fn execute_tool_calls(
    context: &OrchestratorContext,
    tool_calls: &[Value],
) -> Result<Vec<Message>> {
    let mut tool_results = Vec::new();

    for tool_call in tool_calls {
        // Check for required fields and report errors for malformed tool calls
        let id = tool_call.get("id").and_then(|i| i.as_str());
        let function = tool_call.get("function");

        if id.is_none() {
            eprintln!("{}", "Warning: Tool call missing 'id' field, skipping".yellow());
            // Generate a temporary ID for error reporting
            let temp_id = format!(
                "error_{}",
                std::time::SystemTime::now()
                    .duration_since(std::time::UNIX_EPOCH)
                    .unwrap()
                    .as_nanos()
            );
            tool_results.push(Message {
                role: "tool".to_string(),
                content: Some("Error: Tool call missing required 'id' field".to_string()),
                tool_calls: None,
                tool_call_id: Some(temp_id),
            });
            continue;
        }
        let id = id.unwrap();

        if function.is_none() {
            eprintln!(
                "{}",
                format!("Warning: Tool call {} missing 'function' field, skipping", id).yellow()
            );
            tool_results.push(Message {
                role: "tool".to_string(),
                content: Some(format!(
                    "Error: Tool call {} missing required 'function' field",
                    id
                )),
                tool_calls: None,
                tool_call_id: Some(id.to_string()),
            });
            continue;
        }
        let function = function.unwrap();

        let name = function.get("name").and_then(|n| n.as_str());
        let arguments_str = function.get("arguments").and_then(|a| a.as_str());

        if name.is_none() {
            eprintln!(
                "{}",
                format!("Warning: Tool call {} missing 'function.name' field, skipping", id)
                    .yellow()
            );
            tool_results.push(Message {
                role: "tool".to_string(),
                content: Some(format!(
                    "Error: Tool call {} missing required 'function.name' field",
                    id
                )),
                tool_calls: None,
                tool_call_id: Some(id.to_string()),
            });
            continue;
        }
        let name = name.unwrap();

        if arguments_str.is_none() {
            eprintln!(
                "{}",
                format!("Warning: Tool call {} missing 'function.arguments' field, skipping", id)
                    .yellow()
            );
            tool_results.push(Message {
                role: "tool".to_string(),
                content: Some(format!(
                    "Error: Tool call {} missing required 'function.arguments' field",
                    id
                )),
                tool_calls: None,
                tool_call_id: Some(id.to_string()),
            });
            continue;
        }
        let arguments_str = arguments_str.unwrap();

        if context.config.verbose {
            let args_preview = if arguments_str.len() > 100 {
                format!("{}...", &arguments_str[..100])
            } else {
                arguments_str.to_string()
            };
            eprintln!(
                "{}",
                format!("[tools] Selected tool: '{}' with args: {}", name, args_preview).dimmed()
            );
        }

        println!("{}", format!("Calling tool: {}...", name).cyan());

        // Parse arguments
        match serde_json::from_str::<Value>(arguments_str) {
            Ok(arguments) => {
                // Execute local tool
                if let Some(ref registry) = context.local_tools_registry {
                    if registry.get(name).is_some() {
                        match call_local_tool(registry, name, &arguments).await {
                            Ok(result_text) => {
                                display_tool_result(name, &result_text);

                                // Keep the original result_text for the message (not the formatted version)
                                tool_results.push(Message {
                                    role: "tool".to_string(),
                                    content: Some(result_text),
                                    tool_calls: None,
                                    tool_call_id: Some(id.to_string()),
                                });
                            }
                            Err(e) => {
                                let error_text = format!("Error: {}", e);
                                display_tool_error(name, &error_text);

                                tool_results.push(Message {
                                    role: "tool".to_string(),
                                    content: Some(error_text),
                                    tool_calls: None,
                                    tool_call_id: Some(id.to_string()),
                                });
                            }
                        }
                    } else {
                        // Display tool not found error in a boxed format
                        let error_text = format!("Error: Tool '{}' not found", name);
                        display_tool_error(name, &error_text);

                        tool_results.push(Message {
                            role: "tool".to_string(),
                            content: Some(error_text),
                            tool_calls: None,
                            tool_call_id: Some(id.to_string()),
                        });
                    }
                } else {
                    // Display tool not found error (local tools disabled) in a boxed format
                    let error_text = format!("Error: Tool '{}' not found (local tools disabled)", name);
                    display_tool_error(name, &error_text);

                    tool_results.push(Message {
                        role: "tool".to_string(),
                        content: Some(format!("Error: Tool '{}' not found", name)),
                        tool_calls: None,
                        tool_call_id: Some(id.to_string()),
                    });
                }
            }
            Err(err) => {
                // Display argument parsing error in a boxed format
                let error_text =
                    format!("Error: failed to parse arguments for tool '{}' : {}", name, err);
                display_tool_error(name, &error_text);

                tool_results.push(Message {
                    role: "tool".to_string(),
                    content: Some(error_text),
                    tool_calls: None,
                    tool_call_id: Some(id.to_string()),
                });
            }
        }
    }

    Ok(tool_results)
}

