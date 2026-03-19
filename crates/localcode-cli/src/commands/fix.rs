use std::io::{self, Write};
use std::sync::Arc;
use localcode_core::agent::{AgentEngine, AgentEvent, ToolContext, ToolRegistry};
use localcode_core::agent::builtin;
use localcode_core::config::Config;
use localcode_core::llm::provider::*;
use localcode_core::llm::local::LocalProvider;
use localcode_core::llm::openai::OpenAIProvider;
use localcode_core::llm::anthropic::AnthropicProvider;

use crate::rendering;

pub async fn run_fix(
    error: &str,
    provider_name: Option<&str>,
) -> Result<(), Box<dyn std::error::Error>> {
    let config = Config::load().unwrap_or_default();
    let name = provider_name.unwrap_or(&config.default_provider);
    let cwd = std::env::current_dir()?.display().to_string();

    let provider: Arc<dyn LLMProvider> = match name {
        "openai" => Arc::new(OpenAIProvider::new(&config.get_openai_key(), &config.get_openai_model())),
        "anthropic" => Arc::new(AnthropicProvider::new(&config.get_anthropic_key(), &config.get_anthropic_model())),
        "ollama" | "local" => {
            let server_url = &config.providers.local.server_url;
            let base_url = format!("{}/v1", server_url);
            let model = config.providers.local.active_catalog_model
                .clone()
                .unwrap_or_else(|| "qwen2.5-coder:14b".to_string());
            Arc::new(OpenAIProvider::with_base_url("ollama", &base_url, &model))
        }
        _ => Arc::new(LocalProvider::new()),
    };

    let mut registry = ToolRegistry::new();
    builtin::register_all(&mut registry);

    let mut engine = AgentEngine::new(provider.clone(), registry)
        .with_system_prompt(
            "You are LocalCode, an AI coding assistant. Fix the error described by the user. \
             Read relevant files, identify the issue, and fix it."
                .to_string(),
        );

    let ctx = ToolContext {
        project_path: cwd,
        current_file: None,
        provider: Some(provider),
    };

    let task = format!(
        "Fix this error:\n\n{}\n\n\
         Find the relevant files, identify the root cause, and fix the issue.",
        error
    );

    println!();
    rendering::print_info("Diagnosing error...");
    println!();

    engine
        .execute(&task, &ctx, &|event| match event {
            AgentEvent::Step(step) => {
                if step.step_type == "tool_call" {
                    if let Some(ref tool) = step.tool {
                        rendering::print_tool_call(
                            tool,
                            step.args.as_ref().unwrap_or(&serde_json::json!({})),
                        );
                    }
                } else if step.step_type == "tool_result" {
                    if let (Some(ref tool), Some(ref result)) = (&step.tool, &step.result) {
                        rendering::print_tool_result(tool, result);
                    }
                }
            }
            AgentEvent::TextChunk(text) => {
                print!("{}", text);
                io::stdout().flush().unwrap();
            }
            AgentEvent::Done(text) => {
                println!();
                rendering::print_markdown(&text);
            }
            AgentEvent::Error(e) => {
                rendering::print_error(&e);
            }
        })
        .await?;

    println!();
    Ok(())
}
