use async_trait::async_trait;
use futures::StreamExt;
use reqwest::Client;
use tokio_stream::wrappers::ReceiverStream;

use super::provider::*;
use crate::CoreError;

pub struct OpenAIProvider {
    api_key: String,
    base_url: String,
    model: String,
    client: Client,
}

impl OpenAIProvider {
    pub fn new(api_key: &str, model: &str) -> Self {
        Self {
            api_key: api_key.to_string(),
            base_url: "https://api.openai.com/v1".to_string(),
            model: model.to_string(),
            client: Client::new(),
        }
    }

    pub fn with_base_url(api_key: &str, base_url: &str, model: &str) -> Self {
        Self {
            api_key: api_key.to_string(),
            base_url: base_url.trim_end_matches('/').to_string(),
            model: model.to_string(),
            client: Client::new(),
        }
    }
}

#[async_trait]
impl LLMProvider for OpenAIProvider {
    fn name(&self) -> &str {
        "openai"
    }

    async fn chat(
        &self,
        messages: Vec<ChatMessage>,
        opts: ChatOptions,
    ) -> Result<ChatStream, CoreError> {
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
            "model": self.model,
            "messages": api_messages,
            "stream": true,
            "temperature": opts.temperature,
            "max_tokens": opts.max_tokens,
        });

        if !opts.tools.is_empty() {
            body["tools"] = serde_json::to_value(&opts.tools)?;
        }

        if let Some(ref stop) = opts.stop {
            body["stop"] = serde_json::to_value(stop)?;
        }

        let response = self
            .client
            .post(format!("{}/chat/completions", self.base_url))
            .header("Authorization", format!("Bearer {}", self.api_key))
            .header("Content-Type", "application/json")
            .json(&body)
            .send()
            .await?;

        if !response.status().is_success() {
            let status = response.status();
            let body = response.text().await.unwrap_or_default();
            return Err(CoreError::Llm(format!("OpenAI API error {}: {}", status, body)));
        }

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
                            if !line.starts_with("data: ") {
                                continue;
                            }
                            let data = &line[6..];
                            if data == "[DONE]" {
                                let _ = tx.send(Ok(ChatChunk::Done)).await;
                                return;
                            }

                            if let Ok(parsed) = serde_json::from_str::<serde_json::Value>(data) {
                                let delta = &parsed["choices"][0]["delta"];

                                // Text content
                                if let Some(content) = delta["content"].as_str() {
                                    if !content.is_empty() {
                                        let _ = tx
                                            .send(Ok(ChatChunk::Text(content.to_string())))
                                            .await;
                                    }
                                }

                                // Tool calls
                                if let Some(tool_calls) = delta["tool_calls"].as_array() {
                                    for tc in tool_calls {
                                        let id = tc["id"].as_str().unwrap_or("").to_string();
                                        if let Some(func) = tc.get("function") {
                                            if let Some(name) = func["name"].as_str() {
                                                let _ = tx
                                                    .send(Ok(ChatChunk::ToolCallStart {
                                                        id: id.clone(),
                                                        name: name.to_string(),
                                                    }))
                                                    .await;
                                            }
                                            if let Some(args) = func["arguments"].as_str() {
                                                if !args.is_empty() {
                                                    let _ = tx
                                                        .send(Ok(ChatChunk::ToolCallDelta {
                                                            id: id.clone(),
                                                            arguments_delta: args.to_string(),
                                                        }))
                                                        .await;
                                                }
                                            }
                                        }
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
            "model": self.model,
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
            .post(format!("{}/chat/completions", self.base_url))
            .header("Authorization", format!("Bearer {}", self.api_key))
            .header("Content-Type", "application/json")
            .json(&body)
            .send()
            .await?;

        if !response.status().is_success() {
            let status = response.status();
            let body = response.text().await.unwrap_or_default();
            return Err(CoreError::Llm(format!("OpenAI API error {}: {}", status, body)));
        }

        let result: serde_json::Value = response.json().await?;
        let message = &result["choices"][0]["message"];

        let content = message["content"].as_str().unwrap_or("").to_string();
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
        _suffix: &str,
        opts: CompletionOptions,
    ) -> Result<String, CoreError> {
        // Use chat completions for FIM-style completion
        let messages = vec![ChatMessage {
            role: "user".to_string(),
            content: format!("Complete the following code. Only output the completion, no explanation:\n\n{}", prompt),
            tool_calls: None,
            tool_call_id: None,
        }];

        let chat_opts = ChatOptions {
            temperature: opts.temperature,
            max_tokens: opts.max_tokens,
            stream: false,
            ..Default::default()
        };

        let result = self.chat_sync(messages, chat_opts).await?;
        Ok(result.content)
    }

    async fn embed(&self, texts: &[String]) -> Result<Vec<Vec<f32>>, CoreError> {
        let body = serde_json::json!({
            "model": "text-embedding-3-small",
            "input": texts,
        });

        let response = self
            .client
            .post(format!("{}/embeddings", self.base_url))
            .header("Authorization", format!("Bearer {}", self.api_key))
            .header("Content-Type", "application/json")
            .json(&body)
            .send()
            .await?;

        if !response.status().is_success() {
            let status = response.status();
            let body = response.text().await.unwrap_or_default();
            return Err(CoreError::Llm(format!("OpenAI embeddings error {}: {}", status, body)));
        }

        let result: serde_json::Value = response.json().await?;
        let embeddings: Vec<Vec<f32>> = result["data"]
            .as_array()
            .unwrap_or(&Vec::new())
            .iter()
            .filter_map(|item| {
                item["embedding"]
                    .as_array()
                    .map(|arr| arr.iter().filter_map(|v| v.as_f64().map(|f| f as f32)).collect())
            })
            .collect();

        Ok(embeddings)
    }

    fn capabilities(&self) -> ProviderCapabilities {
        ProviderCapabilities {
            chat: true,
            completion: true,
            embeddings: true,
            tool_calling: true,
            streaming: true,
            vision: true,
        }
    }
}
