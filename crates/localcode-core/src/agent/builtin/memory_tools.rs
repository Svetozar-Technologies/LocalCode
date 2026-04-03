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

/// Update memory tool — stores learnings in Engram cognitive memory
pub struct UpdateMemoryTool;

#[async_trait]
impl Tool for UpdateMemoryTool {
    fn name(&self) -> &str {
        "update_memory"
    }

    fn description(&self) -> &str {
        "Save learnings, conventions, or facts about the project to persistent cognitive memory. \
         This information will be available in future sessions and is searchable semantically. \
         Use this when you discover important project patterns, conventions, commands, or facts worth remembering."
    }

    fn parameters_schema(&self) -> serde_json::Value {
        json!({
            "type": "object",
            "properties": {
                "category": {
                    "type": "string",
                    "enum": ["convention", "fact", "command", "procedure"],
                    "description": "Type of memory: 'convention' for coding style rules, 'fact' for discovered information, 'command' for useful commands, 'procedure' for how-to steps"
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

        // Use Engram if available, fallback to legacy
        if let (Some(ref engram), Some(agent_id)) = (&ctx.engram, ctx.engram_agent_id) {
            let node = match category {
                "convention" => engram_core::MemoryNode::new(
                    agent_id,
                    engram_core::MemoryType::Semantic,
                    content,
                )
                .tag("convention")
                .with_visibility(engram_core::Visibility::Global),

                "procedure" => engram_core::MemoryNode::new(
                    agent_id,
                    engram_core::MemoryType::Procedural,
                    content,
                )
                .tag("procedure"),

                "command" => engram_core::MemoryNode::new(
                    agent_id,
                    engram_core::MemoryType::Procedural,
                    format!("Useful command: {}", content),
                )
                .tag("command"),

                _ => engram_core::MemoryNode::new(
                    agent_id,
                    engram_core::MemoryType::Semantic,
                    content,
                )
                .tag("learned"),
            };

            engram.store(node)
                .map_err(|e| CoreError::Agent(format!("Engram store failed: {}", e)))?;

            Ok(format!("Saved {} to cognitive memory: {}", category, content))
        } else {
            // Fallback to legacy MemoryManager
            let mut memory = crate::agent::memory::MemoryManager::new();

            match category {
                "convention" => memory.add_convention(&ctx.project_path, content),
                "fact" => memory.add_learned(&ctx.project_path, content),
                "command" => memory.add_learned(&ctx.project_path, &format!("Useful command: {}", content)),
                _ => memory.add_learned(&ctx.project_path, content),
            }

            memory.save().map_err(|e| CoreError::Agent(format!("Failed to save memory: {}", e)))?;
            Ok(format!("Saved {} to project memory: {}", category, content))
        }
    }
}

/// Memory recall tool — semantic search across agent's long-term memory
pub struct MemoryRecallTool;

#[async_trait]
impl Tool for MemoryRecallTool {
    fn name(&self) -> &str {
        "memory_recall"
    }

    fn description(&self) -> &str {
        "Search your cognitive memory for relevant information from past sessions. \
         Returns facts, conventions, session summaries, and procedures that match the query. \
         Use this to recall previously learned information about the project."
    }

    fn parameters_schema(&self) -> serde_json::Value {
        json!({
            "type": "object",
            "properties": {
                "query": {
                    "type": "string",
                    "description": "What to search for in memory (e.g., 'database setup', 'deployment process', 'coding conventions')"
                },
                "limit": {
                    "type": "integer",
                    "description": "Maximum results to return (default: 5)",
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

        let limit = args["limit"].as_u64().unwrap_or(5) as usize;

        if let (Some(ref engram), Some(agent_id)) = (&ctx.engram, ctx.engram_agent_id) {
            let results = engram.recall(
                engram_core::RecallQuery::new(query)
                    .with_agent(agent_id)
                    .with_limit(limit),
            );

            if results.is_empty() {
                Ok("No matching memories found.".to_string())
            } else {
                let mut output = format!("Found {} memories for '{}':\n\n", results.len(), query);
                for (i, scored) in results.iter().enumerate() {
                    let tags: Vec<&str> = scored.memory.tags.iter().map(|t| t.as_str()).collect();
                    output.push_str(&format!(
                        "{}. [score: {:.2}] [{}] {}\n",
                        i + 1,
                        scored.score,
                        tags.join(", "),
                        scored.memory.content,
                    ));
                }
                Ok(output)
            }
        } else {
            Ok("Cognitive memory not available. Use codebase_search instead.".to_string())
        }
    }
}

/// Memory forget tool — delete a specific memory by searching and removing
pub struct MemoryForgetTool;

#[async_trait]
impl Tool for MemoryForgetTool {
    fn name(&self) -> &str {
        "memory_forget"
    }

    fn description(&self) -> &str {
        "Remove outdated or incorrect information from memory. \
         Searches for the most relevant match and deletes it. \
         Use this when previously stored information is no longer accurate."
    }

    fn parameters_schema(&self) -> serde_json::Value {
        json!({
            "type": "object",
            "properties": {
                "query": {
                    "type": "string",
                    "description": "Description of the memory to forget (e.g., 'the old database password convention')"
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

        if let (Some(ref engram), Some(agent_id)) = (&ctx.engram, ctx.engram_agent_id) {
            let results = engram.recall(
                engram_core::RecallQuery::new(query)
                    .with_agent(agent_id)
                    .with_limit(1),
            );

            if let Some(top) = results.first() {
                let content = top.memory.content.to_string();
                engram.forget(&top.memory.id)
                    .map_err(|e| CoreError::Agent(format!("Forget failed: {}", e)))?;
                Ok(format!("Deleted memory: {}", content))
            } else {
                Ok("No matching memory found to delete.".to_string())
            }
        } else {
            Ok("Cognitive memory not available.".to_string())
        }
    }
}
