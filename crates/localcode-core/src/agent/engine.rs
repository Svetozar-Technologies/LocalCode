use std::sync::Arc;

use super::context::ContextManager;
use super::memory::MemoryManager;
use super::session::SessionState;
use super::tools::{AgentStep, ToolContext, ToolRegistry, now_ms};
use crate::llm::provider::*;
use crate::llm::streaming::collect_stream_message;
use crate::CoreError;

pub struct AgentEngine {
    provider: Arc<dyn LLMProvider>,
    registry: ToolRegistry,
    max_iterations: usize,
    system_prompt: String,
    context_manager: Option<ContextManager>,
    memory_manager: Option<MemoryManager>,
    session_state: Option<SessionState>,
}

#[derive(Debug, Clone)]
pub enum AgentEvent {
    Step(AgentStep),
    TextChunk(String),
    Done(String),
    Error(String),
}

impl AgentEngine {
    pub fn new(provider: Arc<dyn LLMProvider>, registry: ToolRegistry) -> Self {
        Self {
            provider,
            registry,
            max_iterations: 15,
            system_prompt: String::new(),
            context_manager: None,
            memory_manager: None,
            session_state: None,
        }
    }

    pub fn with_max_iterations(mut self, max: usize) -> Self {
        self.max_iterations = max;
        self
    }

    pub fn with_system_prompt(mut self, prompt: String) -> Self {
        self.system_prompt = prompt;
        self
    }

    /// Initialize memory, auto-discovery, and session for a project
    pub fn initialize(&mut self, project_path: &str) {
        // Set up memory manager with auto-discovery
        let mut memory = MemoryManager::new();

        // Auto-discover project if stale (> 1 hour)
        if memory.needs_reindex(project_path, 3600) {
            memory.auto_discover_project(project_path);
            let _ = memory.save();
        }

        // Build enriched system prompt from memory
        let memory_ctx = memory.build_context(project_path);
        if !memory_ctx.is_empty() {
            if self.system_prompt.is_empty() {
                self.system_prompt = format!(
                    "You are LocalCode Agent, an autonomous AI coding assistant.\n\n{}",
                    memory_ctx
                );
            } else {
                self.system_prompt.push_str("\n\n");
                self.system_prompt.push_str(&memory_ctx);
            }
        }

        // Set up context manager
        let ctx_manager = ContextManager::new(self.max_iterations * 4096)
            .with_provider(self.provider.clone());
        self.context_manager = Some(ctx_manager);

        // Load or create session state
        let session = SessionState::load(project_path)
            .ok()
            .flatten()
            .unwrap_or_else(|| SessionState::new(project_path));
        self.session_state = Some(session);

        self.memory_manager = Some(memory);
    }

    fn build_system_prompt(&self, ctx: &ToolContext) -> String {
        let tools_desc: Vec<String> = self
            .registry
            .list()
            .iter()
            .enumerate()
            .map(|(i, name)| {
                let tool = self.registry.get(name).unwrap();
                format!("{}. {} - {}", i + 1, name, tool.description())
            })
            .collect();

        let base = if self.system_prompt.is_empty() {
            "You are LocalCode Agent, an autonomous AI coding assistant.".to_string()
        } else {
            self.system_prompt.clone()
        };

        format!(
            "{}\n\nAvailable tools:\n{}\n\nProject path: {}\n\n\
             IMPORTANT: All file paths must be RELATIVE to the project root. \
             For example, use 'src/main.rs' not '/src/main.rs'. \
             Use '.' or omit path to refer to the project root directory. \
             Never use absolute paths like '/home/...' or '~/...'. \
             Start by listing the project directory to understand its structure.\n\n\
             Use tools to accomplish the task. Call tools one at a time. When done, provide a summary.",
            base,
            tools_desc.join("\n"),
            ctx.project_path,
        )
    }

    /// Track file modifications from tool results
    fn track_file_modification(&mut self, tool_name: &str, args: &serde_json::Value) {
        if let Some(ref mut session) = self.session_state {
            match tool_name {
                "write_file" | "edit_file" | "create_file" | "delete_file" => {
                    if let Some(path) = args.get("path").and_then(|p| p.as_str()) {
                        session.add_file_modified(path);
                    }
                }
                _ => {}
            }
        }
    }

    /// Save session summary when done
    fn finalize_session(&mut self, task: &str, final_response: &str) {
        if let (Some(ref mut memory), Some(ref session)) =
            (&mut self.memory_manager, &self.session_state)
        {
            let summary = super::memory::SessionSummary {
                timestamp: std::time::SystemTime::now()
                    .duration_since(std::time::UNIX_EPOCH)
                    .unwrap_or_default()
                    .as_secs(),
                task: task.to_string(),
                files_modified: session.files_modified.clone(),
                tasks_completed: session.tasks_completed.clone(),
                summary: final_response.chars().take(500).collect(),
            };

            memory.save_session_summary(&session.project_path, summary);
            let _ = memory.save();
        }

        if let Some(ref session) = self.session_state {
            let _ = session.save();
        }
    }

    /// Execute agent with native tool calling (OpenAI/Anthropic format)
    pub async fn execute_native(
        &mut self,
        task: &str,
        ctx: &ToolContext,
        on_event: &(dyn Fn(AgentEvent) + Send + Sync),
    ) -> Result<String, CoreError> {
        let system = self.build_system_prompt(ctx);
        let tool_defs = self.registry.tool_definitions();

        let mut messages = vec![ChatMessage {
            role: "user".to_string(),
            content: task.to_string(),
            tool_calls: None,
            tool_call_id: None,
        }];

        // Add to context manager if available
        if let Some(ref mut cm) = self.context_manager {
            cm.add_message(ChatMessage {
                role: "system".to_string(),
                content: system.clone(),
                tool_calls: None,
                tool_call_id: None,
            });
            cm.add_message(messages[0].clone());
        }

        for _iteration in 0..self.max_iterations {
            let opts = ChatOptions {
                temperature: 0.3,
                max_tokens: 4096,
                tools: tool_defs.clone(),
                stream: true,
                system: Some(system.clone()),
                stop: None,
            };

            let stream = self.provider.chat(messages.clone(), opts).await?;
            let response = collect_stream_message(stream).await?;

            // Check for tool calls
            if let Some(ref tool_calls) = response.tool_calls {
                // Emit the text portion if any
                if !response.content.is_empty() {
                    on_event(AgentEvent::TextChunk(response.content.clone()));
                }

                messages.push(response.clone());
                if let Some(ref mut cm) = self.context_manager {
                    cm.add_message(response.clone());
                }

                for tc in tool_calls {
                    let args: serde_json::Value =
                        serde_json::from_str(&tc.function.arguments).unwrap_or_default();

                    on_event(AgentEvent::Step(AgentStep {
                        step_type: "tool_call".to_string(),
                        tool: Some(tc.function.name.clone()),
                        args: Some(args.clone()),
                        result: None,
                        content: None,
                        timestamp: now_ms(),
                    }));

                    // Track file modifications
                    self.track_file_modification(&tc.function.name, &args);

                    let result = self
                        .registry
                        .execute(&tc.function.name, args, ctx)
                        .await
                        .unwrap_or_else(|e| format!("Error: {}", e));

                    on_event(AgentEvent::Step(AgentStep {
                        step_type: "tool_result".to_string(),
                        tool: Some(tc.function.name.clone()),
                        args: None,
                        result: Some(result.clone()),
                        content: None,
                        timestamp: now_ms(),
                    }));

                    let tool_msg = ChatMessage {
                        role: "tool".to_string(),
                        content: result,
                        tool_calls: None,
                        tool_call_id: Some(tc.id.clone()),
                    };
                    messages.push(tool_msg.clone());
                    if let Some(ref mut cm) = self.context_manager {
                        cm.add_message(tool_msg);
                    }
                }

                // Check if context compression is needed
                if let Some(ref mut cm) = self.context_manager {
                    if cm.needs_compression() {
                        cm.compress().await;
                        // Use compressed messages for next iteration
                        messages = cm.messages().to_vec();
                        // Remove system message from messages list (it's passed via opts)
                        if !messages.is_empty() && messages[0].role == "system" {
                            messages.remove(0);
                        }
                        // Also remove any summary system messages
                        messages.retain(|m| !(m.role == "system" && m.content.starts_with("[Previous")));
                    }
                }
            } else {
                // No tool calls — final response
                on_event(AgentEvent::Done(response.content.clone()));
                self.finalize_session(task, &response.content);
                return Ok(response.content);
            }
        }

        let msg = "[Agent reached max iterations]".to_string();
        on_event(AgentEvent::Done(msg.clone()));
        self.finalize_session(task, &msg);
        Ok(msg)
    }

    /// Execute agent with XML tool calling (for local models without native tool calling)
    pub async fn execute_xml(
        &mut self,
        task: &str,
        ctx: &ToolContext,
        on_event: &(dyn Fn(AgentEvent) + Send + Sync),
    ) -> Result<String, CoreError> {
        let tools_desc: Vec<String> = self
            .registry
            .list()
            .iter()
            .enumerate()
            .map(|(i, name)| {
                let tool = self.registry.get(name).unwrap();
                format!("{}. {}({}) - {}", i + 1, name, "", tool.description())
            })
            .collect();

        let system = format!(
            r#"You are LocalCode Agent, an autonomous AI coding assistant.
You can use tools to accomplish tasks. Available tools:

{}

To use a tool, respond with EXACTLY this format:
<tool>tool_name</tool>
<args>{{"param": "value"}}</args>

Project path: {}

IMPORTANT: All file paths must be RELATIVE to the project root.
For example, use 'src/main.rs' not '/src/main.rs'.
Use '.' to refer to the project root directory.
Never use absolute paths like '/home/...' or '~/...'.
Start by listing the project directory to understand its structure.

Analyze the task, plan your approach, then use tools step by step.
After each tool result, decide the next action."#,
            tools_desc.join("\n"),
            ctx.project_path
        );

        let mut conversation = vec![ChatMessage {
            role: "user".to_string(),
            content: task.to_string(),
            tool_calls: None,
            tool_call_id: None,
        }];

        for _iteration in 0..self.max_iterations {
            let opts = ChatOptions {
                temperature: 0.3,
                max_tokens: 2048,
                stream: false,
                system: Some(system.clone()),
                ..Default::default()
            };

            let response = self.provider.chat_sync(conversation.clone(), opts).await?;

            if let Some((tool_name, args_str)) = parse_xml_tool_call(&response.content) {
                let args: serde_json::Value =
                    serde_json::from_str(&args_str).unwrap_or_default();

                on_event(AgentEvent::Step(AgentStep {
                    step_type: "tool_call".to_string(),
                    tool: Some(tool_name.clone()),
                    args: Some(args.clone()),
                    result: None,
                    content: None,
                    timestamp: now_ms(),
                }));

                if tool_name == "done" {
                    let summary = args["summary"]
                        .as_str()
                        .unwrap_or("Task completed")
                        .to_string();
                    on_event(AgentEvent::Done(summary.clone()));
                    self.finalize_session(task, &summary);
                    return Ok(summary);
                }

                // Track file modifications
                self.track_file_modification(&tool_name, &args);

                let result = self
                    .registry
                    .execute(&tool_name, args, ctx)
                    .await
                    .unwrap_or_else(|e| format!("Error: {}", e));

                on_event(AgentEvent::Step(AgentStep {
                    step_type: "tool_result".to_string(),
                    tool: Some(tool_name),
                    args: None,
                    result: Some(result.clone()),
                    content: None,
                    timestamp: now_ms(),
                }));

                conversation.push(ChatMessage {
                    role: "assistant".to_string(),
                    content: response.content,
                    tool_calls: None,
                    tool_call_id: None,
                });
                conversation.push(ChatMessage {
                    role: "user".to_string(),
                    content: format!("Tool result:\n{}", result),
                    tool_calls: None,
                    tool_call_id: None,
                });
            } else {
                on_event(AgentEvent::Done(response.content.clone()));
                self.finalize_session(task, &response.content);
                return Ok(response.content);
            }
        }

        let msg = "[Agent reached max iterations]".to_string();
        on_event(AgentEvent::Done(msg.clone()));
        self.finalize_session(task, &msg);
        Ok(msg)
    }

    /// Auto-select between native and XML based on provider capabilities
    pub async fn execute(
        &mut self,
        task: &str,
        ctx: &ToolContext,
        on_event: &(dyn Fn(AgentEvent) + Send + Sync),
    ) -> Result<String, CoreError> {
        if self.provider.capabilities().tool_calling {
            self.execute_native(task, ctx, on_event).await
        } else {
            self.execute_xml(task, ctx, on_event).await
        }
    }
}

fn parse_xml_tool_call(text: &str) -> Option<(String, String)> {
    let tool_start = text.find("<tool>")?;
    let tool_end = text.find("</tool>")?;
    let tool_name = text[tool_start + 6..tool_end].trim().to_string();

    let args_start = text.find("<args>")?;
    let args_end = text.find("</args>")?;
    let args_str = text[args_start + 6..args_end].trim().to_string();

    Some((tool_name, args_str))
}
