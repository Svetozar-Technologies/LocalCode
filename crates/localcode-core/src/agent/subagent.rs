use std::sync::Arc;
use tokio::task::JoinHandle;

use super::engine::{AgentEngine, AgentEvent};
use super::tools::{ToolContext, ToolRegistry};
use super::builtin;
use crate::llm::provider::LLMProvider;
use crate::CoreError;

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
