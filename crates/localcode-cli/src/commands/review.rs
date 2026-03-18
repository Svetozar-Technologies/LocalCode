use std::sync::Arc;
use localcode_core::config::Config;
use localcode_core::llm::provider::*;
use localcode_core::llm::local::LocalProvider;
use localcode_core::llm::openai::OpenAIProvider;
use localcode_core::llm::anthropic::AnthropicProvider;

use crate::streaming;
use crate::rendering;

pub async fn run_review(
    provider_name: Option<&str>,
) -> Result<(), Box<dyn std::error::Error>> {
    let config = Config::load().unwrap_or_default();
    let name = provider_name.unwrap_or(&config.default_provider);
    let cwd = std::env::current_dir()?.display().to_string();

    let diff = localcode_core::git::git_diff(&cwd)?;
    if diff.is_empty() {
        rendering::print_info("No changes to review");
        return Ok(());
    }

    let provider: Arc<dyn LLMProvider> = match name {
        "openai" => Arc::new(OpenAIProvider::new(&config.get_openai_key(), &config.get_openai_model())),
        "anthropic" => Arc::new(AnthropicProvider::new(&config.get_anthropic_key(), &config.get_anthropic_model())),
        _ => Arc::new(LocalProvider::new()),
    };

    let messages = vec![ChatMessage {
        role: "user".to_string(),
        content: format!(
            "Review this git diff for bugs, issues, and improvements. \
             Be concise and actionable:\n\n```diff\n{}\n```",
            diff.chars().take(6000).collect::<String>()
        ),
        tool_calls: None,
        tool_call_id: None,
    }];

    let opts = ChatOptions {
        temperature: 0.3,
        max_tokens: 2048,
        stream: true,
        system: Some("You are a senior code reviewer. Be thorough but concise.".to_string()),
        ..Default::default()
    };

    rendering::print_info("Reviewing changes...\n");

    let stream = provider.chat(messages, opts).await?;
    streaming::stream_to_stdout(stream).await?;

    Ok(())
}
