use crate::ui::highlight::CodeBuffer;

/// Display a tool result in a boxed format
pub fn display_tool_result(name: &str, result: &str) {
    // Avoid double newline if result_text already ends with one
    let sep = if result.ends_with('\n') { "" } else { "\n" };
    let tool_block = format!("```TOOL: {}\n{}{}\n```", name, result, sep);
    let mut code_buffer = CodeBuffer::new();
    let formatted = code_buffer.append(&tool_block);
    if !formatted.is_empty() {
        print!("{}", formatted);
    }
    let remaining = code_buffer.flush();
    if !remaining.is_empty() {
        print!("{}", remaining.trim_end());
    }
    println!();
}

/// Display a tool error in a boxed format
pub fn display_tool_error(name: &str, error: &str) {
    // Avoid double newline if error_text already ends with one
    let sep = if error.ends_with('\n') { "" } else { "\n" };
    let tool_error_block = format!("```TOOL ERROR: {}\n{}{}\n```", name, error, sep);
    let mut code_buffer = CodeBuffer::new();
    let formatted = code_buffer.append(&tool_error_block);
    if !formatted.is_empty() {
        print!("{}", formatted);
    }
    let remaining = code_buffer.flush();
    if !remaining.is_empty() {
        print!("{}", remaining.trim_end());
    }
    println!();
}

/// Display reasoning content in a boxed format
pub fn display_reasoning(reasoning: &str) {
    // Clean up markdown formatting for display
    let display_reasoning = reasoning.replace("**", "").trim().to_string();

    // Use CodeBuffer to render reasoning block with dynamic width
    // Avoid double newline if content already ends with one
    let sep = if display_reasoning.ends_with('\n') { "" } else { "\n" };
    let reasoning_block = format!("```REASONING\n{}{}\n```", display_reasoning, sep);
    let mut reasoning_code_buffer = CodeBuffer::new();
    let formatted = reasoning_code_buffer.append(&reasoning_block);
    if !formatted.is_empty() {
        println!();
        print!("{}", formatted);
    }
    let remaining = reasoning_code_buffer.flush();
    if !remaining.is_empty() {
        print!("{}", remaining.trim_end());
    }
    println!();
}

/// Display content with syntax highlighting
pub fn display_content(content: &str) {
    let mut code_buffer = CodeBuffer::new();
    let formatted = code_buffer.append(content);
    if !formatted.is_empty() {
        print!("{}", formatted);
    }
    let remaining = code_buffer.flush();
    if !remaining.is_empty() {
        print!("{}", remaining.trim_end());
    }
    println!();
}

