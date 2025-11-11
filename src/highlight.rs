use colored::*;
use syntect::easy::HighlightLines;
use syntect::highlighting::{Style, ThemeSet};
use syntect::parsing::SyntaxSet;
use syntect::util::{as_24_bit_terminal_escaped, LinesWithEndings};
use terminal_size::{terminal_size, Width};

pub struct CodeBuffer {
    buffer: String,
    in_code_block: bool,
    code_block_content: String,
    code_block_lang: Option<String>,
    syntax_set: SyntaxSet,
    theme_set: ThemeSet,
    displayed_lines: usize,
}

impl CodeBuffer {
    pub fn new() -> Self {
        Self {
            buffer: String::new(),
            in_code_block: false,
            code_block_content: String::new(),
            code_block_lang: None,
            syntax_set: SyntaxSet::load_defaults_newlines(),
            theme_set: ThemeSet::load_defaults(),
            displayed_lines: 0,
        }
    }

    /// Compute target width for code block borders
    /// Returns width between 50 and 120, defaulting to 80 if terminal size unavailable
    fn compute_box_width(&self) -> usize {
        if let Some((Width(w), _)) = terminal_size() {
            let cols = w as usize;
            cols.min(120).max(50)
        } else {
            80
        }
    }

    /// Generate header line for code block with dynamic width
    fn format_header(&self, label: &str) -> String {
        let width = self.compute_box_width();
        // Calculate label length: label itself + 2 brackets
        let label_len = label.len() + 2;
        // Account for "┌─" prefix (2 chars)
        let dash_count = width.saturating_sub(2 + label_len);
        let dashes = "─".repeat(dash_count.max(1));
        format!(
            "{}[{}]{}\n",
            "┌─".dimmed(),
            label.cyan(),
            dashes.dimmed()
        )
    }

    /// Generate footer line for code block with dynamic width
    fn format_footer(&self) -> String {
        let width = self.compute_box_width();
        // Account for "└" prefix (1 char)
        let dash_count = width.saturating_sub(1);
        let dashes = "─".repeat(dash_count.max(1));
        format!("\n{}{}", "└─".dimmed(), dashes.dimmed())
    }

    fn find_code_block_end(&self, text: &str) -> Option<usize> {
        // Look for ``` at the beginning of a line
        if text.starts_with("```") {
            return Some(0);
        }

        // Look for \n``` in the text
        if let Some(pos) = text.find("\n```") {
            // Return position of the backticks (after the newline)
            return Some(pos + 1);
        }

        None
    }

    fn highlight_code(&self, code: &str, lang: Option<&str>) -> String {
        let theme = &self.theme_set.themes["Solarized (dark)"];

        let syntax = if let Some(lang) = lang {
            self.syntax_set
                .find_syntax_by_token(lang)
                .or_else(|| self.syntax_set.find_syntax_by_extension(lang))
                .unwrap_or_else(|| self.syntax_set.find_syntax_plain_text())
        } else {
            self.syntax_set.find_syntax_plain_text()
        };

        let mut highlighter = HighlightLines::new(syntax, theme);
        let mut output = String::new();

        for line in LinesWithEndings::from(code) {
            let ranges: Vec<(Style, &str)> =
                highlighter.highlight_line(line, &self.syntax_set).unwrap();
            let escaped = as_24_bit_terminal_escaped(&ranges[..], false);
            output.push_str(&escaped);
        }

        output
    }

    pub fn append(&mut self, content: &str) -> String {
        self.buffer.push_str(content);
        let mut output = String::new();

        while !self.buffer.is_empty() {
            if !self.in_code_block {
                // Look for code block start
                if let Some(code_start) = self.buffer.find("```") {
                    // Output everything before the code block
                    output.push_str(&self.buffer[..code_start]);

                    // Extract the code block marker and language
                    self.buffer = self.buffer[code_start + 3..].to_string();

                    // Check if we have a complete first line with language
                    if let Some(newline_pos) = self.buffer.find('\n') {
                        let lang_line = self.buffer[..newline_pos].trim();
                        self.code_block_lang = if lang_line.is_empty() {
                            None
                        } else {
                            Some(lang_line.to_string())
                        };

                        self.buffer = self.buffer[newline_pos + 1..].to_string();
                        self.in_code_block = true;
                        self.code_block_content.clear();
                        self.displayed_lines = 0;

                        // Output code block header
                        let label = self.code_block_lang.as_deref().unwrap_or("code");
                        output.push_str(&self.format_header(label));
                    } else {
                        // Incomplete first line, wait for more content
                        self.buffer = format!("```{}", self.buffer);
                        break;
                    }
                } else {
                    // No code block found, output everything and clear buffer
                    output.push_str(&self.buffer);
                    self.buffer.clear();
                }
            } else {
                // In code block, look for end marker at the beginning of a line
                let code_end = self.find_code_block_end(&self.buffer);
                if let Some(code_end) = code_end {
                    // Add content before the end marker to code block
                    // Strip trailing newline if present (the \n before ```)
                    let content_before_marker = &self.buffer[..code_end];
                    let stripped_newline = content_before_marker.ends_with('\n');
                    let content_to_add = if stripped_newline {
                        &content_before_marker[..content_before_marker.len() - 1]
                    } else {
                        content_before_marker
                    };
                    self.code_block_content.push_str(content_to_add);

                    // Highlight and output any remaining lines
                    let all_lines: Vec<&str> = self.code_block_content.lines().collect();
                    if self.displayed_lines < all_lines.len() {
                        let remaining_lines: Vec<&str> = all_lines[self.displayed_lines..].to_vec();
                        if !remaining_lines.is_empty() {
                            let remaining_content = remaining_lines.join("\n");
                            // Add final newline only if the original content had one
                            // and we didn't just strip a newline before the closing marker
                            let final_content =
                                if self.code_block_content.ends_with('\n') && !stripped_newline {
                                    remaining_content + "\n"
                                } else {
                                    remaining_content
                                };
                            let highlighted = self
                                .highlight_code(&final_content, self.code_block_lang.as_deref());
                            output.push_str(&highlighted);
                        }
                    }

                    // Output code block footer
                    output.push_str(&self.format_footer());

                    // Consume the closing ``` and check what comes after
                    let after_marker = &self.buffer[code_end + 3..];

                    // Only add newline after footer if there's content following or a newline
                    if !after_marker.is_empty() {
                        output.push('\n');
                    }

                    // Reset state
                    self.buffer = after_marker.to_string();
                    self.in_code_block = false;
                    self.code_block_content.clear();
                    self.code_block_lang = None;
                    self.displayed_lines = 0;
                } else {
                    // Still in code block, accumulate content and highlight incrementally
                    self.code_block_content.push_str(&self.buffer);

                    // Count complete lines in the accumulated content
                    let complete_lines: Vec<&str> = self.code_block_content.lines().collect();
                    let total_lines = complete_lines.len();

                    // Check if the last line is incomplete (doesn't end with newline)
                    let has_incomplete_last_line =
                        !self.code_block_content.ends_with('\n') && !self.buffer.is_empty();

                    // Determine how many lines to display
                    let lines_to_display = if has_incomplete_last_line && total_lines > 0 {
                        total_lines - 1 // Don't display the incomplete last line yet
                    } else {
                        total_lines
                    };

                    // Check if we have new complete lines to display
                    if lines_to_display > self.displayed_lines {
                        // Highlight only the new complete lines
                        let new_lines: Vec<&str> =
                            complete_lines[self.displayed_lines..lines_to_display].to_vec();

                        if !new_lines.is_empty() {
                            let new_content = new_lines.join("\n") + "\n";
                            let highlighted =
                                self.highlight_code(&new_content, self.code_block_lang.as_deref());
                            output.push_str(&highlighted);
                            self.displayed_lines = lines_to_display;
                        }
                    }

                    self.buffer.clear();
                    break;
                }
            }
        }

        output
    }

    pub fn flush(&mut self) -> String {
        let mut output = String::new();

        if self.in_code_block {
            // Unterminated code block
            if !self.code_block_content.is_empty() {
                // Highlight any remaining lines that haven't been displayed
                let all_lines: Vec<&str> = self.code_block_content.lines().collect();
                if self.displayed_lines < all_lines.len() {
                    let remaining_lines: Vec<&str> = all_lines[self.displayed_lines..].to_vec();
                    if !remaining_lines.is_empty() {
                        let remaining_content = remaining_lines.join("\n");
                        // Add newline only if the original content ended with one
                        let final_content = if self.code_block_content.ends_with('\n') {
                            remaining_content + "\n"
                        } else {
                            remaining_content
                        };
                        let highlighted =
                            self.highlight_code(&final_content, self.code_block_lang.as_deref());
                        output.push_str(&highlighted);
                    }
                }
                output.push_str(&self.format_footer());
            }
        } else if !self.buffer.is_empty() {
            output.push_str(&self.buffer);
        }

        self.buffer.clear();
        self.code_block_content.clear();
        self.in_code_block = false;
        self.code_block_lang = None;
        self.displayed_lines = 0;

        output
    }
}
