use std::io::{self, BufRead, Write};
use std::sync::Arc;

use localcode_core::agent::{AgentEngine, AgentEvent, ToolContext, ToolRegistry};
use localcode_core::agent::builtin;
use localcode_core::agent::memory::MemoryManager;
use localcode_core::config::Config;
use localcode_core::llm::provider::*;

use crate::conversation::Conversation;
use crate::rendering;
use crate::slash::{self, SlashCommand};
use crate::streaming;

pub async fn run_repl() -> Result<(), Box<dyn std::error::Error>> {
    rendering::print_header();

    let config = Config::load().unwrap_or_default();
    let provider = create_provider(&config)?;
    let cwd = std::env::current_dir()?.display().to_string();
    let memory = MemoryManager::new();

    let mut conversation = Conversation::new(&cwd);
    let mut agent_mode = false;

    let stdin = io::stdin();

    loop {
        if agent_mode {
            print!("{} ", crossterm::style::Stylize::yellow("agent>"));
        } else {
            rendering::print_prompt();
        }
        io::stdout().flush()?;

        let mut input = String::new();
        if stdin.lock().read_line(&mut input)? == 0 {
            break; // EOF
        }

        let input = input.trim().to_string();
        if input.is_empty() {
            continue;
        }

        // Check slash commands
        if let Some(cmd) = slash::parse_slash(&input) {
            match cmd {
                SlashCommand::Help => {
                    slash::print_help();
                    continue;
                }
                SlashCommand::Clear => {
                    conversation = Conversation::new(&cwd);
                    rendering::print_info("Conversation cleared");
                    continue;
                }
                SlashCommand::Exit => {
                    let _ = conversation.save();
                    break;
                }
                SlashCommand::Config => {
                    println!("{}", serde_json::to_string_pretty(&config)?);
                    continue;
                }
                SlashCommand::Memory => {
                    let ctx = memory.build_context(&cwd);
                    if ctx.is_empty() {
                        rendering::print_info("No memory for this project");
                    } else {
                        println!("{}", ctx);
                    }
                    continue;
                }
                SlashCommand::Model(name) => {
                    if let Some(name) = name {
                        rendering::print_info(&format!("Model switching to: {}", name));
                    } else {
                        rendering::print_info(&format!("Current provider: {}", config.default_provider));
                    }
                    continue;
                }
                SlashCommand::Commit => {
                    run_commit(&provider, &cwd).await;
                    continue;
                }
                SlashCommand::Unknown(cmd) => {
                    rendering::print_error(&format!("Unknown command: {}", cmd));
                    continue;
                }
            }
        }

        // Toggle agent mode
        if input == "/agent" || input == "/a" {
            agent_mode = !agent_mode;
            rendering::print_info(&format!(
                "Agent mode: {}",
                if agent_mode { "ON" } else { "OFF" }
            ));
            continue;
        }

        if agent_mode {
            // Run as agent
            let mut registry = ToolRegistry::new();
            builtin::register_all(&mut registry);

            let mut engine = AgentEngine::new(provider.clone(), registry);
            engine.initialize(&cwd);

            let ctx = ToolContext {
                project_path: cwd.clone(),
                current_file: None,
            };

            rendering::print_assistant_prefix();

            let result = engine
                .execute(&input, &ctx, &|event| match event {
                    AgentEvent::Step(step) => {
                        if step.step_type == "tool_call" {
                            if let Some(ref tool) = step.tool {
                                rendering::print_tool_call(
                                    tool,
                                    step.args.as_ref().unwrap_or(&serde_json::json!({})),
                                );
                            }
                        } else if step.step_type == "tool_result" {
                            if let (Some(ref tool), Some(ref result)) = (&step.tool, &step.result) {
                                rendering::print_tool_result(tool, result);
                            }
                        }
                    }
                    AgentEvent::TextChunk(text) => {
                        print!("{}", text);
                        io::stdout().flush().unwrap();
                    }
                    AgentEvent::Done(text) => {
                        println!("\n");
                        rendering::print_markdown(&text);
                    }
                    AgentEvent::Error(e) => {
                        rendering::print_error(&e);
                    }
                })
                .await;

            if let Err(e) = result {
                rendering::print_error(&format!("{}", e));
            }
            println!();
        } else {
            // Run as chat
            conversation.add_message("user", &input);

            let opts = ChatOptions {
                temperature: 0.7,
                max_tokens: 4096,
                stream: true,
                system: Some("You are LocalCode AI, a helpful coding assistant. Be concise and practical.".to_string()),
                ..Default::default()
            };

            rendering::print_assistant_prefix();

            match provider.chat(conversation.messages.clone(), opts).await {
                Ok(stream) => {
                    match streaming::stream_to_stdout(stream).await {
                        Ok(text) => {
                            conversation.add_message("assistant", &text);
                        }
                        Err(e) => {
                            rendering::print_error(&format!("{}", e));
                        }
                    }
                }
                Err(e) => {
                    rendering::print_error(&format!("{}", e));
                }
            }
            println!();
        }
    }

    Ok(())
}

fn create_provider(config: &Config) -> Result<Arc<dyn LLMProvider>, Box<dyn std::error::Error>> {
    use localcode_core::llm::local::LocalProvider;
    use localcode_core::llm::openai::OpenAIProvider;
    use localcode_core::llm::anthropic::AnthropicProvider;

    let provider: Arc<dyn LLMProvider> = match config.default_provider.as_str() {
        "openai" => {
            let key = config.get_openai_key();
            if key.is_empty() {
                return Err("OpenAI API key not set. Run: localcode config --set openai.api_key=YOUR_KEY".into());
            }
            let model = config.get_openai_model();
            Arc::new(OpenAIProvider::new(&key, &model))
        }
        "anthropic" => {
            let key = config.get_anthropic_key();
            if key.is_empty() {
                return Err("Anthropic API key not set. Run: localcode config --set anthropic.api_key=YOUR_KEY".into());
            }
            let model = config.get_anthropic_model();
            Arc::new(AnthropicProvider::new(&key, &model))
        }
        _ => {
            Arc::new(LocalProvider::new())
        }
    };

    Ok(provider)
}

async fn run_commit(provider: &Arc<dyn LLMProvider>, cwd: &str) {
    match localcode_core::git::git_diff(cwd) {
        Ok(diff) => {
            if diff.is_empty() {
                rendering::print_info("No changes to commit");
                return;
            }

            rendering::print_info("Generating commit message...");

            let messages = vec![ChatMessage {
                role: "user".to_string(),
                content: format!(
                    "Generate a concise git commit message for these changes. \
                     Only output the commit message, nothing else:\n\n{}",
                    diff.chars().take(3000).collect::<String>()
                ),
                tool_calls: None,
                tool_call_id: None,
            }];

            let opts = ChatOptions {
                temperature: 0.3,
                max_tokens: 200,
                stream: false,
                ..Default::default()
            };

            match provider.chat_sync(messages, opts).await {
                Ok(msg) => {
                    let commit_msg = msg.content.trim().to_string();
                    rendering::print_info(&format!("Commit: {}", commit_msg));

                    match localcode_core::git::staging::git_add_all(cwd) {
                        Ok(_) => {
                            match localcode_core::git::staging::git_commit(cwd, &commit_msg) {
                                Ok(hash) => {
                                    rendering::print_success(&format!("Committed: {} ({})", commit_msg, hash));
                                }
                                Err(e) => rendering::print_error(&format!("Commit failed: {}", e)),
                            }
                        }
                        Err(e) => rendering::print_error(&format!("Stage failed: {}", e)),
                    }
                }
                Err(e) => rendering::print_error(&format!("Failed to generate message: {}", e)),
            }
        }
        Err(e) => rendering::print_error(&format!("Git diff failed: {}", e)),
    }
}
