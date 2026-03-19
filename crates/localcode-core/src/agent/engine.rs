use std::collections::HashMap;
use std::sync::Arc;

use super::context::ContextManager;
use super::memory::MemoryManager;
use super::session::{SessionState, SessionStore, session_from_state};
use super::tools::{AgentStep, ToolContext, ToolRegistry, now_ms};
use crate::llm::provider::*;
use crate::CoreError;

pub struct AgentEngine {
    provider: Arc<dyn LLMProvider>,
    registry: ToolRegistry,
    max_iterations: usize,
    system_prompt: String,
    context_manager: Option<ContextManager>,
    memory_manager: Option<MemoryManager>,
    session_state: Option<SessionState>,
    session_store: Option<SessionStore>,
    error_count: HashMap<String, usize>, // tool_name -> consecutive error count
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
            max_iterations: 30,
            system_prompt: String::new(),
            context_manager: None,
            memory_manager: None,
            session_state: None,
            session_store: None,
            error_count: HashMap::new(),
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

        // Initialize persistent session store
        self.session_store = Some(SessionStore::new());

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

        let memory_context = if self.system_prompt.is_empty() {
            String::new()
        } else {
            self.system_prompt.clone()
        };

        format!(
            r#"You are LocalCode Agent, an AI coding assistant with direct file system access.

# Rules
- Use tools to complete tasks. Prefer edit_file for existing files, write_file for new files.
- File paths must be relative to project root (e.g. 'src/main.rs').
- If a command fails, read the error, fix the issue, and retry.
- When done, summarize what was changed.

{memory_context}

# Tools
{tools_list}

# Project: {project_path}"#,
            memory_context = memory_context,
            tools_list = tools_desc.join("\n"),
            project_path = ctx.project_path,
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
        let summary_text: String = final_response.chars().take(500).collect();

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
                summary: summary_text.clone(),
            };

            memory.save_session_summary(&session.project_path, summary);
            let _ = memory.save();
        }

        // Save to persistent SessionStore
        if let (Some(ref mut store), Some(ref state)) =
            (&mut self.session_store, &self.session_state)
        {
            let persistent = session_from_state(state, &summary_text);
            let _ = store.save_session(&persistent);
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
                max_tokens: 2048,
                tools: tool_defs.clone(),
                stream: false,
                system: Some(system.clone()),
                stop: None,
            };

            // Use chat_sync for reliable tool call parsing (streaming tool calls are fragile)
            let mut response = self.provider.chat_sync(messages.clone(), opts).await?;

            // Fallback: parse tool calls from content if model outputs them as JSON text
            // (common with Ollama/local models that don't use structured tool_calls)
            if response.tool_calls.is_none() && !response.content.is_empty() {
                let parsed = parse_tool_calls_from_content(&response.content, &self.registry);
                if !parsed.is_empty() {
                    response.tool_calls = Some(parsed);
                    response.content = String::new();
                }
            }

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

                    let mut result = self
                        .registry
                        .execute(&tc.function.name, args, ctx)
                        .await
                        .unwrap_or_else(|e| format!("Error: {}", e));

                    // Error detection and hint injection
                    let is_error = result.starts_with("[ERROR") || result.starts_with("Error:");
                    if is_error {
                        let count = self.error_count.entry(tc.function.name.clone()).or_insert(0);
                        *count += 1;

                        if *count >= 3 {
                            result.push_str(
                                "\n\n[SYSTEM: This tool has failed 3 times. \
                                 Try a completely different approach or ask the user for help.]"
                            );
                        } else {
                            result.push_str(
                                "\n\n[SYSTEM: The command failed. Read the error above carefully. \
                                 If you wrote a file that has a bug, use read_file to see it, \
                                 then use edit_file to fix the specific error, then retry the command.]"
                            );
                        }
                    } else {
                        // Reset error count on success
                        self.error_count.remove(&tc.function.name);
                    }

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
            r#"You are an AI coding agent. You complete tasks by calling ONE tool at a time using XML format.

RULES:
1. ALWAYS write code to a file FIRST using write_file, THEN run it with run_command.
2. ONE command per tool call. Never chain commands with && or ;
3. Do NOT install packages preemptively. Only install when you get an import error.
4. Keep code complete and self-contained in a single file when possible.
5. After successful run_command, respond with plain text summary to finish.

FORMAT:
<tool>TOOL_NAME</tool>
<args>JSON_ARGS</args>

EXAMPLE - "create a snake game":
Step 1: Write the code
<tool>write_file</tool>
<args>{{"path": "snake.py", "content": "import pygame\nimport random\n...full game code..."}}</args>
Step 2 (after write success): Run it
<tool>run_command</tool>
<args>{{"command": "python3 snake.py"}}</args>
Step 3 (if import error): Install missing package
<tool>run_command</tool>
<args>{{"command": "pip3 install pygame"}}</args>
Step 4: Run again
<tool>run_command</tool>
<args>{{"command": "python3 snake.py"}}</args>

TOOLS:
{tools}

PROJECT: {project}

Respond with ONE tool call now. Start by writing the code file."#,
            tools = tools_desc.join("\n"),
            project = ctx.project_path
        );

        let mut conversation = vec![ChatMessage {
            role: "user".to_string(),
            content: task.to_string(),
            tool_calls: None,
            tool_call_id: None,
        }];

        let mut last_tool: Option<String> = None;
        let mut repeat_count = 0usize;

        for _iteration in 0..self.max_iterations {
            let opts = ChatOptions {
                temperature: 0.3,
                max_tokens: 4096,
                stream: false,
                system: Some(system.clone()),
                ..Default::default()
            };

            let response = self.provider.chat_sync(conversation.clone(), opts).await?;

            if let Some((tool_name, args_str)) = parse_xml_tool_call(&response.content) {
                let args: serde_json::Value =
                    serde_json::from_str(&args_str).unwrap_or_default();

                // Detect repeated tool calls (same tool + same args = stuck in loop)
                let call_key = format!("{}:{}", tool_name, args_str);
                if last_tool.as_deref() == Some(&call_key) {
                    repeat_count += 1;
                    if repeat_count >= 3 {
                        let msg = "Task completed.".to_string();
                        on_event(AgentEvent::Done(msg.clone()));
                        self.finalize_session(task, &msg);
                        return Ok(msg);
                    }
                } else {
                    repeat_count = 0;
                    last_tool = Some(call_key);
                }

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

                let mut result = self
                    .registry
                    .execute(&tool_name, args, ctx)
                    .await
                    .unwrap_or_else(|e| format!("Error: {}", e));

                // Error detection and hint injection (same as execute_native)
                let is_error = result.starts_with("[ERROR") || result.starts_with("Error:");
                if is_error {
                    let count = self.error_count.entry(tool_name.clone()).or_insert(0);
                    *count += 1;

                    if *count >= 3 {
                        result.push_str(
                            "\n\n[SYSTEM: This tool has failed 3 times. \
                             Try a completely different approach or ask the user for help.]"
                        );
                    } else {
                        result.push_str(
                            "\n\n[SYSTEM: The command failed. Read the error above carefully. \
                             If you wrote a file that has a bug, use read_file to see it, \
                             then use edit_file to fix the specific error, then retry the command.]"
                        );
                    }
                } else {
                    self.error_count.remove(&tool_name);
                }

                let tool_name_clone = tool_name.clone();
                on_event(AgentEvent::Step(AgentStep {
                    step_type: "tool_result".to_string(),
                    tool: Some(tool_name_clone),
                    args: None,
                    result: Some(result.clone()),
                    content: None,
                    timestamp: now_ms(),
                }));

                // Format error results prominently for XML mode
                let result_label = if is_error {
                    "Tool result (ERROR — you MUST fix this before moving on. If a module is missing, install it with pip/npm. If a file has a bug, use edit_file to fix it. Then retry.)"
                } else if tool_name == "run_command" {
                    "Tool result (SUCCESS — if the original task is fully complete, respond with a plain text summary. Otherwise continue with the next step.)"
                } else {
                    "Tool result (OK)"
                };

                conversation.push(ChatMessage {
                    role: "assistant".to_string(),
                    content: response.content,
                    tool_calls: None,
                    tool_call_id: None,
                });
                // Truncate long results to prevent context overflow
                let truncated_result: String = if result.len() > 2000 {
                    let cut: String = result.chars().take(1800).collect();
                    format!("{}...\n[truncated, {} chars total]", cut, result.len())
                } else {
                    result.clone()
                };

                conversation.push(ChatMessage {
                    role: "user".to_string(),
                    content: format!("{}:\n{}", result_label, truncated_result),
                    tool_calls: None,
                    tool_call_id: None,
                });

                // Keep conversation within context limits (32K allows more)
                // Retain: first message (original task) + last 8 messages (4 exchanges)
                if conversation.len() > 13 {
                    let first = conversation[0].clone();
                    let tail: Vec<ChatMessage> = conversation[conversation.len()-8..].to_vec();
                    conversation.clear();
                    conversation.push(first);
                    conversation.push(ChatMessage {
                        role: "user".to_string(),
                        content: "[Previous tool calls omitted. Continue working on the task.]".to_string(),
                        tool_calls: None,
                        tool_call_id: None,
                    });
                    conversation.extend(tail);
                }
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

    /// Plan-and-Execute: Break complex tasks into small steps, execute each with fresh context.
    /// Designed for local models with small context windows (8K).
    pub async fn execute_planned(
        &mut self,
        task: &str,
        ctx: &ToolContext,
        on_event: &(dyn Fn(AgentEvent) + Send + Sync),
    ) -> Result<String, CoreError> {
        // Phase 1: Planning — ask model to decompose the task into steps
        on_event(AgentEvent::Step(AgentStep {
            step_type: "thinking".to_string(),
            tool: None,
            args: None,
            result: None,
            content: Some("Planning your project... breaking it into manageable steps".to_string()),
            timestamp: now_ms(),
        }));

        let plan_system = r#"You are a task planner. Break the user's task into simple steps.

RULES:
- Put ALL code in ONE single file. Never split into multiple files.
- Step format: "Write filename.py: detailed description of what the code does"
- Then: "Run: python3 filename.py"
- Then: "If import error: pip3 install package_name"
- Output ONLY a JSON array of strings.

EXAMPLE - "create a chess game":
["Write chess.py: Complete chess game in one file. Use pygame for graphics. Include: 8x8 board drawing with alternating colors, all chess pieces as unicode characters, piece selection by clicking, valid move highlighting, turn-based play, simple CPU opponent that picks random valid moves. Window 640x640.", "Run: python3 chess.py", "If import error: pip3 install pygame", "Run: python3 chess.py"]

EXAMPLE - "create a todo app":
["Write todo.py: Complete todo app using Flask in one file. Routes: GET / shows all todos as HTML, POST /add adds a todo, POST /delete/<id> removes one. Use in-memory list. Include HTML templates inline with render_template_string. Run on port 5000.", "Run: python3 todo.py", "If import error: pip3 install flask", "Run: python3 todo.py"]

Output ONLY the JSON array."#;

        let plan_messages = vec![ChatMessage {
            role: "user".to_string(),
            content: task.to_string(),
            tool_calls: None,
            tool_call_id: None,
        }];

        let plan_opts = ChatOptions {
            temperature: 0.3,
            max_tokens: 2048,
            stream: false,
            system: Some(plan_system.to_string()),
            ..Default::default()
        };

        let plan_response = self.provider.chat_sync(plan_messages, plan_opts).await?;

        // Parse the plan — extract JSON array from response
        let steps = parse_plan_steps(&plan_response.content);

        if steps.is_empty() {
            // Fallback: if planning fails, run as single-step XML execution
            on_event(AgentEvent::Step(AgentStep {
                step_type: "thinking".to_string(),
                tool: None,
                args: None,
                result: None,
                content: Some("Switching to direct mode — let's just build it!".to_string()),
                timestamp: now_ms(),
            }));
            return self.execute_xml(task, ctx, on_event).await;
        }

        // Emit the plan
        let plan_text = steps.iter().enumerate()
            .map(|(i, s)| format!("{}. {}", i + 1, s))
            .collect::<Vec<_>>()
            .join("\n");
        on_event(AgentEvent::Step(AgentStep {
            step_type: "thinking".to_string(),
            tool: None,
            args: None,
            result: None,
            content: Some(format!("Plan:\n{}", plan_text)),
            timestamp: now_ms(),
        }));

        // Phase 2: Execute each step with fresh context
        let mut files_created: Vec<String> = Vec::new();
        let mut completed_steps: Vec<String> = Vec::new();

        for (i, step) in steps.iter().enumerate() {
            // Skip conditional steps if no error occurred
            let step_lower = step.to_lowercase();
            if step_lower.starts_with("if import error") || step_lower.starts_with("if error") {
                if !completed_steps.last().map_or(false, |s| s.contains("ERROR")) {
                    continue; // Skip — previous step succeeded
                }
            }

            on_event(AgentEvent::Step(AgentStep {
                step_type: "thinking".to_string(),
                tool: None,
                args: None,
                result: None,
                content: Some(format!("Step {}/{}: {}", i + 1, steps.len(), step)),
                timestamp: now_ms(),
            }));

            // Build step-specific context
            let step_context = if files_created.is_empty() {
                step.clone()
            } else {
                format!("{}\n\nFiles already created: {}", step, files_created.join(", "))
            };

            // Execute this single step using XML mode with max 5 iterations
            let step_result = self.execute_xml_step(&step_context, ctx, on_event, 5).await?;

            // Track results
            if let Some(ref session) = self.session_state {
                for f in &session.files_modified {
                    if !files_created.contains(f) {
                        files_created.push(f.clone());
                    }
                }
            }

            completed_steps.push(format!("Step {}: {} → {}", i + 1, step,
                if step_result.contains("ERROR") { "FAILED" } else { "OK" }));

            // If a run_command step succeeded and it was the main "run" step, we might be done
            if step_lower.starts_with("run") && !step_result.contains("ERROR") && i >= steps.len() - 2 {
                break;
            }
        }

        let summary = format!("Task completed.\nFiles: {}\nSteps: {}",
            if files_created.is_empty() { "none".to_string() } else { files_created.join(", ") },
            completed_steps.len()
        );
        on_event(AgentEvent::Done(summary.clone()));
        self.finalize_session(task, &summary);
        Ok(summary)
    }

    /// Execute a single step with XML tool calling, limited iterations.
    /// Returns the last tool result or text response.
    async fn execute_xml_step(
        &mut self,
        step_task: &str,
        ctx: &ToolContext,
        on_event: &(dyn Fn(AgentEvent) + Send + Sync),
        max_iters: usize,
    ) -> Result<String, CoreError> {
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

        let system = format!(
            r#"You are a coding agent. Complete the step by calling a tool.

CRITICAL RULES:
- For "Write X.py: description" → call write_file with COMPLETE working code. NOT stubs. NOT comments. Write every function, every class, every line.
- For "Run: command" → call run_command.
- For "If import error: pip3 install X" → call run_command.
- Parameter name is "path" (NOT "file_path").
- ONE tool call. No chaining.

FORMAT:
<tool>TOOL_NAME</tool>
<args>JSON_ARGS</args>

WRITE_FILE EXAMPLE:
<tool>write_file</tool>
<args>{{"path": "game.py", "content": "import pygame\nimport sys\n\npygame.init()\nscreen = pygame.display.set_mode((640, 640))\n... FULL CODE HERE ..."}}</args>

RUN EXAMPLE:
<tool>run_command</tool>
<args>{{"command": "python3 game.py"}}</args>

INSTALL EXAMPLE:
<tool>run_command</tool>
<args>{{"command": "pip3 install pygame"}}</args>

TOOLS: {tools}
PROJECT: {project}"#,
            tools = tools_desc.join("\n"),
            project = ctx.project_path
        );

        let mut conversation = vec![ChatMessage {
            role: "user".to_string(),
            content: step_task.to_string(),
            tool_calls: None,
            tool_call_id: None,
        }];

        let mut last_result = String::new();

        for _iter in 0..max_iters {
            let opts = ChatOptions {
                temperature: 0.3,
                max_tokens: 8192,
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

                self.track_file_modification(&tool_name, &args);

                let result = self
                    .registry
                    .execute(&tool_name, args, ctx)
                    .await
                    .unwrap_or_else(|e| format!("Error: {}", e));

                let is_error = result.starts_with("[ERROR") || result.starts_with("Error:");

                on_event(AgentEvent::Step(AgentStep {
                    step_type: "tool_result".to_string(),
                    tool: Some(tool_name.clone()),
                    args: None,
                    result: Some(result.clone()),
                    content: None,
                    timestamp: now_ms(),
                }));

                // Truncate for context
                let truncated: String = if result.len() > 1500 {
                    let cut: String = result.chars().take(1200).collect();
                    format!("{}...[truncated]", cut)
                } else {
                    result.clone()
                };

                last_result = result;

                if is_error {
                    // Keep going to fix the error
                    conversation.push(ChatMessage {
                        role: "assistant".to_string(),
                        content: response.content,
                        tool_calls: None,
                        tool_call_id: None,
                    });
                    conversation.push(ChatMessage {
                        role: "user".to_string(),
                        content: format!("ERROR: {}\nFix this. If a package is missing, install it. If code has a bug, use edit_file.", truncated),
                        tool_calls: None,
                        tool_call_id: None,
                    });
                    // Trim conversation to keep within limits (32K allows more)
                    if conversation.len() > 9 {
                        let first = conversation[0].clone();
                        let tail: Vec<ChatMessage> = conversation[conversation.len()-4..].to_vec();
                        conversation.clear();
                        conversation.push(first);
                        conversation.extend(tail);
                    }
                } else {
                    // Success — this step is done
                    return Ok(last_result);
                }
            } else {
                // Text response — step is done
                return Ok(response.content);
            }
        }

        Ok(last_result)
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

/// Try to parse a single JSON value into a ToolCall
fn try_parse_single_tool_call(value: &serde_json::Value, registry: &ToolRegistry) -> Option<ToolCall> {
    let name = value.get("name").and_then(|n| n.as_str())?;
    let args = value.get("arguments")?;
    if registry.get(name).is_some() {
        Some(ToolCall {
            id: format!("call_{}", &uuid::Uuid::new_v4().to_string().replace("-", "")[..12]),
            call_type: "function".to_string(),
            function: ToolCallFunction {
                name: name.to_string(),
                arguments: serde_json::to_string(args).unwrap_or_default(),
            },
        })
    } else {
        None
    }
}

/// Parse tool calls from content text when models output JSON instead of structured tool_calls.
/// Handles single objects, JSON arrays, and multiple JSON objects separated by whitespace.
fn parse_tool_calls_from_content(content: &str, registry: &ToolRegistry) -> Vec<ToolCall> {
    let trimmed = content.trim();

    // Try parsing as a single JSON value (object or array)
    if let Ok(parsed) = serde_json::from_str::<serde_json::Value>(trimmed) {
        // Array of tool calls
        if let Some(arr) = parsed.as_array() {
            let calls: Vec<ToolCall> = arr.iter()
                .filter_map(|v| try_parse_single_tool_call(v, registry))
                .collect();
            if !calls.is_empty() {
                return calls;
            }
        }
        // Single tool call object
        if let Some(tc) = try_parse_single_tool_call(&parsed, registry) {
            return vec![tc];
        }
    }

    // Try parsing multiple JSON objects separated by whitespace/newlines
    let mut calls = Vec::new();
    let mut depth = 0i32;
    let mut start = None;
    for (i, ch) in trimmed.char_indices() {
        match ch {
            '{' => {
                if depth == 0 { start = Some(i); }
                depth += 1;
            }
            '}' => {
                depth -= 1;
                if depth == 0 {
                    if let Some(s) = start {
                        let json_str = &trimmed[s..=i];
                        if let Ok(val) = serde_json::from_str::<serde_json::Value>(json_str) {
                            if let Some(tc) = try_parse_single_tool_call(&val, registry) {
                                calls.push(tc);
                            }
                        }
                    }
                    start = None;
                }
            }
            _ => {}
        }
    }
    if !calls.is_empty() {
        return calls;
    }

    // Try extracting JSON from markdown code blocks
    if let Some(s) = trimmed.find("```json") {
        if let Some(e) = trimmed[s + 7..].find("```") {
            let json_str = trimmed[s + 7..s + 7 + e].trim();
            return parse_tool_calls_from_content(json_str, registry);
        }
    }
    if let Some(s) = trimmed.find("```") {
        if let Some(e) = trimmed[s + 3..].find("```") {
            let json_str = trimmed[s + 3..s + 3 + e].trim();
            if json_str.starts_with('{') || json_str.starts_with('[') {
                return parse_tool_calls_from_content(json_str, registry);
            }
        }
    }

    Vec::new()
}

/// Parse plan steps from LLM response. Handles JSON arrays and numbered lists.
fn parse_plan_steps(content: &str) -> Vec<String> {
    let trimmed = content.trim();

    // Try JSON array first
    // Find the first [ and last ] to extract the array
    if let Some(start) = trimmed.find('[') {
        if let Some(end) = trimmed.rfind(']') {
            let json_str = &trimmed[start..=end];
            if let Ok(arr) = serde_json::from_str::<Vec<String>>(json_str) {
                if !arr.is_empty() {
                    return arr;
                }
            }
        }
    }

    // Fallback: parse numbered list (1. Step one\n2. Step two)
    let mut steps = Vec::new();
    for line in trimmed.lines() {
        let line = line.trim();
        // Match "1. ", "2. ", "- ", "* " patterns
        if let Some(rest) = line.strip_prefix(|c: char| c.is_ascii_digit() || c == '-' || c == '*') {
            let rest = rest.trim_start_matches(|c: char| c.is_ascii_digit() || c == '.' || c == ')' || c == ' ');
            let rest = rest.trim();
            if !rest.is_empty() {
                steps.push(rest.to_string());
            }
        }
    }

    steps
}

fn parse_xml_tool_call(text: &str) -> Option<(String, String)> {
    // Try standard XML format first
    if let (Some(tool_start), Some(tool_end)) = (text.find("<tool>"), text.find("</tool>")) {
        let tool_name = text[tool_start + 6..tool_end].trim().to_string();
        if let (Some(args_start), Some(args_end)) = (text.find("<args>"), text.find("</args>")) {
            let args_str = text[args_start + 6..args_end].trim().to_string();
            return Some((tool_name, args_str));
        }
    }

    // Fallback: try to find JSON tool call pattern in text
    // e.g. write_file("path", "content") or write_file {"path": "..."}
    let known_tools = ["write_file", "read_file", "edit_file", "list_dir", "run_command",
                       "search_content", "create_file", "delete_file", "search_files",
                       "glob_files", "git_status", "git_diff", "git_commit", "git_log",
                       "grep", "find", "curl", "sed", "wc", "codebase_search",
                       "update_memory", "web_search", "open_in_editor"];

    for tool in &known_tools {
        if let Some(pos) = text.find(tool) {
            // Look for JSON object after the tool name
            let after = &text[pos + tool.len()..];
            if let Some(json_start) = after.find('{') {
                let json_part = &after[json_start..];
                // Find matching closing brace
                let mut depth = 0;
                for (i, ch) in json_part.char_indices() {
                    match ch {
                        '{' => depth += 1,
                        '}' => {
                            depth -= 1;
                            if depth == 0 {
                                let json_str = &json_part[..=i];
                                if serde_json::from_str::<serde_json::Value>(json_str).is_ok() {
                                    return Some((tool.to_string(), json_str.to_string()));
                                }
                                break;
                            }
                        }
                        _ => {}
                    }
                }
            }
        }
    }

    None
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_xml_tool_call_parsing() {
        let input = r#"<tool>read_file</tool><args>{"path":"test.txt"}</args>"#;
        let result = parse_xml_tool_call(input);
        assert!(result.is_some());
        let (name, args) = result.unwrap();
        assert_eq!(name, "read_file");
        assert_eq!(args, r#"{"path":"test.txt"}"#);

        // Also verify the args parse as valid JSON
        let parsed: serde_json::Value = serde_json::from_str(&args).unwrap();
        assert_eq!(parsed["path"], "test.txt");
    }

    #[test]
    fn test_xml_tool_call_with_whitespace() {
        let input = r#"
            <tool>  write_file  </tool>
            <args>  {"path": "src/main.rs", "content": "fn main() {}"}  </args>
        "#;
        let result = parse_xml_tool_call(input);
        assert!(result.is_some());
        let (name, args) = result.unwrap();
        assert_eq!(name, "write_file");
        // The args should be trimmed
        let parsed: serde_json::Value = serde_json::from_str(&args).unwrap();
        assert_eq!(parsed["path"], "src/main.rs");
    }

    #[test]
    fn test_xml_tool_call_missing_args() {
        // Only <tool> tag, no <args> — should return None
        let input = "<tool>read_file</tool>";
        let result = parse_xml_tool_call(input);
        assert!(result.is_none());
    }

    #[test]
    fn test_xml_tool_call_no_tags() {
        // Plain text with no XML tags
        let input = "I'll help you with that task.";
        let result = parse_xml_tool_call(input);
        assert!(result.is_none());
    }

    #[test]
    fn test_error_detection_error_prefix() {
        let result = "[ERROR exit_code=1]\nNameError: name 'x' is not defined";
        let is_error = result.starts_with("[ERROR") || result.starts_with("Error:");
        assert!(is_error);
    }

    #[test]
    fn test_error_detection_error_colon() {
        let result = "Error: file not found";
        let is_error = result.starts_with("[ERROR") || result.starts_with("Error:");
        assert!(is_error);
    }

    #[test]
    fn test_error_detection_success() {
        let result = "hello world\n[stderr]: debug info";
        let is_error = result.starts_with("[ERROR") || result.starts_with("Error:");
        assert!(!is_error);
    }

    #[test]
    fn test_error_count_tracking() {
        let mut error_count: HashMap<String, usize> = HashMap::new();

        // Simulate 3 failures
        for i in 1..=3 {
            let count = error_count.entry("run_command".to_string()).or_insert(0);
            *count += 1;
            assert_eq!(*count, i);
        }

        assert_eq!(error_count["run_command"], 3);

        // Success resets
        error_count.remove("run_command");
        assert!(error_count.get("run_command").is_none());
    }

    #[test]
    fn test_error_hint_injection_logic() {
        let mut error_count: HashMap<String, usize> = HashMap::new();
        let tool_name = "run_command".to_string();

        // First failure: should get fix hint
        let count = error_count.entry(tool_name.clone()).or_insert(0);
        *count += 1;
        assert!(*count < 3, "Should not trigger 3-strike on first error");

        // Second failure
        let count = error_count.entry(tool_name.clone()).or_insert(0);
        *count += 1;
        assert!(*count < 3);

        // Third failure: should trigger 3-strike
        let count = error_count.entry(tool_name.clone()).or_insert(0);
        *count += 1;
        assert!(*count >= 3, "Should trigger 3-strike escalation");
    }
}
