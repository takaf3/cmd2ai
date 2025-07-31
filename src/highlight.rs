use colored::*;
use syntect::easy::HighlightLines;
use syntect::highlighting::{Style, ThemeSet};
use syntect::parsing::SyntaxSet;
use syntect::util::{as_24_bit_terminal_escaped, LinesWithEndings};

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
                        output.push_str(&format!(
                            "{}[{}]{}\n",
                            "┌─".dimmed(),
                            self.code_block_lang.as_deref().unwrap_or("code").cyan(),
                            "─────────────────────────────────────────────────".dimmed()
                        ));
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
                // In code block, look for end marker
                if let Some(code_end) = self.buffer.find("```") {
                    // Add content before the end marker to code block
                    self.code_block_content.push_str(&self.buffer[..code_end]);

                    // Highlight and output any remaining lines
                    let all_lines: Vec<&str> = self.code_block_content.lines().collect();
                    if self.displayed_lines < all_lines.len() {
                        let remaining_lines: Vec<&str> = all_lines[self.displayed_lines..].to_vec();
                        if !remaining_lines.is_empty() {
                            let remaining_content = remaining_lines.join("\n") + "\n";
                            let highlighted = self.highlight_code(&remaining_content, self.code_block_lang.as_deref());
                            output.push_str(&highlighted);
                        }
                    }

                    // Output code block footer
                    output.push_str(&format!(
                        "{}\n",
                        "└──────────────────────────────────────────────────────────".dimmed()
                    ));

                    // Reset state
                    self.buffer = self.buffer[code_end + 3..].to_string();
                    self.in_code_block = false;
                    self.code_block_content.clear();
                    self.code_block_lang = None;
                    self.displayed_lines = 0;
                } else {
                    // Still in code block, accumulate content and highlight incrementally
                    self.code_block_content.push_str(&self.buffer);
                    self.buffer.clear();
                    
                    // Count complete lines in the accumulated content
                    let complete_lines: Vec<&str> = self.code_block_content.lines().collect();
                    let total_lines = complete_lines.len();
                    
                    // Check if we have new complete lines to display
                    if total_lines > self.displayed_lines && total_lines > 0 {
                        // Highlight only the new lines
                        let new_lines: Vec<&str> = complete_lines[self.displayed_lines..total_lines - 1].to_vec();
                        
                        if !new_lines.is_empty() {
                            let new_content = new_lines.join("\n") + "\n";
                            let highlighted = self.highlight_code(&new_content, self.code_block_lang.as_deref());
                            output.push_str(&highlighted);
                            self.displayed_lines = total_lines - 1;
                        }
                    }
                    
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
                        let highlighted = self.highlight_code(&final_content, self.code_block_lang.as_deref());
                        output.push_str(&highlighted);
                    }
                }
                output.push_str(&format!(
                    "{}\n",
                    "└──────────────────────────────────────────────────────────".dimmed()
                ));
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
