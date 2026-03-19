use async_trait::async_trait;
use futures::Stream;
use serde::{Deserialize, Serialize};
use std::pin::Pin;

use crate::CoreError;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ChatMessage {
    pub role: String,
    pub content: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub tool_calls: Option<Vec<ToolCall>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub tool_call_id: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ToolCall {
    pub id: String,
    #[serde(rename = "type")]
    pub call_type: String,
    pub function: ToolCallFunction,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ToolCallFunction {
    pub name: String,
    pub arguments: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ToolDefinition {
    #[serde(rename = "type")]
    pub tool_type: String,
    pub function: ToolFunctionDef,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ToolFunctionDef {
    pub name: String,
    pub description: String,
    pub parameters: serde_json::Value,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ChatOptions {
    #[serde(default = "default_temperature")]
    pub temperature: f32,
    #[serde(default = "default_max_tokens")]
    pub max_tokens: u32,
    #[serde(default)]
    pub tools: Vec<ToolDefinition>,
    #[serde(default)]
    pub stream: bool,
    pub system: Option<String>,
    pub stop: Option<Vec<String>>,
}

fn default_temperature() -> f32 {
    0.7
}
fn default_max_tokens() -> u32 {
    4096
}

impl Default for ChatOptions {
    fn default() -> Self {
        Self {
            temperature: 0.7,
            max_tokens: 4096,
            tools: Vec::new(),
            stream: true,
            system: None,
            stop: None,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CompletionOptions {
    pub max_tokens: u32,
    pub temperature: f32,
    pub stop: Vec<String>,
}

impl Default for CompletionOptions {
    fn default() -> Self {
        Self {
            max_tokens: 128,
            temperature: 0.2,
            stop: vec!["\n\n".to_string(), "\r\n\r\n".to_string()],
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum ChatChunk {
    Text(String),
    ToolCallStart {
        id: String,
        name: String,
    },
    ToolCallDelta {
        id: String,
        arguments_delta: String,
    },
    ToolCallEnd {
        id: String,
    },
    Done,
    Error(String),
}

#[derive(Debug, Clone)]
pub struct ProviderCapabilities {
    pub chat: bool,
    pub completion: bool,
    pub embeddings: bool,
    pub tool_calling: bool,
    pub streaming: bool,
    pub vision: bool,
}

pub type ChatStream = Pin<Box<dyn Stream<Item = Result<ChatChunk, CoreError>> + Send>>;

#[async_trait]
pub trait LLMProvider: Send + Sync {
    fn name(&self) -> &str;

    async fn chat(
        &self,
        messages: Vec<ChatMessage>,
        opts: ChatOptions,
    ) -> Result<ChatStream, CoreError>;

    async fn chat_sync(
        &self,
        messages: Vec<ChatMessage>,
        opts: ChatOptions,
    ) -> Result<ChatMessage, CoreError>;

    async fn complete(
        &self,
        prompt: &str,
        suffix: &str,
        opts: CompletionOptions,
    ) -> Result<String, CoreError>;

    async fn embed(&self, texts: &[String]) -> Result<Vec<Vec<f32>>, CoreError>;

    fn capabilities(&self) -> ProviderCapabilities;
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_chat_message_serialization() {
        let msg = ChatMessage {
            role: "user".to_string(),
            content: "Hello, world!".to_string(),
            tool_calls: None,
            tool_call_id: None,
        };

        let json = serde_json::to_string(&msg).unwrap();
        let deserialized: ChatMessage = serde_json::from_str(&json).unwrap();

        assert_eq!(deserialized.role, "user");
        assert_eq!(deserialized.content, "Hello, world!");
        assert!(deserialized.tool_calls.is_none());
        assert!(deserialized.tool_call_id.is_none());
    }

    #[test]
    fn test_chat_message_with_tool_calls_serialization() {
        let msg = ChatMessage {
            role: "assistant".to_string(),
            content: String::new(),
            tool_calls: Some(vec![ToolCall {
                id: "call_123".to_string(),
                call_type: "function".to_string(),
                function: ToolCallFunction {
                    name: "read_file".to_string(),
                    arguments: r#"{"path":"test.txt"}"#.to_string(),
                },
            }]),
            tool_call_id: None,
        };

        let json = serde_json::to_string(&msg).unwrap();
        let deserialized: ChatMessage = serde_json::from_str(&json).unwrap();

        assert_eq!(deserialized.role, "assistant");
        let tool_calls = deserialized.tool_calls.unwrap();
        assert_eq!(tool_calls.len(), 1);
        assert_eq!(tool_calls[0].id, "call_123");
        assert_eq!(tool_calls[0].function.name, "read_file");
    }

    #[test]
    fn test_chat_options_defaults() {
        let opts = ChatOptions::default();

        assert!((opts.temperature - 0.7).abs() < f32::EPSILON);
        assert_eq!(opts.max_tokens, 4096);
        assert!(opts.tools.is_empty());
        assert!(opts.stream);
        assert!(opts.system.is_none());
        assert!(opts.stop.is_none());
    }

    #[test]
    fn test_completion_options_defaults() {
        let opts = CompletionOptions::default();

        assert_eq!(opts.max_tokens, 128);
        assert!((opts.temperature - 0.2).abs() < f32::EPSILON);
        assert!(!opts.stop.is_empty());
    }
}
