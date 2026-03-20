use localcode_core::agent::{AgentEngine, AgentEvent, ToolRegistry, ToolContext};
use localcode_core::agent::builtin;
use localcode_core::llm::openai::OpenAIProvider;
use localcode_core::llm::anthropic::AnthropicProvider;
use localcode_core::llm::provider::LLMProvider;
use std::sync::Arc;
use serde::Deserialize;
use tauri::{AppHandle, Emitter};

use super::llm::LLMManager;

#[derive(Debug, Deserialize)]
pub struct ChatHistoryEntry {
    pub role: String,
    pub content: String,
}

#[tauri::command]
#[allow(clippy::too_many_arguments)]
pub async fn agent_execute(
    response_id: String,
    task: String,
    project_path: String,
    current_file: String,
    current_file_content: String,
    chat_history: Option<Vec<ChatHistoryEntry>>,
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
            "ollama" | "local" => {
                let server_url = &llm.config.providers.local.server_url;
                let base_url = format!("{}/v1", server_url);
                let model = llm.config.providers.local.active_catalog_model
                    .clone()
                    .unwrap_or_else(|| "qwen2.5-coder:14b".to_string());
                Arc::new(OpenAIProvider::with_base_url("ollama", &base_url, &model))
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

    let is_local = provider_name.as_deref().unwrap_or("local") == "local"
        || provider_name.is_none();

    let mut registry = ToolRegistry::new();
    if is_local {
        // Local models: register only essential tools to save context tokens
        builtin::register_essential(&mut registry);
    } else {
        builtin::register_all(&mut registry);
    }

    let mut engine = AgentEngine::new(provider.clone(), registry);

    if is_local {
        // For local models: skip heavy memory/discovery to save context tokens
        // Just set a minimal system prompt
        let mut prompt = String::from("You are LocalCode Agent.");
        if !current_file.is_empty() && !current_file_content.is_empty() {
            let truncated: String = current_file_content.chars().take(1000).collect();
            prompt.push_str(&format!("\nCurrent file: {}\n```\n{}\n```", current_file, truncated));
        }
        engine = engine.with_system_prompt(prompt);
    } else {
        // For cloud models: use full memory and project discovery
        engine.initialize(&project_path);
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
            engine.initialize(&project_path);
        }
    }

    // Build task — keep it lean for local models
    let full_task = if is_local {
        // Local: just the task, no history (save tokens)
        task
    } else if let Some(ref history) = chat_history {
        if history.is_empty() {
            task
        } else {
            let mut context = String::from("## Conversation History\n");
            let start = if history.len() > 10 { history.len() - 10 } else { 0 };
            for entry in &history[start..] {
                let role_label = if entry.role == "user" { "User" } else { "Assistant" };
                context.push_str(&format!("**{}**: {}\n\n", role_label, entry.content));
            }
            context.push_str(&format!("## Current Task\n{}", task));
            context
        }
    } else {
        task
    };

    let ctx = ToolContext {
        project_path,
        current_file: if current_file.is_empty() {
            None
        } else {
            Some(current_file)
        },
        provider: Some(provider.clone()),
    };

    let app_clone = app.clone();
    let rid = response_id.clone();

    let event_handler = move |event: AgentEvent| match event {
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
    };

    // Local models: use plan-and-execute (breaks task into small steps for 8K context)
    // Cloud models: auto-select based on provider capabilities
    let result = if is_local {
        engine.execute_planned(&full_task, &ctx, &event_handler).await
    } else {
        engine.execute(&full_task, &ctx, &event_handler).await
    };

    if let Err(e) = result {
        let _ = app.emit(
            "llm-chat-chunk",
            serde_json::json!({"id": response_id, "chunk": format!("\n\nError: {}", e)}),
        );
    }

    let _ = app.emit("llm-chat-done", serde_json::json!({"id": response_id}));
    Ok(())
}

#[tauri::command]
pub async fn composer_generate(
    task: String,
    project_path: String,
    app: AppHandle,
    state: tauri::State<'_, LLMManager>,
) -> Result<(), String> {
    let provider: Arc<dyn LLMProvider> = {
        let llm = state.lock().map_err(|e| e.to_string())?;
        let name = &llm.config.default_provider;

        match name.as_str() {
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
            "ollama" | "local" => {
                let server_url = &llm.config.providers.local.server_url;
                let base_url = format!("{}/v1", server_url);
                let model = llm.config.providers.local.active_catalog_model
                    .clone()
                    .unwrap_or_else(|| "qwen2.5-coder:14b".to_string());
                Arc::new(OpenAIProvider::with_base_url("ollama", &base_url, &model))
            }
            _ => llm.local.clone() as Arc<dyn LLMProvider>,
        }
    };

    let project_path = if project_path.is_empty() {
        std::env::var("HOME").unwrap_or_else(|_| "/tmp".to_string())
    } else {
        project_path
    };

    let mut registry = ToolRegistry::new();
    builtin::register_all(&mut registry);

    let mut engine = AgentEngine::new(provider.clone(), registry);
    engine.initialize(&project_path);

    // System prompt for composer mode
    engine = engine.with_system_prompt(
        "You are LocalCode Composer, a multi-file code generation assistant. \
         When given a task, identify the files that need to be created or modified \
         and make the changes using your tools (write_file, edit_file, create_file). \
         Work through each file methodically. After each file change, the UI will \
         show the diff to the user for review.".to_string()
    );
    engine.initialize(&project_path);

    let ctx = ToolContext {
        project_path,
        current_file: None,
        provider: Some(provider),
    };

    let app_clone = app.clone();

    let result = engine
        .execute(&task, &ctx, &move |event| match event {
            AgentEvent::Step(step) => {
                // Emit file changes for the composer UI
                let step_json = serde_json::to_value(&step).unwrap_or_default();
                let tool = step_json.get("tool").and_then(|v| v.as_str()).unwrap_or("");
                let result_str = step_json.get("result").and_then(|v| v.as_str()).unwrap_or("");

                if (tool == "write_file" || tool == "edit_file" || tool == "create_file")
                    && !result_str.starts_with("Error")
                {
                    let _ = app_clone.emit("composer-file-change", &step_json);
                }

                let _ = app_clone.emit("composer-step", &step_json);
            }
            AgentEvent::TextChunk(text) => {
                let _ = app_clone.emit(
                    "composer-text",
                    serde_json::json!({"chunk": text}),
                );
            }
            AgentEvent::Done(_text) => {
                let _ = app_clone.emit("composer-done", serde_json::json!({}));
            }
            AgentEvent::Error(e) => {
                let _ = app_clone.emit(
                    "composer-error",
                    serde_json::json!({"error": e}),
                );
            }
        })
        .await;

    if let Err(e) = result {
        let _ = app.emit(
            "composer-error",
            serde_json::json!({"error": e.to_string()}),
        );
    }

    let _ = app.emit("composer-done", serde_json::json!({}));
    Ok(())
}
