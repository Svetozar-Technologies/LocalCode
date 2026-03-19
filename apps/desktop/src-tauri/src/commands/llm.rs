use localcode_core::llm::local::LocalProvider;
use localcode_core::llm::provider::*;
use localcode_core::llm::openai::OpenAIProvider;
use localcode_core::llm::anthropic::AnthropicProvider;
use localcode_core::llm::model_manager::{ModelManager, ModelCatalogEntry, DownloadedModel};
use localcode_core::config::Config;
use futures::StreamExt;
use std::sync::Arc;
use tauri::{AppHandle, Emitter};

pub struct LLMState {
    pub local: Arc<LocalProvider>,
    pub config: Config,
}

pub type LLMManager = std::sync::Arc<std::sync::Mutex<LLMState>>;

pub fn create_llm_manager() -> LLMManager {
    let config = Config::load().unwrap_or_default();
    std::sync::Arc::new(std::sync::Mutex::new(LLMState {
        local: Arc::new(LocalProvider::new()),
        config,
    }))
}

#[tauri::command]
pub async fn start_llm_server(
    model_path: String,
    app: AppHandle,
    state: tauri::State<'_, LLMManager>,
) -> Result<(), String> {
    let local = {
        let llm = state.lock().map_err(|e| e.to_string())?;
        llm.local.clone()
    };

    let model_name = local
        .start_server(&model_path)
        .await
        .map_err(|e| e.to_string())?;

    let _ = app.emit("llm-ready", &model_name);
    Ok(())
}

#[tauri::command]
pub fn stop_llm_server(state: tauri::State<'_, LLMManager>) -> Result<(), String> {
    let local = {
        let llm = state.lock().map_err(|e| e.to_string())?;
        llm.local.clone()
    };
    local.stop_server().map_err(|e| e.to_string())
}

#[tauri::command]
pub async fn llm_chat(
    response_id: String,
    messages: Vec<ChatMessage>,
    context: String,
    provider_name: Option<String>,
    images: Option<Vec<String>>,
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
                if llm.config.providers.openai.base_url.is_empty() {
                    Arc::new(OpenAIProvider::new(&key, &model))
                } else {
                    Arc::new(OpenAIProvider::with_base_url(
                        &key,
                        &llm.config.providers.openai.base_url,
                        &model,
                    ))
                }
            }
            "anthropic" => {
                let key = llm.config.get_anthropic_key();
                let model = llm.config.get_anthropic_model();
                Arc::new(AnthropicProvider::new(&key, &model))
            }
            _ => llm.local.clone(),
        }
    };

    // Build image context if present
    let image_context = if let Some(ref imgs) = images {
        if !imgs.is_empty() {
            format!("\n\n[{} image(s) attached by user - vision content included in message]", imgs.len())
        } else {
            String::new()
        }
    } else {
        String::new()
    };

    let opts = ChatOptions {
        temperature: 0.7,
        max_tokens: 4096,
        stream: true,
        system: Some(format!(
            "You are LocalCode AI, a helpful coding assistant. \
             Be concise and practical.\n\n{}{}",
            context, image_context
        )),
        ..Default::default()
    };

    let mut stream = provider
        .chat(messages, opts)
        .await
        .map_err(|e| e.to_string())?;

    while let Some(chunk) = stream.next().await {
        match chunk {
            Ok(ChatChunk::Text(text)) => {
                let _ = app.emit(
                    "llm-chat-chunk",
                    serde_json::json!({"id": response_id, "chunk": text}),
                );
            }
            Ok(ChatChunk::Done) => break,
            Ok(ChatChunk::Error(e)) => {
                let _ = app.emit(
                    "llm-chat-chunk",
                    serde_json::json!({"id": response_id, "chunk": format!("\n\nError: {}", e)}),
                );
                break;
            }
            Err(e) => {
                let _ = app.emit(
                    "llm-chat-chunk",
                    serde_json::json!({"id": response_id, "chunk": format!("\n\nError: {}", e)}),
                );
                break;
            }
            _ => {}
        }
    }

    let _ = app.emit("llm-chat-done", serde_json::json!({"id": response_id}));
    Ok(())
}

#[tauri::command]
pub async fn llm_complete(
    prompt: String,
    suffix: String,
    provider_name: Option<String>,
    multiline: Option<bool>,
    stop: Option<Vec<String>>,
    state: tauri::State<'_, LLMManager>,
) -> Result<String, String> {
    let provider: Arc<dyn LLMProvider> = {
        let llm = state.lock().map_err(|e| e.to_string())?;
        let name = provider_name
            .as_deref()
            .unwrap_or(&llm.config.default_provider);

        match name {
            "openai" => {
                let key = llm.config.get_openai_key();
                let model = llm.config.get_openai_model();
                if llm.config.providers.openai.base_url.is_empty() {
                    Arc::new(OpenAIProvider::new(&key, &model))
                } else {
                    Arc::new(OpenAIProvider::with_base_url(
                        &key,
                        &llm.config.providers.openai.base_url,
                        &model,
                    ))
                }
            }
            "anthropic" => {
                let key = llm.config.get_anthropic_key();
                let model = llm.config.get_anthropic_model();
                Arc::new(AnthropicProvider::new(&key, &model))
            }
            _ => llm.local.clone(),
        }
    };

    let is_multiline = multiline.unwrap_or(false);
    let opts = if is_multiline {
        let mut opts = CompletionOptions::multiline_default();
        if let Some(s) = stop {
            opts.stop = s;
        }
        opts
    } else {
        let mut opts = CompletionOptions::default();
        if let Some(s) = stop {
            opts.stop = s;
        }
        opts
    };

    provider
        .complete(&prompt, &suffix, opts)
        .await
        .map_err(|e| e.to_string())
}

#[tauri::command]
pub async fn llm_complete_stream(
    response_id: String,
    prompt: String,
    suffix: String,
    provider_name: Option<String>,
    multiline: Option<bool>,
    stop: Option<Vec<String>>,
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
                if llm.config.providers.openai.base_url.is_empty() {
                    Arc::new(OpenAIProvider::new(&key, &model))
                } else {
                    Arc::new(OpenAIProvider::with_base_url(
                        &key,
                        &llm.config.providers.openai.base_url,
                        &model,
                    ))
                }
            }
            "anthropic" => {
                let key = llm.config.get_anthropic_key();
                let model = llm.config.get_anthropic_model();
                Arc::new(AnthropicProvider::new(&key, &model))
            }
            _ => llm.local.clone(),
        }
    };

    let is_multiline = multiline.unwrap_or(false);
    let opts = if is_multiline {
        let mut opts = CompletionOptions::multiline_default();
        if let Some(s) = stop {
            opts.stop = s;
        }
        opts
    } else {
        let mut opts = CompletionOptions::default();
        if let Some(s) = stop {
            opts.stop = s;
        }
        opts
    };

    let mut stream = provider
        .complete_stream(&prompt, &suffix, opts)
        .await
        .map_err(|e| e.to_string())?;

    while let Some(chunk) = stream.next().await {
        match chunk {
            Ok(ChatChunk::Text(text)) => {
                let _ = app.emit(
                    "llm-completion-chunk",
                    serde_json::json!({"id": response_id, "chunk": text}),
                );
            }
            Ok(ChatChunk::Done) => break,
            Ok(ChatChunk::Error(e)) => {
                let _ = app.emit(
                    "llm-completion-chunk",
                    serde_json::json!({"id": response_id, "chunk": "", "error": e}),
                );
                break;
            }
            Err(e) => {
                let _ = app.emit(
                    "llm-completion-chunk",
                    serde_json::json!({"id": response_id, "chunk": "", "error": e.to_string()}),
                );
                break;
            }
            _ => {}
        }
    }

    let _ = app.emit("llm-completion-done", serde_json::json!({"id": response_id}));
    Ok(())
}

#[tauri::command]
pub fn save_config(
    config_json: String,
    state: tauri::State<'_, LLMManager>,
) -> Result<(), String> {
    let config: Config =
        serde_json::from_str(&config_json).map_err(|e| e.to_string())?;
    config.save().map_err(|e| e.to_string())?;
    let mut llm = state.lock().map_err(|e| e.to_string())?;
    llm.config = config;
    Ok(())
}

#[tauri::command]
pub fn load_config(state: tauri::State<'_, LLMManager>) -> Result<String, String> {
    let llm = state.lock().map_err(|e| e.to_string())?;
    serde_json::to_string(&llm.config).map_err(|e| e.to_string())
}

#[derive(serde::Serialize)]
pub struct CatalogEntryWithStatus {
    #[serde(flatten)]
    pub entry: ModelCatalogEntry,
    pub downloaded: bool,
}

#[tauri::command]
pub fn list_model_catalog() -> Result<Vec<CatalogEntryWithStatus>, String> {
    let manager = ModelManager::new();
    let catalog = ModelManager::catalog();
    let result = catalog
        .into_iter()
        .map(|entry| {
            let downloaded = manager.is_downloaded(&entry.id);
            CatalogEntryWithStatus { entry, downloaded }
        })
        .collect();
    Ok(result)
}

#[tauri::command]
pub async fn download_model(
    catalog_id: String,
    app: AppHandle,
) -> Result<String, String> {
    let mut manager = ModelManager::new();
    let cid = catalog_id.clone();
    let app_clone = app.clone();

    let path = manager
        .download(&catalog_id, move |progress| {
            let _ = app_clone.emit("model-download-progress", serde_json::json!({
                "catalog_id": progress.catalog_id,
                "downloaded_bytes": progress.downloaded_bytes,
                "total_bytes": progress.total_bytes,
                "speed_bps": progress.speed_bps,
                "eta_seconds": progress.eta_seconds,
            }));
        })
        .await
        .map_err(|e| e.to_string())?;

    let _ = app.emit("model-download-complete", serde_json::json!({
        "catalog_id": cid,
        "path": path,
    }));

    Ok(path)
}

#[tauri::command]
pub fn list_downloaded_models() -> Result<Vec<DownloadedModel>, String> {
    let manager = ModelManager::new();
    Ok(manager.list_downloaded())
}

#[tauri::command]
pub fn delete_model(catalog_id: String) -> Result<(), String> {
    let mut manager = ModelManager::new();
    manager.delete_model(&catalog_id).map_err(|e| e.to_string())
}
