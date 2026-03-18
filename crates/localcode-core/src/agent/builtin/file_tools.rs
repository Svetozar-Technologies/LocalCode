use async_trait::async_trait;
use crate::agent::tools::{Tool, ToolContext};
use crate::CoreError;
use crate::fs;

pub struct ReadFileTool;

#[async_trait]
impl Tool for ReadFileTool {
    fn name(&self) -> &str { "read_file" }
    fn description(&self) -> &str { "Read a file's contents" }
    fn parameters_schema(&self) -> serde_json::Value {
        serde_json::json!({
            "type": "object",
            "properties": {
                "path": { "type": "string", "description": "File path (relative to project or absolute)" }
            },
            "required": ["path"]
        })
    }
    async fn execute(&self, args: serde_json::Value, ctx: &ToolContext) -> Result<String, CoreError> {
        let path = args["path"].as_str().unwrap_or("");
        let full_path = fs::resolve_path(path, &ctx.project_path);
        match fs::read_file(&full_path) {
            Ok(content) => Ok(content.chars().take(8000).collect()),
            Err(e) => Ok(format!("Error reading {}: {}", full_path, e)),
        }
    }
}

pub struct WriteFileTool;

#[async_trait]
impl Tool for WriteFileTool {
    fn name(&self) -> &str { "write_file" }
    fn description(&self) -> &str { "Write content to a file (creates or overwrites)" }
    fn parameters_schema(&self) -> serde_json::Value {
        serde_json::json!({
            "type": "object",
            "properties": {
                "path": { "type": "string", "description": "File path" },
                "content": { "type": "string", "description": "Content to write" }
            },
            "required": ["path", "content"]
        })
    }
    async fn execute(&self, args: serde_json::Value, ctx: &ToolContext) -> Result<String, CoreError> {
        let path = args["path"].as_str().unwrap_or("");
        let content = args["content"].as_str().unwrap_or("");
        let full_path = fs::resolve_path(path, &ctx.project_path);
        match fs::write_file(&full_path, content) {
            Ok(_) => Ok(format!("Successfully wrote to {}", full_path)),
            Err(e) => Ok(format!("Error writing {}: {}", full_path, e)),
        }
    }
}

pub struct EditFileTool;

#[async_trait]
impl Tool for EditFileTool {
    fn name(&self) -> &str { "edit_file" }
    fn description(&self) -> &str { "Replace text in a file" }
    fn parameters_schema(&self) -> serde_json::Value {
        serde_json::json!({
            "type": "object",
            "properties": {
                "path": { "type": "string", "description": "File path" },
                "old_text": { "type": "string", "description": "Text to find" },
                "new_text": { "type": "string", "description": "Replacement text" }
            },
            "required": ["path", "old_text", "new_text"]
        })
    }
    async fn execute(&self, args: serde_json::Value, ctx: &ToolContext) -> Result<String, CoreError> {
        let path = args["path"].as_str().unwrap_or("");
        let old_text = args["old_text"].as_str().unwrap_or("");
        let new_text = args["new_text"].as_str().unwrap_or("");
        let full_path = fs::resolve_path(path, &ctx.project_path);
        match fs::edit_file(&full_path, old_text, new_text) {
            Ok(true) => Ok(format!("Successfully edited {}", full_path)),
            Ok(false) => Ok(format!("Text not found in {}", full_path)),
            Err(e) => Ok(format!("Error editing {}: {}", full_path, e)),
        }
    }
}

pub struct ListDirTool;

#[async_trait]
impl Tool for ListDirTool {
    fn name(&self) -> &str { "list_dir" }
    fn description(&self) -> &str { "List directory contents" }
    fn parameters_schema(&self) -> serde_json::Value {
        serde_json::json!({
            "type": "object",
            "properties": {
                "path": { "type": "string", "description": "Directory path (default: project root)" }
            }
        })
    }
    async fn execute(&self, args: serde_json::Value, ctx: &ToolContext) -> Result<String, CoreError> {
        let path = args["path"].as_str().unwrap_or(".");
        let full_path = fs::resolve_path(path, &ctx.project_path);
        match fs::read_dir(&full_path) {
            Ok(entries) => {
                let items: Vec<String> = entries
                    .iter()
                    .map(|e| {
                        let ft = if e.is_dir { "dir" } else { "file" };
                        format!("[{}] {}", ft, e.name)
                    })
                    .collect();
                Ok(items.join("\n"))
            }
            Err(e) => Ok(format!("Error listing {}: {}", full_path, e)),
        }
    }
}

pub struct CreateFileTool;

#[async_trait]
impl Tool for CreateFileTool {
    fn name(&self) -> &str { "create_file" }
    fn description(&self) -> &str { "Create a new empty file" }
    fn parameters_schema(&self) -> serde_json::Value {
        serde_json::json!({
            "type": "object",
            "properties": {
                "path": { "type": "string", "description": "File path to create" }
            },
            "required": ["path"]
        })
    }
    async fn execute(&self, args: serde_json::Value, ctx: &ToolContext) -> Result<String, CoreError> {
        let path = args["path"].as_str().unwrap_or("");
        let full_path = fs::resolve_path(path, &ctx.project_path);
        match fs::create_file(&full_path) {
            Ok(_) => Ok(format!("Created {}", full_path)),
            Err(e) => Ok(format!("Error creating {}: {}", full_path, e)),
        }
    }
}

pub struct DeleteFileTool;

#[async_trait]
impl Tool for DeleteFileTool {
    fn name(&self) -> &str { "delete_file" }
    fn description(&self) -> &str { "Delete a file or directory" }
    fn parameters_schema(&self) -> serde_json::Value {
        serde_json::json!({
            "type": "object",
            "properties": {
                "path": { "type": "string", "description": "Path to delete" }
            },
            "required": ["path"]
        })
    }
    async fn execute(&self, args: serde_json::Value, ctx: &ToolContext) -> Result<String, CoreError> {
        let path = args["path"].as_str().unwrap_or("");
        let full_path = fs::resolve_path(path, &ctx.project_path);
        match fs::delete_entry(&full_path) {
            Ok(_) => Ok(format!("Deleted {}", full_path)),
            Err(e) => Ok(format!("Error deleting {}: {}", full_path, e)),
        }
    }
}
