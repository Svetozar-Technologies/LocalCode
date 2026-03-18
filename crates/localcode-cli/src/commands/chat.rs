use std::sync::Arc;
use localcode_core::config::Config;
use localcode_core::llm::provider::*;
use localcode_core::llm::local::LocalProvider;
use localcode_core::llm::openai::OpenAIProvider;
use localcode_core::llm::anthropic::AnthropicProvider;

use crate::streaming;

pub async fn run_chat(
    message: &str,
    provider_name: Option<&str>,
    model: Option<&str>,
) -> Result<(), Box<dyn std::error::Error>> {
    let config = Config::load().unwrap_or_default();
    let name = provider_name.unwrap_or(&config.default_provider);

    let provider: Arc<dyn LLMProvider> = match name {
        "openai" => {
            let key = config.get_openai_key();
            let m = model.unwrap_or(&config.providers.openai.model);
            let m = if m.is_empty() { "gpt-4o" } else { m };
            Arc::new(OpenAIProvider::new(&key, m))
        }
        "anthropic" => {
            let key = config.get_anthropic_key();
            let m = model.unwrap_or(&config.providers.anthropic.model);
            let m = if m.is_empty() { "claude-sonnet-4-20250514" } else { m };
            Arc::new(AnthropicProvider::new(&key, m))
        }
        _ => Arc::new(LocalProvider::new()),
    };

    let messages = vec![ChatMessage {
        role: "user".to_string(),
        content: message.to_string(),
        tool_calls: None,
        tool_call_id: None,
    }];

    let opts = ChatOptions {
        temperature: 0.7,
        max_tokens: 4096,
        stream: true,
        system: Some("You are LocalCode AI, a helpful coding assistant. Be concise and practical.".to_string()),
        ..Default::default()
    };

    let stream = provider.chat(messages, opts).await?;
    streaming::stream_to_stdout(stream).await?;

    Ok(())
}
