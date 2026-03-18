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
        if !output.stdout.is_empty() {
            result.push_str(&output.stdout);
        }
        if !output.stderr.is_empty() {
            if !result.is_empty() {
                result.push_str("\nSTDERR: ");
            }
            result.push_str(&output.stderr);
        }
        if result.is_empty() {
            result = format!("Command completed with exit code: {}", output.exit_code);
        }

        Ok(result.chars().take(4000).collect())
    }
}
