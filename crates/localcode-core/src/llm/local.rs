use async_trait::async_trait;
use futures::StreamExt;
use reqwest::Client;

use std::process::{Child, Command};
use std::sync::{Arc, Mutex};
use tokio_stream::wrappers::ReceiverStream;

use super::provider::*;
use crate::CoreError;

pub struct LocalProvider {
    server_url: String,
    client: Client,
    server_process: Arc<Mutex<Option<Child>>>,
    model_name: Arc<Mutex<String>>,
    model_path: Arc<Mutex<String>>,
}

impl Default for LocalProvider {
    fn default() -> Self {
        Self::new()
    }
}

impl LocalProvider {
    pub fn new() -> Self {
        Self {
            server_url: "http://127.0.0.1:8081".to_string(),
            client: Client::new(),
            server_process: Arc::new(Mutex::new(None)),
            model_name: Arc::new(Mutex::new(String::new())),
            model_path: Arc::new(Mutex::new(String::new())),
        }
    }

    pub fn with_url(url: &str) -> Self {
        Self {
            server_url: url.to_string(),
            client: Client::new(),
            server_process: Arc::new(Mutex::new(None)),
            model_name: Arc::new(Mutex::new(String::new())),
            model_path: Arc::new(Mutex::new(String::new())),
        }
    }

    pub async fn start_server(&self, model_path: &str) -> Result<String, CoreError> {
        // Kill existing managed server process
        {
            let mut proc = self.server_process.lock().map_err(|e| CoreError::Other(e.to_string()))?;
            if let Some(ref mut child) = *proc {
                let _ = child.kill();
                let _ = child.wait(); // Reap the zombie process
            }
            *proc = None;
        }

        // Kill any orphaned llama-server processes to avoid GPU memory conflicts
        let _ = Command::new("pkill").arg("-f").arg("llama-server").output();
        // Wait for processes to fully terminate and release GPU memory
        tokio::time::sleep(tokio::time::Duration::from_millis(1000)).await;

        let model_name = std::path::Path::new(model_path)
            .file_stem()
            .and_then(|s| s.to_str())
            .unwrap_or("unknown")
            .to_string();

        let server_binary = which_llama_server();

        // Determine safe GPU layers based on available system memory
        let (ctx_size, gpu_layers) = get_safe_params(model_path);

        let child = Command::new(&server_binary)
            .arg("-m")
            .arg(model_path)
            .arg("--host")
            .arg("127.0.0.1")
            .arg("--port")
            .arg("8081")
            .arg("-c")
            .arg(ctx_size.to_string())
            .arg("-ngl")
            .arg(gpu_layers.to_string())
            .arg("--chat-template")
            .arg("chatml")
            .stdout(std::process::Stdio::null())
            .stderr(std::process::Stdio::null())
            .spawn()
            .map_err(|e| {
                let install_hint = if cfg!(target_os = "linux") {
                    "Install with: sudo apt install llama.cpp  OR  snap install llama-cpp"
                } else {
                    "Install with: brew install llama.cpp"
                };
                CoreError::Llm(format!(
                    "Failed to start llama-server ({}): {}. {}",
                    server_binary, e, install_hint
                ))
            })?;

        {
            let mut proc = self.server_process.lock().map_err(|e| CoreError::Other(e.to_string()))?;
            *proc = Some(child);
            let mut name = self.model_name.lock().map_err(|e| CoreError::Other(e.to_string()))?;
            *name = model_name.clone();
            let mut path = self.model_path.lock().map_err(|e| CoreError::Other(e.to_string()))?;
            *path = model_path.to_string();
        }

        // Poll until server is ready — check if process is still alive too
        for _ in 0..60 {
            tokio::time::sleep(tokio::time::Duration::from_millis(500)).await;

            // Check if server process crashed
            {
                let mut proc = self.server_process.lock().map_err(|e| CoreError::Other(e.to_string()))?;
                if let Some(ref mut child) = *proc {
                    match child.try_wait() {
                        Ok(Some(status)) => {
                            *proc = None;
                            return Err(CoreError::Llm(format!(
                                "llama-server exited prematurely with status: {}. The model may be too large for available memory.",
                                status
                            )));
                        }
                        Ok(None) => {} // Still running, good
                        Err(e) => {
                            return Err(CoreError::Llm(format!("Failed to check server status: {}", e)));
                        }
                    }
                }
            }

            if let Ok(resp) = self
                .client
                .get(format!("{}/health", self.server_url))
                .send()
                .await
            {
                if resp.status().is_success() {
                    return Ok(model_name);
                }
            }
        }

        // Timed out — kill the server process
        {
            let mut proc = self.server_process.lock().map_err(|e| CoreError::Other(e.to_string()))?;
            if let Some(ref mut child) = *proc {
                let _ = child.kill();
                let _ = child.wait();
            }
            *proc = None;
        }

        Err(CoreError::Llm(
            "Server failed to start within 30 seconds".to_string(),
        ))
    }

    pub fn stop_server(&self) -> Result<(), CoreError> {
        let mut proc = self.server_process.lock().map_err(|e| CoreError::Other(e.to_string()))?;
        if let Some(ref mut child) = *proc {
            child.kill().map_err(|e| CoreError::Other(e.to_string()))?;
        }
        *proc = None;
        let mut name = self.model_name.lock().map_err(|e| CoreError::Other(e.to_string()))?;
        name.clear();
        let mut path = self.model_path.lock().map_err(|e| CoreError::Other(e.to_string()))?;
        path.clear();
        Ok(())
    }

    /// Start server using a model from the catalog
    pub async fn start_with_catalog_model(&self, catalog_id: &str) -> Result<String, CoreError> {
        let manager = super::model_manager::ModelManager::new();
        let path = manager
            .get_model_path(catalog_id)
            .ok_or_else(|| CoreError::Llm(format!("Model '{}' not downloaded", catalog_id)))?;
        self.start_server(&path).await
    }

    /// Check if the llama-server is healthy by hitting its /health endpoint.
    pub async fn health_check(&self) -> bool {
        match self
            .client
            .get(format!("{}/health", self.server_url))
            .timeout(tokio::time::Duration::from_secs(3))
            .send()
            .await
        {
            Ok(resp) => resp.status().is_success(),
            Err(_) => false,
        }
    }

    /// Ensure the server is alive before making a request. If it's dead and we have a
    /// model loaded, attempt to restart it automatically.
    async fn ensure_server_alive(&self) -> Result<(), CoreError> {
        if !self.is_running() {
            return Ok(()); // No managed server, nothing to check
        }

        if self.health_check().await {
            return Ok(());
        }

        // Server is unresponsive — check if the process died
        {
            let mut proc = self.server_process.lock().map_err(|e| CoreError::Other(e.to_string()))?;
            if let Some(ref mut child) = *proc {
                match child.try_wait() {
                    Ok(Some(_)) => {
                        *proc = None;
                    }
                    Ok(None) => {
                        // Process is alive but unresponsive — kill and restart
                        let _ = child.kill();
                        let _ = child.wait();
                        *proc = None;
                    }
                    Err(_) => {
                        *proc = None;
                    }
                }
            }
        }

        // Try to auto-restart using the stored model path
        let stored_path = self.model_path.lock()
            .map_err(|e| CoreError::Other(e.to_string()))?
            .clone();

        if stored_path.is_empty() {
            return Err(CoreError::Llm("LLM server is not responding and no model path is stored for auto-restart".to_string()));
        }

        if std::path::Path::new(&stored_path).exists() {
            log::info!("Auto-restarting llama-server with model: {}", stored_path);
            self.start_server(&stored_path).await?;
            Ok(())
        } else {
            Err(CoreError::Llm(format!(
                "LLM server crashed. Model file '{}' not found. Please restart manually from Settings.",
                stored_path
            )))
        }
    }

    pub fn is_running(&self) -> bool {
        self.server_process
            .lock()
            .map(|p| p.is_some())
            .unwrap_or(false)
    }

    pub fn model_name(&self) -> String {
        self.model_name
            .lock()
            .map(|n| n.clone())
            .unwrap_or_default()
    }
}

impl Drop for LocalProvider {
    fn drop(&mut self) {
        let _ = self.stop_server();
    }
}

#[async_trait]
impl LLMProvider for LocalProvider {
    fn name(&self) -> &str {
        "local"
    }

    async fn chat(
        &self,
        messages: Vec<ChatMessage>,
        opts: ChatOptions,
    ) -> Result<ChatStream, CoreError> {
        self.ensure_server_alive().await?;

        // When tools are present, use non-streaming to properly parse tool calls
        if !opts.tools.is_empty() {
            let response = self.chat_sync(messages, opts).await?;
            let (tx, rx) = tokio::sync::mpsc::channel(10);
            tokio::spawn(async move {
                if let Some(ref tool_calls) = response.tool_calls {
                    if !tool_calls.is_empty() {
                        // Send the full message as a single chunk so the engine can extract tool calls
                        let _ = tx.send(Ok(ChatChunk::Text(response.content.clone()))).await;
                        for tc in tool_calls {
                            let _ = tx.send(Ok(ChatChunk::ToolCallStart {
                                id: tc.id.clone(),
                                name: tc.function.name.clone(),
                            })).await;
                            let _ = tx.send(Ok(ChatChunk::ToolCallDelta {
                                id: tc.id.clone(),
                                arguments_delta: tc.function.arguments.clone(),
                            })).await;
                            let _ = tx.send(Ok(ChatChunk::ToolCallEnd {
                                id: tc.id.clone(),
                            })).await;
                        }
                        let _ = tx.send(Ok(ChatChunk::Done)).await;
                        return;
                    }
                }
                let _ = tx.send(Ok(ChatChunk::Text(response.content))).await;
                let _ = tx.send(Ok(ChatChunk::Done)).await;
            });
            return Ok(Box::pin(ReceiverStream::new(rx)));
        }

        let mut api_messages: Vec<serde_json::Value> = Vec::new();

        if let Some(ref system) = opts.system {
            api_messages.push(serde_json::json!({
                "role": "system",
                "content": system
            }));
        }

        for msg in &messages {
            api_messages.push(serde_json::json!({
                "role": msg.role,
                "content": msg.content
            }));
        }

        let body = serde_json::json!({
            "messages": api_messages,
            "stream": true,
            "temperature": opts.temperature,
            "max_tokens": opts.max_tokens,
        });

        let response = self
            .client
            .post(format!("{}/v1/chat/completions", self.server_url))
            .json(&body)
            .send()
            .await?;

        let (tx, rx) = tokio::sync::mpsc::channel(100);
        let mut stream = response.bytes_stream();

        tokio::spawn(async move {
            let mut buffer = String::new();

            while let Some(chunk) = stream.next().await {
                match chunk {
                    Ok(bytes) => {
                        let text = String::from_utf8_lossy(&bytes);
                        buffer.push_str(&text);

                        while let Some(pos) = buffer.find('\n') {
                            let line = buffer[..pos].to_string();
                            buffer = buffer[pos + 1..].to_string();

                            let line = line.trim();
                            if let Some(data) = line.strip_prefix("data: ") {
                                if data == "[DONE]" {
                                    let _ = tx.send(Ok(ChatChunk::Done)).await;
                                    return;
                                }

                                if let Ok(parsed) =
                                    serde_json::from_str::<serde_json::Value>(data)
                                {
                                    if let Some(content) =
                                        parsed["choices"][0]["delta"]["content"].as_str()
                                    {
                                        let _ = tx
                                            .send(Ok(ChatChunk::Text(content.to_string())))
                                            .await;
                                    }
                                }
                            }
                        }
                    }
                    Err(e) => {
                        let _ = tx
                            .send(Err(CoreError::Http(e)))
                            .await;
                        return;
                    }
                }
            }

            let _ = tx.send(Ok(ChatChunk::Done)).await;
        });

        Ok(Box::pin(ReceiverStream::new(rx)))
    }

    async fn chat_sync(
        &self,
        messages: Vec<ChatMessage>,
        opts: ChatOptions,
    ) -> Result<ChatMessage, CoreError> {
        let mut api_messages: Vec<serde_json::Value> = Vec::new();

        if let Some(ref system) = opts.system {
            api_messages.push(serde_json::json!({
                "role": "system",
                "content": system
            }));
        }

        for msg in &messages {
            let mut m = serde_json::json!({
                "role": msg.role,
                "content": msg.content
            });
            if let Some(ref tool_calls) = msg.tool_calls {
                m["tool_calls"] = serde_json::to_value(tool_calls).unwrap_or_default();
            }
            if let Some(ref tool_call_id) = msg.tool_call_id {
                m["tool_call_id"] = serde_json::json!(tool_call_id);
            }
            api_messages.push(m);
        }

        let mut body = serde_json::json!({
            "messages": api_messages,
            "stream": false,
            "temperature": opts.temperature,
            "max_tokens": opts.max_tokens,
        });

        if !opts.tools.is_empty() {
            body["tools"] = serde_json::to_value(&opts.tools)?;
        }

        let response = self
            .client
            .post(format!("{}/v1/chat/completions", self.server_url))
            .json(&body)
            .send()
            .await?;

        let result: serde_json::Value = response.json().await?;
        let message = &result["choices"][0]["message"];

        let content = message["content"]
            .as_str()
            .unwrap_or("")
            .to_string();

        let tool_calls = message.get("tool_calls").and_then(|tc| {
            serde_json::from_value::<Vec<ToolCall>>(tc.clone()).ok()
        });

        Ok(ChatMessage {
            role: "assistant".to_string(),
            content,
            tool_calls,
            tool_call_id: None,
        })
    }

    async fn complete(
        &self,
        prompt: &str,
        suffix: &str,
        opts: CompletionOptions,
    ) -> Result<String, CoreError> {
        self.ensure_server_alive().await?;

        let body = serde_json::json!({
            "prompt": prompt,
            "suffix": suffix,
            "n_predict": opts.max_tokens,
            "temperature": opts.temperature,
            "stop": opts.stop,
            "stream": false,
        });

        let response = tokio::time::timeout(
            tokio::time::Duration::from_secs(5),
            self.client
                .post(format!("{}/completion", self.server_url))
                .json(&body)
                .send(),
        )
        .await
        .map_err(|_| CoreError::Llm("Completion request timed out (5s)".to_string()))??;

        let result: serde_json::Value = response.json().await?;
        Ok(result["content"].as_str().unwrap_or("").to_string())
    }

    async fn complete_stream(
        &self,
        prompt: &str,
        suffix: &str,
        opts: CompletionOptions,
    ) -> Result<ChatStream, CoreError> {
        self.ensure_server_alive().await?;

        let body = serde_json::json!({
            "prompt": prompt,
            "suffix": suffix,
            "n_predict": opts.max_tokens,
            "temperature": opts.temperature,
            "stop": opts.stop,
            "stream": true,
        });

        let response = tokio::time::timeout(
            tokio::time::Duration::from_secs(5),
            self.client
                .post(format!("{}/completion", self.server_url))
                .json(&body)
                .send(),
        )
        .await
        .map_err(|_| CoreError::Llm("Completion stream request timed out (5s)".to_string()))??;

        let (tx, rx) = tokio::sync::mpsc::channel(100);
        let mut stream = response.bytes_stream();

        tokio::spawn(async move {
            let mut buffer = String::new();

            while let Some(chunk) = stream.next().await {
                match chunk {
                    Ok(bytes) => {
                        let text = String::from_utf8_lossy(&bytes);
                        buffer.push_str(&text);

                        while let Some(pos) = buffer.find('\n') {
                            let line = buffer[..pos].to_string();
                            buffer = buffer[pos + 1..].to_string();

                            let line = line.trim();
                            let data = match line.strip_prefix("data: ") {
                                Some(d) => d,
                                None => continue,
                            };

                            if let Ok(parsed) =
                                serde_json::from_str::<serde_json::Value>(data)
                            {
                                let stop = parsed["stop"]
                                    .as_bool()
                                    .unwrap_or(false);
                                if stop {
                                    let _ = tx.send(Ok(ChatChunk::Done)).await;
                                    return;
                                }
                                if let Some(content) = parsed["content"].as_str() {
                                    if !content.is_empty() {
                                        let _ = tx
                                            .send(Ok(ChatChunk::Text(
                                                content.to_string(),
                                            )))
                                            .await;
                                    }
                                }
                            }
                        }
                    }
                    Err(e) => {
                        let _ = tx.send(Err(CoreError::Http(e))).await;
                        return;
                    }
                }
            }

            let _ = tx.send(Ok(ChatChunk::Done)).await;
        });

        Ok(Box::pin(ReceiverStream::new(rx)))
    }

    async fn embed(&self, _texts: &[String]) -> Result<Vec<Vec<f32>>, CoreError> {
        Err(CoreError::Llm(
            "Local provider does not support embeddings yet".to_string(),
        ))
    }

    fn capabilities(&self) -> ProviderCapabilities {
        ProviderCapabilities {
            chat: true,
            completion: true,
            embeddings: false,
            tool_calling: false,
            streaming: true,
            vision: false,
        }
    }
}

/// Determine safe context size and GPU layer count based on model size and available memory.
/// This prevents OOM kernel panics by being conservative with GPU memory.
fn get_safe_params(model_path: &str) -> (u32, u32) {
    // Get model file size in GB as a rough proxy for memory requirements
    let model_size_gb = std::fs::metadata(model_path)
        .map(|m| m.len() as f64 / (1024.0 * 1024.0 * 1024.0))
        .unwrap_or(4.0);

    // Get total system memory in GB (platform-specific)
    let total_mem_gb = get_total_memory_gb();

    // Reserve ~4GB for macOS + app overhead, use the rest for the model
    let available_for_model = (total_mem_gb - 4.0).max(2.0);

    // Context size: larger models need less context to stay within memory
    // KV cache memory ≈ ctx_size * n_layers * d_model * 2 * 2 bytes (K+V, fp16)
    let ctx_size: u32 = if available_for_model > model_size_gb * 2.0 {
        8192 // Plenty of headroom
    } else if available_for_model > model_size_gb * 1.5 {
        4096 // Moderate headroom
    } else {
        2048 // Tight — keep context small
    };

    // GPU layers: offload as many as safely fit
    // If model fits comfortably, offload all; otherwise be conservative
    let gpu_layers: u32 = if available_for_model > model_size_gb * 1.8 {
        99 // Full GPU offload
    } else if available_for_model > model_size_gb * 1.2 {
        40 // Partial offload
    } else {
        0 // CPU only — not enough memory for GPU offload
    };

    (ctx_size, gpu_layers)
}

/// Get total system memory in GB using platform-specific methods.
fn get_total_memory_gb() -> f64 {
    #[cfg(target_os = "macos")]
    {
        Command::new("sysctl")
            .arg("-n")
            .arg("hw.memsize")
            .output()
            .ok()
            .and_then(|o| String::from_utf8_lossy(&o.stdout).trim().parse::<u64>().ok())
            .map(|bytes| bytes as f64 / (1024.0 * 1024.0 * 1024.0))
            .unwrap_or(8.0)
    }
    #[cfg(target_os = "linux")]
    {
        std::fs::read_to_string("/proc/meminfo")
            .ok()
            .and_then(|s| {
                s.lines()
                    .find(|l| l.starts_with("MemTotal:"))
                    .and_then(|l| l.split_whitespace().nth(1)?.parse::<u64>().ok())
                    .map(|kb| kb as f64 / (1024.0 * 1024.0))
            })
            .unwrap_or(8.0)
    }
    #[cfg(not(any(target_os = "macos", target_os = "linux")))]
    {
        8.0
    }
}

fn which_llama_server() -> String {
    let candidates: Vec<&str> = if cfg!(target_os = "linux") {
        vec![
            "llama-server",
            "/usr/local/bin/llama-server",
            "/usr/bin/llama-server",
            "/snap/bin/llama-server",
        ]
    } else {
        vec![
            "llama-server",
            "/usr/local/bin/llama-server",
            "/opt/homebrew/bin/llama-server",
        ]
    };

    for candidate in &candidates {
        if Command::new(candidate).arg("--version").output().is_ok() {
            return candidate.to_string();
        }
    }

    "llama-server".to_string()
}
