use async_trait::async_trait;
use std::collections::HashMap;
use std::sync::Arc;

use super::provider::*;
use crate::CoreError;

pub struct RouterProvider {
    providers: HashMap<String, Arc<dyn LLMProvider>>,
    default_provider: String,
}

impl RouterProvider {
    pub fn new() -> Self {
        Self {
            providers: HashMap::new(),
            default_provider: String::new(),
        }
    }

    pub fn add_provider(&mut self, name: &str, provider: Arc<dyn LLMProvider>) {
        if self.default_provider.is_empty() {
            self.default_provider = name.to_string();
        }
        self.providers.insert(name.to_string(), provider);
    }

    pub fn set_default(&mut self, name: &str) {
        if self.providers.contains_key(name) {
            self.default_provider = name.to_string();
        }
    }

    pub fn get_provider(&self, name: &str) -> Option<&Arc<dyn LLMProvider>> {
        self.providers.get(name)
    }

    pub fn default_provider(&self) -> Option<&Arc<dyn LLMProvider>> {
        self.providers.get(&self.default_provider)
    }

    pub fn list_providers(&self) -> Vec<String> {
        self.providers.keys().cloned().collect()
    }

    fn get_active(&self) -> Result<&Arc<dyn LLMProvider>, CoreError> {
        self.providers
            .get(&self.default_provider)
            .ok_or_else(|| CoreError::Llm("No provider configured".to_string()))
    }
}

impl Default for RouterProvider {
    fn default() -> Self {
        Self::new()
    }
}

#[async_trait]
impl LLMProvider for RouterProvider {
    fn name(&self) -> &str {
        "router"
    }

    async fn chat(
        &self,
        messages: Vec<ChatMessage>,
        opts: ChatOptions,
    ) -> Result<ChatStream, CoreError> {
        self.get_active()?.chat(messages, opts).await
    }

    async fn chat_sync(
        &self,
        messages: Vec<ChatMessage>,
        opts: ChatOptions,
    ) -> Result<ChatMessage, CoreError> {
        self.get_active()?.chat_sync(messages, opts).await
    }

    async fn complete(
        &self,
        prompt: &str,
        suffix: &str,
        opts: CompletionOptions,
    ) -> Result<String, CoreError> {
        self.get_active()?.complete(prompt, suffix, opts).await
    }

    async fn embed(&self, texts: &[String]) -> Result<Vec<Vec<f32>>, CoreError> {
        self.get_active()?.embed(texts).await
    }

    fn capabilities(&self) -> ProviderCapabilities {
        self.providers
            .get(&self.default_provider)
            .map(|p| p.capabilities())
            .unwrap_or(ProviderCapabilities {
                chat: false,
                completion: false,
                embeddings: false,
                tool_calling: false,
                streaming: false,
                vision: false,
            })
    }
}
