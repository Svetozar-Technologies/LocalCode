use async_trait::async_trait;
use serde_json::json;

use crate::agent::tools::{Tool, ToolContext};
use crate::CoreError;

/// Codebase search tool — semantic search across indexed codebase
pub struct CodebaseSearchTool;

#[async_trait]
impl Tool for CodebaseSearchTool {
    fn name(&self) -> &str {
        "codebase_search"
    }

    fn description(&self) -> &str {
        "Search across the project codebase for relevant code snippets. Automatically builds an index if needed. \
         Use this to find implementations, definitions, usage patterns, or any code related to a query."
    }

    fn parameters_schema(&self) -> serde_json::Value {
        json!({
            "type": "object",
            "properties": {
                "query": {
                    "type": "string",
                    "description": "Search query describing what you're looking for (e.g., 'authentication middleware', 'database connection setup')"
                },
                "top_k": {
                    "type": "integer",
                    "description": "Number of results to return (default: 5)",
                    "default": 5
                }
            },
            "required": ["query"]
        })
    }

    async fn execute(
        &self,
        args: serde_json::Value,
        ctx: &ToolContext,
    ) -> Result<String, CoreError> {
        let query = args["query"]
            .as_str()
            .ok_or_else(|| CoreError::Agent("Missing 'query' parameter".to_string()))?;

        let top_k = args["top_k"].as_u64().unwrap_or(8) as usize;

        // Auto-index if needed (skip if index is <5 min old)
        let _ = crate::indexing::query::index_if_needed(&ctx.project_path, 300);

        match crate::indexing::query::query_codebase(&ctx.project_path, query, top_k) {
            Ok(results) => {
                if results.is_empty() {
                    Ok("No matching code found in the project.".to_string())
                } else {
                    let header = format!("Found {} results for '{}':\n\n", results.len(), query);
                    Ok(format!("{}{}", header, results.join("\n\n---\n\n")))
                }
            }
            Err(e) => Ok(format!("Search failed: {}. The project may need indexing first.", e)),
        }
    }
}

/// Update memory tool — agent can save learnings to persistent memory
pub struct UpdateMemoryTool;

#[async_trait]
impl Tool for UpdateMemoryTool {
    fn name(&self) -> &str {
        "update_memory"
    }

    fn description(&self) -> &str {
        "Save learnings, conventions, or facts about the project to persistent memory. \
         This information will be available in future sessions. Use this when you discover \
         important project patterns, conventions, commands, or facts worth remembering."
    }

    fn parameters_schema(&self) -> serde_json::Value {
        json!({
            "type": "object",
            "properties": {
                "category": {
                    "type": "string",
                    "enum": ["convention", "fact", "command"],
                    "description": "Type of memory: 'convention' for coding style rules, 'fact' for discovered information, 'command' for useful commands"
                },
                "content": {
                    "type": "string",
                    "description": "The information to remember"
                }
            },
            "required": ["category", "content"]
        })
    }

    async fn execute(
        &self,
        args: serde_json::Value,
        ctx: &ToolContext,
    ) -> Result<String, CoreError> {
        let category = args["category"]
            .as_str()
            .ok_or_else(|| CoreError::Agent("Missing 'category' parameter".to_string()))?;

        let content = args["content"]
            .as_str()
            .ok_or_else(|| CoreError::Agent("Missing 'content' parameter".to_string()))?;

        let mut memory = crate::agent::memory::MemoryManager::new();

        match category {
            "convention" => {
                memory.add_convention(&ctx.project_path, content);
            }
            "fact" => {
                memory.add_learned(&ctx.project_path, content);
            }
            "command" => {
                // Store commands as learned facts with a prefix
                memory.add_learned(&ctx.project_path, &format!("Useful command: {}", content));
            }
            _ => {
                memory.add_learned(&ctx.project_path, content);
            }
        }

        memory.save().map_err(|e| CoreError::Agent(format!("Failed to save memory: {}", e)))?;

        Ok(format!("Saved {} to project memory: {}", category, content))
    }
}
