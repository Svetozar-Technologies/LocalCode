use async_trait::async_trait;
use crate::agent::tools::{Tool, ToolContext};
use crate::CoreError;

pub struct SearchFilesTool;

#[async_trait]
impl Tool for SearchFilesTool {
    fn name(&self) -> &str { "search_files" }
    fn description(&self) -> &str { "Search for files by name pattern" }
    fn parameters_schema(&self) -> serde_json::Value {
        serde_json::json!({
            "type": "object",
            "properties": {
                "pattern": { "type": "string", "description": "File name pattern to search for" }
            },
            "required": ["pattern"]
        })
    }
    async fn execute(&self, args: serde_json::Value, ctx: &ToolContext) -> Result<String, CoreError> {
        let pattern = args["pattern"].as_str().unwrap_or("");
        let results = crate::search::search_files(&ctx.project_path, pattern, 20)?;
        if results.is_empty() {
            Ok("No files found".to_string())
        } else {
            Ok(results.join("\n"))
        }
    }
}

pub struct SearchContentTool;

#[async_trait]
impl Tool for SearchContentTool {
    fn name(&self) -> &str { "search_content" }
    fn description(&self) -> &str { "Search file contents for a text pattern" }
    fn parameters_schema(&self) -> serde_json::Value {
        serde_json::json!({
            "type": "object",
            "properties": {
                "pattern": { "type": "string", "description": "Text pattern to search for in file contents" }
            },
            "required": ["pattern"]
        })
    }
    async fn execute(&self, args: serde_json::Value, ctx: &ToolContext) -> Result<String, CoreError> {
        let pattern = args["pattern"].as_str().unwrap_or("");
        let results = crate::search::search_content(&ctx.project_path, pattern, 30)?;
        if results.is_empty() {
            Ok("No matches found".to_string())
        } else {
            let lines: Vec<String> = results
                .iter()
                .map(|r| format!("{}:{}: {}", r.file, r.line, r.content))
                .collect();
            Ok(lines.join("\n"))
        }
    }
}

pub struct GlobFilesTool;

#[async_trait]
impl Tool for GlobFilesTool {
    fn name(&self) -> &str { "glob" }
    fn description(&self) -> &str { "Find files matching a glob pattern" }
    fn parameters_schema(&self) -> serde_json::Value {
        serde_json::json!({
            "type": "object",
            "properties": {
                "pattern": { "type": "string", "description": "Glob pattern (e.g., **/*.rs)" }
            },
            "required": ["pattern"]
        })
    }
    async fn execute(&self, args: serde_json::Value, ctx: &ToolContext) -> Result<String, CoreError> {
        let pattern = args["pattern"].as_str().unwrap_or("");
        let results = crate::search::glob_files(&ctx.project_path, pattern, 50)?;
        if results.is_empty() {
            Ok("No files found".to_string())
        } else {
            Ok(results.join("\n"))
        }
    }
}
