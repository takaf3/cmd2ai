use crate::error::Result;
use crate::local_tools::LocalToolRegistry;
use colored::*;
use serde_json::Value;
use std::collections::HashSet;
use std::fs;
use std::path::PathBuf;

pub fn generate_typescript_files(
    registry: &LocalToolRegistry,
    workspace: &PathBuf,
    verbose: bool,
) -> Result<()> {
    let servers_dir = workspace.join("servers");
    let local_dir = servers_dir.join("local");
    fs::create_dir_all(&local_dir)?;

    generate_client(workspace, verbose)?;

    let mut exports = Vec::new();
    for tool in registry.list() {
        generate_tool_wrapper(
            &local_dir,
            &tool.name,
            &tool.description,
            &tool.input_schema,
            verbose,
        )?;
        exports.push(format!("export * from './{}';", tool.name));
    }

    generate_index(&local_dir, &exports, verbose)?;
    Ok(())
}

fn generate_client(workspace: &PathBuf, verbose: bool) -> Result<()> {
    let client_path = workspace.join("client.ts");
    let client_code = r#"import { spawn } from 'child_process';

/**
 * Call a local tool by name with the given arguments.
 * This function spawns the cmd2ai tool runner and returns the result.
 */
export async function callLocalTool(name: string, args: any): Promise<any> {
  return new Promise((resolve, reject) => {
    const child = spawn('ai', ['--tool-exec', name], {
      stdio: ['pipe', 'pipe', 'pipe'],
    });

    let stdout = '';
    let stderr = '';

    child.stdout.on('data', (data) => {
      stdout += data.toString();
    });

    child.stderr.on('data', (data) => {
      stderr += data.toString();
    });

    child.on('error', (error) => {
      reject(new Error(`Failed to spawn tool runner: ${error.message}`));
    });

    child.on('close', (code) => {
      if (code !== 0) {
        try {
          const errorObj = JSON.parse(stderr || stdout);
          reject(new Error(errorObj.error || `Tool execution failed with code ${code}`));
        } catch {
          reject(new Error(stderr || `Tool execution failed with code ${code}`));
        }
        return;
      }

      try {
        const result = JSON.parse(stdout);
        resolve(result.result);
      } catch (error: any) {
        reject(new Error(`Failed to parse tool result: ${error.message}`));
      }
    });

    child.stdin.write(JSON.stringify(args), (err) => {
      if (err) {
        reject(new Error(`Failed to write to stdin: ${err.message}`));
        child.kill();
        return;
      }
      child.stdin.end();
    });
  });
}
"#;

    fs::write(&client_path, client_code)?;
    if verbose {
        eprintln!(
            "{}",
            format!(
                "[code_mode] Generated client.ts at {}",
                client_path.display()
            )
            .dimmed()
        );
    }
    Ok(())
}

fn generate_tool_wrapper(
    local_dir: &PathBuf,
    name: &str,
    description: &str,
    schema: &Value,
    verbose: bool,
) -> Result<()> {
    let tool_path = local_dir.join(format!("{}.ts", name));
    let params = extract_params_from_schema(schema);
    let param_signature = generate_param_signature(&params);
    let doc_block = generate_param_doc(description, &params);

    let tool_code = format!(
        r#"{doc}
import {{ callLocalTool }} from '../../client.ts';

export async function {name}(input: {signature}): Promise<any> {{
  return callLocalTool('{tool_name}', input);
}}
"#,
        doc = doc_block,
        name = name,
        signature = param_signature,
        tool_name = name
    );

    fs::write(&tool_path, tool_code)?;
    if verbose {
        eprintln!("{}", format!("[code_mode] Generated {}.ts", name).dimmed());
    }

    Ok(())
}

fn generate_index(local_dir: &PathBuf, exports: &[String], verbose: bool) -> Result<()> {
    let index_path = local_dir.join("index.ts");
    let content = format!("{}\n", exports.join("\n"));
    fs::write(&index_path, content)?;
    if verbose {
        eprintln!("{}", "[code_mode] Generated index.ts".dimmed());
    }
    Ok(())
}

fn extract_params_from_schema(schema: &Value) -> Vec<(String, String, Option<String>, bool)> {
    let properties = schema
        .get("properties")
        .and_then(|p| p.as_object())
        .cloned()
        .unwrap_or_default();

    let required_list: HashSet<String> = schema
        .get("required")
        .and_then(|r| r.as_array())
        .map(|arr| {
            arr.iter()
                .filter_map(|v| v.as_str().map(|s| s.to_string()))
                .collect()
        })
        .unwrap_or_default();

    properties
        .into_iter()
        .map(|(key, value)| {
            let ts_type = match value.get("type").and_then(|t| t.as_str()) {
                Some("string") => "string",
                Some("number") | Some("integer") => "number",
                Some("boolean") => "boolean",
                Some("array") => "any[]",
                Some("object") => "Record<string, any>",
                _ => "any",
            }
            .to_string();

            let description = value
                .get("description")
                .and_then(|d| d.as_str())
                .map(|s| s.to_string());

            let is_required = required_list.contains(&key);

            (key, ts_type, description, is_required)
        })
        .collect()
}

fn generate_param_signature(params: &[(String, String, Option<String>, bool)]) -> String {
    if params.is_empty() {
        return "any".to_string();
    }

    let fields: Vec<String> = params
        .iter()
        .map(|(name, ty, _, required)| {
            if *required {
                format!("{name}: {ty}")
            } else {
                format!("{name}?: {ty}")
            }
        })
        .collect();

    format!("{{ {} }}", fields.join("; "))
}

fn generate_param_doc(
    description: &str,
    params: &[(String, String, Option<String>, bool)],
) -> String {
    let mut lines = vec!["/**".to_string()];
    if !description.is_empty() {
        lines.push(format!(" * {}", description));
    } else {
        lines.push(" * Execute the local tool.".to_string());
    }

    for (name, _, desc, _) in params {
        if let Some(desc) = desc {
            lines.push(format!(" * @param {name} {desc}"));
        }
    }

    lines.push(" */".to_string());
    lines.join("\n")
}
