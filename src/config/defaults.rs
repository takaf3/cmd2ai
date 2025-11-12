pub fn default_tools_enabled() -> bool {
    true
}

pub fn default_local_tools_enabled() -> bool {
    true
}

pub fn default_max_file_size_mb() -> u64 {
    10
}

pub fn default_tool_timeout() -> u64 {
    30
}

pub fn default_max_output_bytes() -> u64 {
    1_048_576 // 1MB default
}

pub fn default_stdin_json() -> bool {
    true // Default to true for backward compatibility
}

pub fn is_default_stdin_json(value: &bool) -> bool {
    *value == default_stdin_json()
}

pub fn default_restrict_to_base_dir() -> bool {
    true // Default to true for security
}

pub fn is_default_restrict_to_base_dir(value: &bool) -> bool {
    *value == default_restrict_to_base_dir()
}

pub fn default_validation_kind() -> String {
    "string".to_string()
}

pub fn default_allow_absolute() -> bool {
    false // Default to false for security
}

pub fn is_default_allow_absolute(value: &bool) -> bool {
    *value == default_allow_absolute()
}

