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
}

impl LocalProvider {
    pub fn new() -> Self {
        Self {
            server_url: "http://127.0.0.1:8081".to_string(),
            client: Client::new(),
            server_process: Arc::new(Mutex::new(None)),
            model_name: Arc::new(Mutex::new(String::new())),
        }
    }

    pub fn with_url(url: &str) -> Self {
        Self {
            server_url: url.to_string(),
            client: Client::new(),
            server_process: Arc::new(Mutex::new(None)),
            model_name: Arc::new(Mutex::new(String::new())),
        }
    }

    pub async fn start_server(&self, model_path: &str) -> Result<String, CoreError> {
        // Kill existing managed server process
        {
            let mut proc = self.server_process.lock().map_err(|e| CoreError::Other(e.to_string()))?;
            if let Some(ref mut child) = *proc {
                let _ = child.kill();
            }
            *proc = None;
        }

        // Kill any orphaned llama-server processes to avoid GPU memory conflicts
        let _ = Command::new("pkill").arg("-f").arg("llama-server").output();

        let model_name = std::path::Path::new(model_path)
            .file_stem()
            .and_then(|s| s.to_str())
            .unwrap_or("unknown")
            .to_string();

        let server_binary = which_llama_server();

        let child = Command::new(&server_binary)
            .arg("-m")
            .arg(model_path)
            .arg("--host")
            .arg("127.0.0.1")
            .arg("--port")
            .arg("8081")
            .arg("-c")
            .arg("8192")
            .arg("-ngl")
            .arg("99")
            .arg("--chat-template")
            .arg("chatml")
            .spawn()
            .map_err(|e| {
                CoreError::Llm(format!(
                    "Failed to start llama-server ({}): {}. Install with: brew install llama.cpp",
                    server_binary, e
                ))
            })?;

        {
            let mut proc = self.server_process.lock().map_err(|e| CoreError::Other(e.to_string()))?;
            *proc = Some(child);
            let mut name = self.model_name.lock().map_err(|e| CoreError::Other(e.to_string()))?;
            *name = model_name.clone();
        }

        // Poll until server is ready
        for _ in 0..60 {
            tokio::time::sleep(tokio::time::Duration::from_millis(500)).await;
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
                            if line.starts_with("data: ") {
                                let data = &line[6..];
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
        let body = serde_json::json!({
            "prompt": prompt,
            "suffix": suffix,
            "n_predict": opts.max_tokens,
            "temperature": opts.temperature,
            "stop": opts.stop,
            "stream": false,
        });

        let response = self
            .client
            .post(format!("{}/completion", self.server_url))
            .json(&body)
            .send()
            .await?;

        let result: serde_json::Value = response.json().await?;
        Ok(result["content"].as_str().unwrap_or("").to_string())
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

fn which_llama_server() -> String {
    let candidates = [
        "llama-server",
        "/usr/local/bin/llama-server",
        "/opt/homebrew/bin/llama-server",
    ];

    for candidate in &candidates {
        if Command::new(candidate).arg("--version").output().is_ok() {
            return candidate.to_string();
        }
    }

    "llama-server".to_string()
}
