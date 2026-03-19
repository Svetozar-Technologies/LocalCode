use std::sync::Arc;
use tokio::task::JoinHandle;

use super::engine::AgentEngine;
use super::tools::{ToolContext, ToolRegistry};
use super::builtin;
use crate::llm::provider::LLMProvider;
use crate::CoreError;

/// Specialized agent roles for multi-agent orchestration
#[derive(Debug, Clone, Copy)]
pub enum AgentRole {
    Searcher,
    Coder,
    Reviewer,
}

impl AgentRole {
    /// Get the system prompt for this role
    pub fn system_prompt(&self) -> &str {
        match self {
            AgentRole::Searcher => {
                "You are a code search specialist. Your job is to find relevant code in the project. \
                 Use codebase_search, search_content, grep_search, find_files, and read_file to find relevant code. \
                 Return a structured summary of what you found, including file paths, line numbers, and brief descriptions. \
                 Be thorough but concise."
            }
            AgentRole::Coder => {
                "You are a code implementation specialist. Given a plan and context, write or modify code \
                 using write_file and edit_file. Be precise and match existing code style. \
                 Prefer edit_file for surgical changes to existing files. \
                 After making changes, verify by reading the file back. \
                 Summarize what you changed when done."
            }
            AgentRole::Reviewer => {
                "You are a code review specialist. Read the specified files and identify bugs, style issues, \
                 missing error handling, or logic errors. Be specific about line numbers and provide concrete \
                 suggestions for fixes. Focus on correctness, security, and maintainability. \
                 Rate overall code quality on a scale of 1-5."
            }
        }
    }

    /// Parse role from string
    pub fn from_str(s: &str) -> Option<Self> {
        match s.to_lowercase().as_str() {
            "searcher" | "search" => Some(AgentRole::Searcher),
            "coder" | "code" => Some(AgentRole::Coder),
            "reviewer" | "review" => Some(AgentRole::Reviewer),
            _ => None,
        }
    }
}

pub struct SubagentManager {
    provider: Arc<dyn LLMProvider>,
}

impl SubagentManager {
    pub fn new(provider: Arc<dyn LLMProvider>) -> Self {
        Self { provider }
    }

    pub fn spawn(
        &self,
        task: String,
        ctx: ToolContext,
    ) -> JoinHandle<Result<String, CoreError>> {
        let provider = self.provider.clone();

        tokio::spawn(async move {
            let mut registry = ToolRegistry::new();
            builtin::register_all(&mut registry);

            let mut engine = AgentEngine::new(provider, registry);

            let events = Arc::new(std::sync::Mutex::new(Vec::new()));
            let events_clone = events.clone();

            let result = engine
                .execute(&task, &ctx, &move |event| {
                    if let Ok(mut evts) = events_clone.lock() {
                        evts.push(event);
                    }
                })
                .await?;

            Ok(result)
        })
    }

    /// Spawn a subagent with a custom system prompt
    pub fn spawn_with_prompt(
        &self,
        task: String,
        system_prompt: String,
        ctx: ToolContext,
    ) -> JoinHandle<Result<String, CoreError>> {
        let provider = self.provider.clone();

        tokio::spawn(async move {
            let mut registry = ToolRegistry::new();
            builtin::register_all(&mut registry);

            let mut engine = AgentEngine::new(provider, registry)
                .with_system_prompt(system_prompt)
                .with_max_iterations(15);

            let events = Arc::new(std::sync::Mutex::new(Vec::new()));
            let events_clone = events.clone();

            let result = engine
                .execute(&task, &ctx, &move |event| {
                    if let Ok(mut evts) = events_clone.lock() {
                        evts.push(event);
                    }
                })
                .await?;

            Ok(result)
        })
    }

    /// Spawn a subagent with a specialized role
    pub fn spawn_role(
        &self,
        role: AgentRole,
        task: String,
        ctx: ToolContext,
    ) -> JoinHandle<Result<String, CoreError>> {
        self.spawn_with_prompt(task, role.system_prompt().to_string(), ctx)
    }

    /// Run multiple tasks in parallel and collect results
    pub async fn run_parallel(
        &self,
        tasks: Vec<(String, ToolContext)>,
    ) -> Vec<Result<String, CoreError>> {
        let handles: Vec<_> = tasks
            .into_iter()
            .map(|(task, ctx)| self.spawn(task, ctx))
            .collect();

        let mut results = Vec::new();
        for handle in handles {
            match handle.await {
                Ok(result) => results.push(result),
                Err(e) => results.push(Err(CoreError::Agent(e.to_string()))),
            }
        }

        results
    }
}
