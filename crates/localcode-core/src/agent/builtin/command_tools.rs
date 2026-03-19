use async_trait::async_trait;
use crate::agent::tools::{Tool, ToolContext};
use crate::CoreError;
use crate::terminal::process::run_command;

pub struct RunCommandTool;

#[async_trait]
impl Tool for RunCommandTool {
    fn name(&self) -> &str { "run_command" }
    fn description(&self) -> &str { "Run a shell command and get output" }
    fn parameters_schema(&self) -> serde_json::Value {
        serde_json::json!({
            "type": "object",
            "properties": {
                "command": { "type": "string", "description": "Shell command to execute" }
            },
            "required": ["command"]
        })
    }
    async fn execute(&self, args: serde_json::Value, ctx: &ToolContext) -> Result<String, CoreError> {
        let command = args["command"].as_str().unwrap_or("");
        let output = run_command(command, &ctx.project_path)?;

        let mut result = String::new();

        if !output.success {
            // Error: put stderr FIRST with clear marker
            result.push_str(&format!("[ERROR exit_code={}]\n", output.exit_code));
            if !output.stderr.is_empty() {
                result.push_str(&output.stderr);
            }
            if !output.stdout.is_empty() {
                result.push_str("\nSTDOUT:\n");
                result.push_str(&output.stdout);
            }
        } else {
            // Success: stdout first, stderr as note
            if !output.stdout.is_empty() {
                result.push_str(&output.stdout);
            }
            if !output.stderr.is_empty() {
                // Some tools write to stderr even on success (pygame, npm)
                result.push_str("\n[stderr]: ");
                result.push_str(&output.stderr);
            }
        }

        if result.is_empty() {
            result = format!("Command completed (exit code: {})", output.exit_code);
        }

        Ok(result.chars().take(4000).collect())
    }
}

// --- GrepTool ---

pub struct GrepTool;

#[async_trait]
impl Tool for GrepTool {
    fn name(&self) -> &str { "grep_search" }

    fn description(&self) -> &str {
        "Search file contents using grep with regex support. More powerful than search_content for complex patterns. Returns matching lines with file:line:content format."
    }

    fn parameters_schema(&self) -> serde_json::Value {
        serde_json::json!({
            "type": "object",
            "properties": {
                "pattern": {
                    "type": "string",
                    "description": "Regex pattern to search for"
                },
                "path": {
                    "type": "string",
                    "description": "Directory or file to search in (default: '.')"
                },
                "include": {
                    "type": "string",
                    "description": "File glob pattern to filter (e.g., '*.rs', '*.ts')"
                }
            },
            "required": ["pattern"]
        })
    }

    async fn execute(&self, args: serde_json::Value, ctx: &ToolContext) -> Result<String, CoreError> {
        let pattern = args["pattern"].as_str().unwrap_or("");
        let path = args["path"].as_str().unwrap_or(".");
        let include = args["include"].as_str();

        if pattern.is_empty() {
            return Err(CoreError::Agent("Missing 'pattern' parameter".to_string()));
        }

        let mut cmd = format!("grep -rn '{}' {}", pattern, path);
        if let Some(glob) = include {
            cmd = format!("grep -rn --include='{}' '{}' {}", glob, pattern, path);
        }

        let output = run_command(&cmd, &ctx.project_path)?;

        let result = if !output.stdout.is_empty() {
            // Limit to 50 results
            let lines: Vec<&str> = output.stdout.lines().take(50).collect();
            let truncated = if output.stdout.lines().count() > 50 {
                format!("{}\n\n... ({} total matches, showing first 50)", lines.join("\n"), output.stdout.lines().count())
            } else {
                lines.join("\n")
            };
            truncated
        } else if !output.stderr.is_empty() {
            format!("grep error: {}", output.stderr)
        } else {
            "No matches found.".to_string()
        };

        Ok(result.chars().take(4000).collect())
    }
}

// --- FindTool ---

pub struct FindTool;

#[async_trait]
impl Tool for FindTool {
    fn name(&self) -> &str { "find_files" }

    fn description(&self) -> &str {
        "Find files and directories by name pattern. Supports wildcards."
    }

    fn parameters_schema(&self) -> serde_json::Value {
        serde_json::json!({
            "type": "object",
            "properties": {
                "path": {
                    "type": "string",
                    "description": "Starting directory (default: '.')"
                },
                "name": {
                    "type": "string",
                    "description": "Filename pattern to match (e.g., '*.rs', 'test_*')"
                },
                "type": {
                    "type": "string",
                    "enum": ["f", "d"],
                    "description": "Type filter: 'f' for files, 'd' for directories"
                }
            },
            "required": ["name"]
        })
    }

    async fn execute(&self, args: serde_json::Value, ctx: &ToolContext) -> Result<String, CoreError> {
        let path = args["path"].as_str().unwrap_or(".");
        let name = args["name"].as_str().unwrap_or("*");
        let file_type = args["type"].as_str();

        let mut cmd = format!("find {} -name '{}'", path, name);
        if let Some(t) = file_type {
            cmd.push_str(&format!(" -type {}", t));
        }

        let output = run_command(&cmd, &ctx.project_path)?;

        let result = if !output.stdout.is_empty() {
            let lines: Vec<&str> = output.stdout.lines().take(100).collect();
            let truncated = if output.stdout.lines().count() > 100 {
                format!("{}\n\n... ({} total results, showing first 100)", lines.join("\n"), output.stdout.lines().count())
            } else {
                lines.join("\n")
            };
            truncated
        } else {
            "No files found matching the pattern.".to_string()
        };

        Ok(result.chars().take(4000).collect())
    }
}

// --- CurlTool ---

pub struct CurlTool;

#[async_trait]
impl Tool for CurlTool {
    fn name(&self) -> &str { "http_request" }

    fn description(&self) -> &str {
        "Make HTTP requests to APIs or fetch web content."
    }

    fn parameters_schema(&self) -> serde_json::Value {
        serde_json::json!({
            "type": "object",
            "properties": {
                "url": {
                    "type": "string",
                    "description": "URL to request"
                },
                "method": {
                    "type": "string",
                    "enum": ["GET", "POST"],
                    "description": "HTTP method (default: GET)"
                },
                "body": {
                    "type": "string",
                    "description": "Request body for POST requests"
                }
            },
            "required": ["url"]
        })
    }

    async fn execute(&self, args: serde_json::Value, ctx: &ToolContext) -> Result<String, CoreError> {
        let url = args["url"].as_str()
            .ok_or_else(|| CoreError::Agent("Missing 'url' parameter".to_string()))?;
        let method = args["method"].as_str().unwrap_or("GET");
        let body = args["body"].as_str();

        let mut cmd = format!("curl -s -L -m 10 -X {}", method);
        if let Some(b) = body {
            cmd.push_str(&format!(" -H 'Content-Type: application/json' -d '{}'", b));
        }
        cmd.push_str(&format!(" '{}'", url));

        let output = run_command(&cmd, &ctx.project_path)?;

        let result = if !output.stdout.is_empty() {
            output.stdout
        } else if !output.stderr.is_empty() {
            format!("Request error: {}", output.stderr)
        } else {
            "Empty response.".to_string()
        };

        Ok(result.chars().take(4000).collect())
    }
}

// --- SedTool ---

pub struct SedTool;

#[async_trait]
impl Tool for SedTool {
    fn name(&self) -> &str { "sed_replace" }

    fn description(&self) -> &str {
        "Find and replace text in a file using sed regex patterns. For complex multi-line edits use edit_file instead."
    }

    fn parameters_schema(&self) -> serde_json::Value {
        serde_json::json!({
            "type": "object",
            "properties": {
                "file": {
                    "type": "string",
                    "description": "File path to modify"
                },
                "pattern": {
                    "type": "string",
                    "description": "Regex pattern to find"
                },
                "replacement": {
                    "type": "string",
                    "description": "Replacement string"
                },
                "global": {
                    "type": "boolean",
                    "description": "Replace all occurrences (default: true)"
                }
            },
            "required": ["file", "pattern", "replacement"]
        })
    }

    async fn execute(&self, args: serde_json::Value, ctx: &ToolContext) -> Result<String, CoreError> {
        let file = args["file"].as_str()
            .ok_or_else(|| CoreError::Agent("Missing 'file' parameter".to_string()))?;
        let pattern = args["pattern"].as_str()
            .ok_or_else(|| CoreError::Agent("Missing 'pattern' parameter".to_string()))?;
        let replacement = args["replacement"].as_str()
            .ok_or_else(|| CoreError::Agent("Missing 'replacement' parameter".to_string()))?;
        let global = args["global"].as_bool().unwrap_or(true);

        let flags = if global { "g" } else { "" };
        let cmd = format!("sed -i '' 's/{}/{}/{}' '{}'", pattern, replacement, flags, file);

        let output = run_command(&cmd, &ctx.project_path)?;

        if !output.stderr.is_empty() {
            Ok(format!("sed error: {}", output.stderr))
        } else {
            Ok(format!("Replaced '{}' with '{}' in {}", pattern, replacement, file))
        }
    }
}

// --- WcTool ---

pub struct WcTool;

#[async_trait]
impl Tool for WcTool {
    fn name(&self) -> &str { "count_lines" }

    fn description(&self) -> &str {
        "Count lines in files. Useful for understanding file/project size."
    }

    fn parameters_schema(&self) -> serde_json::Value {
        serde_json::json!({
            "type": "object",
            "properties": {
                "path": {
                    "type": "string",
                    "description": "File or directory path to count lines in"
                }
            },
            "required": ["path"]
        })
    }

    async fn execute(&self, args: serde_json::Value, ctx: &ToolContext) -> Result<String, CoreError> {
        let path = args["path"].as_str()
            .ok_or_else(|| CoreError::Agent("Missing 'path' parameter".to_string()))?;

        // Check if path is a file or directory
        let full_path = std::path::Path::new(&ctx.project_path).join(path);
        let cmd = if full_path.is_dir() {
            format!("find '{}' -type f -name '*.rs' -o -name '*.py' -o -name '*.ts' -o -name '*.js' -o -name '*.go' -o -name '*.java' -o -name '*.c' -o -name '*.cpp' -o -name '*.h' | xargs wc -l 2>/dev/null | tail -1", path)
        } else {
            format!("wc -l '{}'", path)
        };

        let output = run_command(&cmd, &ctx.project_path)?;

        let result = if !output.stdout.is_empty() {
            output.stdout.trim().to_string()
        } else {
            "Could not count lines.".to_string()
        };

        Ok(result.chars().take(4000).collect())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::agent::tools::ToolContext;

    fn test_ctx() -> ToolContext {
        ToolContext {
            project_path: std::env::temp_dir().display().to_string(),
            current_file: None,
            provider: None,
        }
    }

    #[tokio::test]
    async fn test_run_command_success_output() {
        let tool = RunCommandTool;
        let args = serde_json::json!({"command": "echo hello"});
        let result = tool.execute(args, &test_ctx()).await.unwrap();
        assert!(result.contains("hello"));
        assert!(!result.starts_with("[ERROR"));
    }

    #[tokio::test]
    async fn test_run_command_error_prefix() {
        let tool = RunCommandTool;
        let args = serde_json::json!({"command": "python3 -c \"raise ValueError('boom')\""});
        let result = tool.execute(args, &test_ctx()).await.unwrap();
        assert!(result.starts_with("[ERROR exit_code="));
        assert!(result.contains("ValueError"));
    }

    #[tokio::test]
    async fn test_run_command_error_stderr_first() {
        let tool = RunCommandTool;
        let args = serde_json::json!({
            "command": "echo stdout_msg && echo stderr_msg >&2 && exit 1"
        });
        let result = tool.execute(args, &test_ctx()).await.unwrap();
        assert!(result.starts_with("[ERROR exit_code=1]"));
        let stderr_pos = result.find("stderr_msg").unwrap();
        let stdout_pos = result.find("stdout_msg").unwrap();
        assert!(stderr_pos < stdout_pos, "stderr should come before stdout on error");
    }

    #[tokio::test]
    async fn test_run_command_success_stderr_as_note() {
        let tool = RunCommandTool;
        let args = serde_json::json!({
            "command": "echo ok && echo debug >&2"
        });
        let result = tool.execute(args, &test_ctx()).await.unwrap();
        assert!(!result.starts_with("[ERROR"));
        assert!(result.contains("ok"));
        assert!(result.contains("[stderr]:"));
        assert!(result.contains("debug"));
    }

    #[tokio::test]
    async fn test_run_command_empty_output() {
        let tool = RunCommandTool;
        let args = serde_json::json!({"command": "true"});
        let result = tool.execute(args, &test_ctx()).await.unwrap();
        assert!(result.contains("Command completed (exit code: 0)"));
    }

    #[tokio::test]
    async fn test_run_command_nonexistent_file_error() {
        let tool = RunCommandTool;
        let args = serde_json::json!({"command": "python3 /tmp/nonexistent_xyz_test.py"});
        let result = tool.execute(args, &test_ctx()).await.unwrap();
        assert!(result.starts_with("[ERROR exit_code="));
    }
}
