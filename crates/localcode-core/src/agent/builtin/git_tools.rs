use async_trait::async_trait;
use crate::agent::tools::{Tool, ToolContext};
use crate::CoreError;

pub struct GitStatusTool;

#[async_trait]
impl Tool for GitStatusTool {
    fn name(&self) -> &str { "git_status" }
    fn description(&self) -> &str { "Show git status (modified, untracked, staged files)" }
    fn parameters_schema(&self) -> serde_json::Value {
        serde_json::json!({ "type": "object", "properties": {} })
    }
    async fn execute(&self, _args: serde_json::Value, ctx: &ToolContext) -> Result<String, CoreError> {
        let statuses = crate::git::git_status(&ctx.project_path)?;
        if statuses.is_empty() {
            Ok("Working tree clean".to_string())
        } else {
            let lines: Vec<String> = statuses
                .iter()
                .map(|s| format!("{}: {}", s.status, s.path))
                .collect();
            Ok(lines.join("\n"))
        }
    }
}

pub struct GitDiffTool;

#[async_trait]
impl Tool for GitDiffTool {
    fn name(&self) -> &str { "git_diff" }
    fn description(&self) -> &str { "Show git diff of working tree changes" }
    fn parameters_schema(&self) -> serde_json::Value {
        serde_json::json!({ "type": "object", "properties": {} })
    }
    async fn execute(&self, _args: serde_json::Value, ctx: &ToolContext) -> Result<String, CoreError> {
        let diff = crate::git::git_diff(&ctx.project_path)?;
        if diff.is_empty() {
            Ok("No changes".to_string())
        } else {
            Ok(diff.chars().take(4000).collect())
        }
    }
}

pub struct GitCommitTool;

#[async_trait]
impl Tool for GitCommitTool {
    fn name(&self) -> &str { "git_commit" }
    fn description(&self) -> &str { "Stage all changes and create a git commit" }
    fn parameters_schema(&self) -> serde_json::Value {
        serde_json::json!({
            "type": "object",
            "properties": {
                "message": { "type": "string", "description": "Commit message" }
            },
            "required": ["message"]
        })
    }
    async fn execute(&self, args: serde_json::Value, ctx: &ToolContext) -> Result<String, CoreError> {
        let message = args["message"].as_str().unwrap_or("Auto-commit by LocalCode");
        crate::git::staging::git_add_all(&ctx.project_path)?;
        let hash = crate::git::staging::git_commit(&ctx.project_path, message)?;
        Ok(format!("Committed: {} ({})", message, hash))
    }
}

pub struct GitLogTool;

#[async_trait]
impl Tool for GitLogTool {
    fn name(&self) -> &str { "git_log" }
    fn description(&self) -> &str { "Show recent git commit history" }
    fn parameters_schema(&self) -> serde_json::Value {
        serde_json::json!({
            "type": "object",
            "properties": {
                "count": { "type": "integer", "description": "Number of commits to show (default: 10)" }
            }
        })
    }
    async fn execute(&self, args: serde_json::Value, ctx: &ToolContext) -> Result<String, CoreError> {
        let count = args["count"].as_u64().unwrap_or(10) as usize;
        let entries = crate::git::git_log(&ctx.project_path, count)?;
        let lines: Vec<String> = entries
            .iter()
            .map(|e| format!("{} {} ({})", e.hash, e.message, e.author))
            .collect();
        Ok(lines.join("\n"))
    }
}
