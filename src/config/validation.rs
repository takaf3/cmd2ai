use std::collections::HashMap;
use std::env;

/// Expand environment variables in a string using ${VAR_NAME} syntax
pub fn expand_env_var_in_string(value: &str) -> String {
    let mut result = value.to_string();
    let re = regex::Regex::new(r"\$\{([^}]+)\}").unwrap();

    for cap in re.captures_iter(value) {
        let var_name = &cap[1];
        let replacement = env::var(var_name).unwrap_or_else(|_| format!("${{{}}}", var_name));
        result = result.replace(&cap[0], &replacement);
    }

    result
}

/// Expand environment variables in a HashMap
pub fn expand_env_vars(env: &HashMap<String, String>) -> HashMap<String, String> {
    let mut expanded = HashMap::new();

    for (key, value) in env {
        let expanded_value = expand_env_var_in_string(value);
        expanded.insert(key.clone(), expanded_value);
    }

    expanded
}

