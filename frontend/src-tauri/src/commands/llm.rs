use reqwest::Client;
use serde::{Deserialize, Serialize};
use std::process::{Child, Command};
use std::sync::{Arc, Mutex};
use tauri::{AppHandle, Emitter};

#[derive(Debug, Serialize, Deserialize)]
pub struct ChatMessage {
    pub role: String,
    pub content: String,
}

pub struct LLMState {
    pub server_process: Option<Child>,
    pub server_url: String,
    pub model_name: String,
    pub client: Client,
}

impl LLMState {
    pub fn new() -> Self {
        Self {
            server_process: None,
            server_url: "http://127.0.0.1:8081".to_string(),
            model_name: String::new(),
            client: Client::new(),
        }
    }
}

impl Drop for LLMState {
    fn drop(&mut self) {
        if let Some(ref mut child) = self.server_process {
            let _ = child.kill();
        }
    }
}

pub type LLMManager = Arc<Mutex<LLMState>>;

pub fn create_llm_manager() -> LLMManager {
    Arc::new(Mutex::new(LLMState::new()))
}

#[tauri::command]
pub async fn start_llm_server(
    model_path: String,
    app: AppHandle,
    state: tauri::State<'_, LLMManager>,
) -> Result<(), String> {
    // Extract info from locked state, then drop lock before any await
    let (url, client, model_name) = {
        let mut llm = state.lock().map_err(|e| e.to_string())?;

        // Kill existing server if running
        if let Some(ref mut child) = llm.server_process {
            let _ = child.kill();
            llm.server_process = None;
        }

        let model_name = std::path::Path::new(&model_path)
            .file_stem()
            .and_then(|s| s.to_str())
            .unwrap_or("unknown")
            .to_string();

        let server_binary = which_llama_server();

        let child = Command::new(&server_binary)
            .arg("-m")
            .arg(&model_path)
            .arg("--host")
            .arg("127.0.0.1")
            .arg("--port")
            .arg("8081")
            .arg("-c")
            .arg("4096")
            .arg("-ngl")
            .arg("99") // GPU layers - Metal on M3
            .arg("--chat-template")
            .arg("chatml")
            .spawn()
            .map_err(|e| format!(
                "Failed to start llama-server ({}): {}. Install with: brew install llama.cpp",
                server_binary, e
            ))?;

        llm.server_process = Some(child);
        llm.model_name = model_name.clone();

        let url = llm.server_url.clone();
        let client = llm.client.clone();
        (url, client, model_name)
    }; // MutexGuard dropped here

    // Poll until server is ready (max 30 seconds)
    for _ in 0..60 {
        tokio::time::sleep(tokio::time::Duration::from_millis(500)).await;
        if let Ok(resp) = client.get(format!("{}/health", url)).send().await {
            if resp.status().is_success() {
                let _ = app.emit("llm-ready", &model_name);
                return Ok(());
            }
        }
    }

    Err("Server failed to start within 30 seconds".to_string())
}

#[tauri::command]
pub fn stop_llm_server(
    state: tauri::State<'_, LLMManager>,
) -> Result<(), String> {
    let mut llm = state.lock().map_err(|e| e.to_string())?;
    if let Some(ref mut child) = llm.server_process {
        child.kill().map_err(|e| e.to_string())?;
    }
    llm.server_process = None;
    llm.model_name.clear();
    Ok(())
}

#[tauri::command]
pub async fn llm_chat(
    response_id: String,
    messages: Vec<ChatMessage>,
    context: String,
    app: AppHandle,
    state: tauri::State<'_, LLMManager>,
) -> Result<(), String> {
    let (url, client) = {
        let llm = state.lock().map_err(|e| e.to_string())?;
        (llm.server_url.clone(), llm.client.clone())
    }; // Lock dropped

    let mut full_messages = Vec::new();
    full_messages.push(serde_json::json!({
        "role": "system",
        "content": format!(
            "You are LocalCode AI, a helpful coding assistant running locally. \
             You help with code analysis, writing, debugging, and explanation. \
             Be concise and practical.\n\n{}", context
        )
    }));

    for msg in &messages {
        full_messages.push(serde_json::json!({
            "role": msg.role,
            "content": msg.content
        }));
    }

    let body = serde_json::json!({
        "messages": full_messages,
        "stream": true,
        "temperature": 0.7,
        "max_tokens": 2048,
    });

    let response = client
        .post(format!("{}/v1/chat/completions", url))
        .json(&body)
        .send()
        .await
        .map_err(|e| format!("LLM request failed: {}", e))?;

    // Stream the response
    use futures::StreamExt;
    let mut stream = response.bytes_stream();
    let mut buffer = String::new();

    while let Some(chunk) = stream.next().await {
        let chunk = chunk.map_err(|e| e.to_string())?;
        let text = String::from_utf8_lossy(&chunk);
        buffer.push_str(&text);

        while let Some(pos) = buffer.find('\n') {
            let line = buffer[..pos].to_string();
            buffer = buffer[pos + 1..].to_string();

            let line = line.trim();
            if line.starts_with("data: ") {
                let data = &line[6..];
                if data == "[DONE]" {
                    let _ = app.emit("llm-chat-done", serde_json::json!({ "id": response_id }));
                    return Ok(());
                }

                if let Ok(parsed) = serde_json::from_str::<serde_json::Value>(data) {
                    if let Some(content) = parsed["choices"][0]["delta"]["content"].as_str() {
                        let _ = app.emit(
                            "llm-chat-chunk",
                            serde_json::json!({
                                "id": response_id,
                                "chunk": content
                            }),
                        );
                    }
                }
            }
        }
    }

    let _ = app.emit("llm-chat-done", serde_json::json!({ "id": response_id }));
    Ok(())
}

#[tauri::command]
pub async fn llm_complete(
    prompt: String,
    suffix: String,
    state: tauri::State<'_, LLMManager>,
) -> Result<String, String> {
    let (url, client) = {
        let llm = state.lock().map_err(|e| e.to_string())?;
        (llm.server_url.clone(), llm.client.clone())
    }; // Lock dropped

    let body = serde_json::json!({
        "prompt": prompt,
        "suffix": suffix,
        "n_predict": 128,
        "temperature": 0.2,
        "stop": ["\n\n", "\r\n\r\n"],
        "stream": false,
    });

    let response = client
        .post(format!("{}/completion", url))
        .json(&body)
        .send()
        .await
        .map_err(|e| format!("Completion request failed: {}", e))?;

    let result: serde_json::Value = response.json().await.map_err(|e| e.to_string())?;

    Ok(result["content"]
        .as_str()
        .unwrap_or("")
        .to_string())
}

fn which_llama_server() -> String {
    let candidates = [
        "llama-server",
        "/usr/local/bin/llama-server",
        "/opt/homebrew/bin/llama-server",
    ];

    for candidate in &candidates {
        if Command::new(candidate)
            .arg("--version")
            .output()
            .is_ok()
        {
            return candidate.to_string();
        }
    }

    "llama-server".to_string()
}
