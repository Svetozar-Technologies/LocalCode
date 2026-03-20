use async_trait::async_trait;
use serde_json::json;

use crate::agent::tools::{Tool, ToolContext};
use crate::agent::subagent::{AgentRole, SubagentManager};
use crate::CoreError;

/// Tool that allows the main agent to dispatch specialized subagents
pub struct DispatchSubagentTool;

#[async_trait]
impl Tool for DispatchSubagentTool {
    fn name(&self) -> &str {
        "dispatch_subagent"
    }

    fn description(&self) -> &str {
        "Dispatch a specialized subagent for a specific task. Use 'searcher' to find code, \
         'coder' to implement changes, or 'reviewer' to review code quality. \
         Useful for parallelizing work or delegating specialized tasks."
    }

    fn parameters_schema(&self) -> serde_json::Value {
        json!({
            "type": "object",
            "properties": {
                "role": {
                    "type": "string",
                    "enum": ["searcher", "coder", "reviewer"],
                    "description": "The role of the subagent: 'searcher' finds code, 'coder' writes code, 'reviewer' reviews code"
                },
                "task": {
                    "type": "string",
                    "description": "The task to assign to the subagent"
                }
            },
            "required": ["role", "task"]
        })
    }

    async fn execute(
        &self,
        args: serde_json::Value,
        ctx: &ToolContext,
    ) -> Result<String, CoreError> {
        let role_str = args["role"]
            .as_str()
            .ok_or_else(|| CoreError::Agent("Missing 'role' parameter".to_string()))?;

        let task = args["task"]
            .as_str()
            .ok_or_else(|| CoreError::Agent("Missing 'task' parameter".to_string()))?;

        let role = AgentRole::parse_role(role_str)
            .ok_or_else(|| CoreError::Agent(format!(
                "Invalid role '{}'. Must be one of: searcher, coder, reviewer", role_str
            )))?;

        let provider = ctx.provider.as_ref()
            .ok_or_else(|| CoreError::Agent(
                "No LLM provider available for subagent. Cannot dispatch.".to_string()
            ))?;

        let manager = SubagentManager::new(provider.clone());

        let subagent_ctx = ToolContext {
            project_path: ctx.project_path.clone(),
            current_file: ctx.current_file.clone(),
            provider: ctx.provider.clone(),
        };

        let handle = manager.spawn_role(role, task.to_string(), subagent_ctx);

        match handle.await {
            Ok(Ok(result)) => Ok(format!(
                "[Subagent ({}) completed]\n\n{}",
                role_str, result
            )),
            Ok(Err(e)) => Ok(format!(
                "[Subagent ({}) failed]: {}",
                role_str, e
            )),
            Err(e) => Ok(format!(
                "[Subagent ({}) task panicked]: {}",
                role_str, e
            )),
        }
    }
}
