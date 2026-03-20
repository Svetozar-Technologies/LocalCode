use std::sync::Arc;

use crate::llm::provider::{ChatMessage, ChatOptions, LLMProvider};

pub struct ContextManager {
    max_tokens: usize,
    messages: Vec<ChatMessage>,
    provider: Option<Arc<dyn LLMProvider>>,
}

impl ContextManager {
    pub fn new(max_tokens: usize) -> Self {
        Self {
            max_tokens,
            messages: Vec::new(),
            provider: None,
        }
    }

    pub fn with_provider(mut self, provider: Arc<dyn LLMProvider>) -> Self {
        self.provider = Some(provider);
        self
    }

    pub fn add_message(&mut self, msg: ChatMessage) {
        self.messages.push(msg);
    }

    pub fn messages(&self) -> &[ChatMessage] {
        &self.messages
    }

    pub fn set_messages(&mut self, messages: Vec<ChatMessage>) {
        self.messages = messages;
    }

    fn estimate_tokens(text: &str) -> usize {
        // Word-count based estimation: count words and multiply by 1.3
        // More accurate than simple len/4
        let word_count = text.split_whitespace().count();
        ((word_count as f64) * 1.3) as usize
    }

    fn total_tokens(&self) -> usize {
        self.messages
            .iter()
            .map(|m| Self::estimate_tokens(&m.content))
            .sum()
    }

    /// Check if compression is needed (at 75% capacity)
    pub fn needs_compression(&self) -> bool {
        self.total_tokens() > self.max_tokens * 3 / 4
    }

    /// Compress conversation context to fit within token limits
    pub async fn compress(&mut self) {
        if !self.needs_compression() {
            return;
        }

        if self.messages.len() <= 8 {
            // Not enough messages to compress meaningfully
            return;
        }

        // Strategy:
        // 1. Always keep system message (index 0)
        // 2. Always keep last 6 messages (recent context)
        // 3. For middle messages: preserve tool_call/tool_result pairs,
        //    summarize assistant text responses
        // 4. Use LLM for summarization if available, else naive truncation

        let system_msg = if !self.messages.is_empty() && self.messages[0].role == "system" {
            Some(self.messages[0].clone())
        } else {
            None
        };

        let start_idx = if system_msg.is_some() { 1 } else { 0 };
        let keep_last = 6;
        let total = self.messages.len();

        if total <= start_idx + keep_last {
            return;
        }

        let middle_end = total - keep_last;
        let middle: Vec<ChatMessage> = self.messages[start_idx..middle_end].to_vec();
        let last_msgs: Vec<ChatMessage> = self.messages[middle_end..].to_vec();

        // Separate tool messages from regular conversation
        let summary_text = self.summarize_middle(&middle).await;

        // Rebuild messages
        self.messages.clear();
        if let Some(sys) = system_msg {
            self.messages.push(sys);
        }
        self.messages.push(ChatMessage {
            role: "system".to_string(),
            content: format!("[Previous conversation summary]\n{}", summary_text),
            tool_calls: None,
            tool_call_id: None,
        });
        self.messages.extend(last_msgs);
    }

    async fn summarize_middle(&self, messages: &[ChatMessage]) -> String {
        // Collect key information from middle messages
        let mut tool_actions = Vec::new();
        let mut assistant_texts = Vec::new();
        let mut user_requests = Vec::new();

        for msg in messages {
            match msg.role.as_str() {
                "assistant" => {
                    if msg.tool_calls.is_some() {
                        // Preserve tool call info compactly
                        if let Some(ref calls) = msg.tool_calls {
                            for tc in calls {
                                tool_actions.push(format!(
                                    "Called {}({})",
                                    tc.function.name,
                                    tc.function.arguments.chars().take(100).collect::<String>()
                                ));
                            }
                        }
                    } else if !msg.content.is_empty() {
                        assistant_texts.push(msg.content.clone());
                    }
                }
                "tool" => {
                    // Keep tool results compact
                    let result_preview: String = msg.content.chars().take(200).collect();
                    tool_actions.push(format!("Result: {}", result_preview));
                }
                "user" => {
                    user_requests.push(msg.content.clone());
                }
                _ => {}
            }
        }

        // Try LLM-based summarization
        if let Some(ref provider) = self.provider {
            let combined = format!(
                "User requests: {}\n\nActions taken: {}\n\nAssistant responses: {}",
                user_requests.join("\n"),
                tool_actions.join("\n"),
                assistant_texts.iter().map(|t| {
                    t.chars().take(500).collect::<String>()
                }).collect::<Vec<_>>().join("\n---\n")
            );

            let summary_prompt = vec![ChatMessage {
                role: "user".to_string(),
                content: format!(
                    "Summarize this conversation history into a concise paragraph. \
                     Focus on what was done, what was found, and key decisions made:\n\n{}",
                    combined.chars().take(3000).collect::<String>()
                ),
                tool_calls: None,
                tool_call_id: None,
            }];

            let opts = ChatOptions {
                temperature: 0.3,
                max_tokens: 500,
                stream: false,
                system: Some("You are a conversation summarizer. Be extremely concise.".to_string()),
                ..Default::default()
            };

            if let Ok(response) = provider.chat_sync(summary_prompt, opts).await {
                return response.content;
            }
        }

        // Fallback: naive summarization
        let mut summary = String::new();

        if !user_requests.is_empty() {
            summary.push_str("User asked: ");
            for req in &user_requests {
                summary.push_str(&req.chars().take(200).collect::<String>());
                summary.push_str(". ");
            }
            summary.push('\n');
        }

        if !tool_actions.is_empty() {
            summary.push_str("Actions: ");
            for action in tool_actions.iter().take(10) {
                summary.push_str(action);
                summary.push_str("; ");
            }
            summary.push('\n');
        }

        if !assistant_texts.is_empty() {
            summary.push_str("Key points: ");
            for text in &assistant_texts {
                summary.push_str(&text.chars().take(500).collect::<String>());
                summary.push('\n');
            }
        }

        summary
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn make_msg(role: &str, content: &str) -> ChatMessage {
        ChatMessage {
            role: role.to_string(),
            content: content.to_string(),
            tool_calls: None,
            tool_call_id: None,
        }
    }

    #[test]
    fn test_add_messages() {
        let mut cm = ContextManager::new(10000);
        cm.add_message(make_msg("user", "hello"));
        cm.add_message(make_msg("assistant", "hi there"));
        cm.add_message(make_msg("user", "how are you?"));
        assert_eq!(cm.messages().len(), 3);
        assert_eq!(cm.messages()[0].role, "user");
        assert_eq!(cm.messages()[1].role, "assistant");
        assert_eq!(cm.messages()[2].content, "how are you?");
    }

    #[test]
    fn test_token_estimation() {
        // "the quick brown fox jumps over the lazy dog" = 9 words
        // estimate_tokens counts words * 1.3 => 9 * 1.3 = 11.7 => 11
        let tokens = ContextManager::estimate_tokens("the quick brown fox jumps over the lazy dog");
        assert!(tokens >= 9, "Token estimate should be at least the word count");
        assert!(tokens <= 20, "Token estimate should be reasonable for 9 words");
    }

    #[test]
    fn test_needs_compression_under_threshold() {
        // max_tokens = 1000, threshold is 75% = 750
        let mut cm = ContextManager::new(1000);
        // Add a short message that will be well under threshold
        cm.add_message(make_msg("user", "hello world"));
        assert!(
            !cm.needs_compression(),
            "Should not need compression when well below 75% capacity"
        );
    }

    #[test]
    fn test_needs_compression_over_threshold() {
        // max_tokens = 10, threshold is 75% = 7
        // We need content whose estimated tokens exceed 7
        let mut cm = ContextManager::new(10);
        // "one two three four five six seven eight nine ten" = 10 words => ~13 tokens
        cm.add_message(make_msg(
            "user",
            "one two three four five six seven eight nine ten",
        ));
        assert!(
            cm.needs_compression(),
            "Should need compression when above 75% capacity"
        );
    }
}
