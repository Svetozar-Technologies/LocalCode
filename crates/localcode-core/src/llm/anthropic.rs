use async_trait::async_trait;
use futures::StreamExt;
use reqwest::Client;
use tokio_stream::wrappers::ReceiverStream;

use super::provider::*;
use crate::CoreError;

pub struct AnthropicProvider {
    api_key: String,
    model: String,
    client: Client,
}

impl AnthropicProvider {
    pub fn new(api_key: &str, model: &str) -> Self {
        Self {
            api_key: api_key.to_string(),
            model: model.to_string(),
            client: Client::new(),
        }
    }
}

#[async_trait]
impl LLMProvider for AnthropicProvider {
    fn name(&self) -> &str {
        "anthropic"
    }

    async fn chat(
        &self,
        messages: Vec<ChatMessage>,
        opts: ChatOptions,
    ) -> Result<ChatStream, CoreError> {
        let api_messages: Vec<serde_json::Value> = messages
            .iter()
            .filter(|m| m.role != "system")
            .map(|msg| {
                if msg.role == "tool" {
                    serde_json::json!({
                        "role": "user",
                        "content": [{
                            "type": "tool_result",
                            "tool_use_id": msg.tool_call_id.as_deref().unwrap_or(""),
                            "content": msg.content
                        }]
                    })
                } else {
                    serde_json::json!({
                        "role": msg.role,
                        "content": msg.content
                    })
                }
            })
            .collect();

        let mut body = serde_json::json!({
            "model": self.model,
            "messages": api_messages,
            "max_tokens": opts.max_tokens,
            "temperature": opts.temperature,
            "stream": true,
        });

        if let Some(ref system) = opts.system {
            body["system"] = serde_json::json!(system);
        }

        if !opts.tools.is_empty() {
            let anthropic_tools: Vec<serde_json::Value> = opts
                .tools
                .iter()
                .map(|t| {
                    serde_json::json!({
                        "name": t.function.name,
                        "description": t.function.description,
                        "input_schema": t.function.parameters,
                    })
                })
                .collect();
            body["tools"] = serde_json::json!(anthropic_tools);
        }

        let response = self
            .client
            .post("https://api.anthropic.com/v1/messages")
            .header("x-api-key", &self.api_key)
            .header("anthropic-version", "2023-06-01")
            .header("Content-Type", "application/json")
            .json(&body)
            .send()
            .await?;

        if !response.status().is_success() {
            let status = response.status();
            let body = response.text().await.unwrap_or_default();
            return Err(CoreError::Llm(format!(
                "Anthropic API error {}: {}",
                status, body
            )));
        }

        let (tx, rx) = tokio::sync::mpsc::channel(100);
        let mut stream = response.bytes_stream();

        tokio::spawn(async move {
            let mut buffer = String::new();
            let mut current_tool_id = String::new();

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

                            if let Ok(parsed) = serde_json::from_str::<serde_json::Value>(data) {
                                let event_type = parsed["type"].as_str().unwrap_or("");

                                match event_type {
                                    "content_block_start" => {
                                        let block = &parsed["content_block"];
                                        if block["type"] == "tool_use" {
                                            let id =
                                                block["id"].as_str().unwrap_or("").to_string();
                                            let name =
                                                block["name"].as_str().unwrap_or("").to_string();
                                            current_tool_id = id.clone();
                                            let _ = tx
                                                .send(Ok(ChatChunk::ToolCallStart {
                                                    id,
                                                    name,
                                                }))
                                                .await;
                                        }
                                    }
                                    "content_block_delta" => {
                                        let delta = &parsed["delta"];
                                        if delta["type"] == "text_delta" {
                                            if let Some(text) = delta["text"].as_str() {
                                                let _ = tx
                                                    .send(Ok(ChatChunk::Text(text.to_string())))
                                                    .await;
                                            }
                                        } else if delta["type"] == "input_json_delta" {
                                            if let Some(json) = delta["partial_json"].as_str() {
                                                let _ = tx
                                                    .send(Ok(ChatChunk::ToolCallDelta {
                                                        id: current_tool_id.clone(),
                                                        arguments_delta: json.to_string(),
                                                    }))
                                                    .await;
                                            }
                                        }
                                    }
                                    "content_block_stop" => {
                                        if !current_tool_id.is_empty() {
                                            let _ = tx
                                                .send(Ok(ChatChunk::ToolCallEnd {
                                                    id: current_tool_id.clone(),
                                                }))
                                                .await;
                                            current_tool_id.clear();
                                        }
                                    }
                                    "message_stop" => {
                                        let _ = tx.send(Ok(ChatChunk::Done)).await;
                                        return;
                                    }
                                    _ => {}
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
        let api_messages: Vec<serde_json::Value> = messages
            .iter()
            .filter(|m| m.role != "system")
            .map(|msg| {
                serde_json::json!({
                    "role": msg.role,
                    "content": msg.content
                })
            })
            .collect();

        let mut body = serde_json::json!({
            "model": self.model,
            "messages": api_messages,
            "max_tokens": opts.max_tokens,
            "temperature": opts.temperature,
        });

        if let Some(ref system) = opts.system {
            body["system"] = serde_json::json!(system);
        }

        if !opts.tools.is_empty() {
            let anthropic_tools: Vec<serde_json::Value> = opts
                .tools
                .iter()
                .map(|t| {
                    serde_json::json!({
                        "name": t.function.name,
                        "description": t.function.description,
                        "input_schema": t.function.parameters,
                    })
                })
                .collect();
            body["tools"] = serde_json::json!(anthropic_tools);
        }

        let response = self
            .client
            .post("https://api.anthropic.com/v1/messages")
            .header("x-api-key", &self.api_key)
            .header("anthropic-version", "2023-06-01")
            .header("Content-Type", "application/json")
            .json(&body)
            .send()
            .await?;

        if !response.status().is_success() {
            let status = response.status();
            let body = response.text().await.unwrap_or_default();
            return Err(CoreError::Llm(format!(
                "Anthropic API error {}: {}",
                status, body
            )));
        }

        let result: serde_json::Value = response.json().await?;

        let mut content = String::new();
        let mut tool_calls = Vec::new();

        if let Some(blocks) = result["content"].as_array() {
            for block in blocks {
                match block["type"].as_str() {
                    Some("text") => {
                        content.push_str(block["text"].as_str().unwrap_or(""));
                    }
                    Some("tool_use") => {
                        tool_calls.push(ToolCall {
                            id: block["id"].as_str().unwrap_or("").to_string(),
                            call_type: "function".to_string(),
                            function: ToolCallFunction {
                                name: block["name"].as_str().unwrap_or("").to_string(),
                                arguments: serde_json::to_string(&block["input"])
                                    .unwrap_or_default(),
                            },
                        });
                    }
                    _ => {}
                }
            }
        }

        Ok(ChatMessage {
            role: "assistant".to_string(),
            content,
            tool_calls: if tool_calls.is_empty() {
                None
            } else {
                Some(tool_calls)
            },
            tool_call_id: None,
        })
    }

    async fn complete(
        &self,
        prompt: &str,
        _suffix: &str,
        opts: CompletionOptions,
    ) -> Result<String, CoreError> {
        let messages = vec![ChatMessage {
            role: "user".to_string(),
            content: format!(
                "Complete the following code. Only output the completion, no explanation:\n\n{}",
                prompt
            ),
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

    async fn embed(&self, _texts: &[String]) -> Result<Vec<Vec<f32>>, CoreError> {
        Err(CoreError::Llm(
            "Anthropic does not support embeddings".to_string(),
        ))
    }

    fn capabilities(&self) -> ProviderCapabilities {
        ProviderCapabilities {
            chat: true,
            completion: true,
            embeddings: false,
            tool_calling: true,
            streaming: true,
            vision: true,
        }
    }
}
