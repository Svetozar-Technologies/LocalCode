use localcode_core::agent::{AgentEngine, AgentEvent, ToolRegistry, ToolContext};
use localcode_core::agent::builtin;
use localcode_core::llm::openai::OpenAIProvider;
use localcode_core::llm::anthropic::AnthropicProvider;
use localcode_core::llm::provider::LLMProvider;
use std::sync::Arc;
use tauri::{AppHandle, Emitter};

use super::llm::LLMManager;

#[tauri::command]
pub async fn agent_execute(
    response_id: String,
    task: String,
    project_path: String,
    current_file: String,
    current_file_content: String,
    provider_name: Option<String>,
    app: AppHandle,
    state: tauri::State<'_, LLMManager>,
) -> Result<(), String> {
    let provider: Arc<dyn LLMProvider> = {
        let llm = state.lock().map_err(|e| e.to_string())?;
        let name = provider_name
            .as_deref()
            .unwrap_or(&llm.config.default_provider);

        match name {
            "openai" => {
                let key = llm.config.get_openai_key();
                let model = llm.config.get_openai_model();
                Arc::new(OpenAIProvider::new(&key, &model))
            }
            "anthropic" => {
                let key = llm.config.get_anthropic_key();
                let model = llm.config.get_anthropic_model();
                Arc::new(AnthropicProvider::new(&key, &model))
            }
            _ => llm.local.clone() as Arc<dyn LLMProvider>,
        }
    };

    // Default to home directory if no project is open
    let project_path = if project_path.is_empty() {
        std::env::var("HOME").unwrap_or_else(|_| "/tmp".to_string())
    } else {
        project_path
    };

    let mut registry = ToolRegistry::new();
    builtin::register_all(&mut registry);

    let mut engine = AgentEngine::new(provider, registry);

    // Initialize memory, auto-discovery, and session
    engine.initialize(&project_path);

    // Add current file context to system prompt if available
    if !current_file_content.is_empty() {
        let file_ctx = format!(
            "\n\nCurrent file ({}):\n```\n{}\n```",
            current_file,
            current_file_content.chars().take(4000).collect::<String>()
        );
        engine = engine.with_system_prompt(format!(
            "You are LocalCode Agent, an autonomous AI coding assistant.{}",
            file_ctx
        ));
        // Re-initialize to merge memory context
        engine.initialize(&project_path);
    }

    let ctx = ToolContext {
        project_path,
        current_file: if current_file.is_empty() {
            None
        } else {
            Some(current_file)
        },
    };

    let app_clone = app.clone();
    let rid = response_id.clone();

    let result = engine
        .execute(&task, &ctx, &move |event| match event {
            AgentEvent::Step(step) => {
                let _ = app_clone.emit(
                    "agent-step",
                    serde_json::json!({"id": rid, "step": step}),
                );
            }
            AgentEvent::TextChunk(text) | AgentEvent::Done(text) => {
                let _ = app_clone.emit(
                    "llm-chat-chunk",
                    serde_json::json!({"id": rid, "chunk": text}),
                );
            }
            AgentEvent::Error(e) => {
                let _ = app_clone.emit(
                    "llm-chat-chunk",
                    serde_json::json!({"id": rid, "chunk": format!("\n\nError: {}", e)}),
                );
            }
        })
        .await;

    if let Err(e) = result {
        let _ = app.emit(
            "llm-chat-chunk",
            serde_json::json!({"id": response_id, "chunk": format!("\n\nError: {}", e)}),
        );
    }

    let _ = app.emit("llm-chat-done", serde_json::json!({"id": response_id}));
    Ok(())
}
