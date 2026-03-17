use serde::{Deserialize, Serialize};
use std::fs;
use std::process::Command;
use tauri::{AppHandle, Emitter};

use super::llm::LLMManager;

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct AgentStep {
    #[serde(rename = "type")]
    pub step_type: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub tool: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub args: Option<serde_json::Value>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub result: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub content: Option<String>,
    pub timestamp: u64,
}

fn now_ms() -> u64 {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap_or_default()
        .as_millis() as u64
}

fn emit_step(app: &AppHandle, response_id: &str, step: &AgentStep) {
    let _ = app.emit(
        "agent-step",
        serde_json::json!({
            "id": response_id,
            "step": step,
        }),
    );
}

#[tauri::command]
pub async fn agent_execute(
    response_id: String,
    task: String,
    project_path: String,
    current_file: String,
    current_file_content: String,
    app: AppHandle,
    state: tauri::State<'_, LLMManager>,
) -> Result<(), String> {
    let (url, client) = {
        let llm = state.lock().map_err(|e| e.to_string())?;
        (llm.server_url.clone(), llm.client.clone())
    };

    // Agent system prompt with tool definitions
    let system_prompt = format!(
        r#"You are LocalCode Agent, an autonomous AI coding assistant.
You can use tools to accomplish tasks. Available tools:

1. read_file(path) - Read a file's contents
2. write_file(path, content) - Write content to a file
3. edit_file(path, old_text, new_text) - Replace text in a file
4. run_command(command) - Run a shell command and get output
5. search_files(pattern) - Search for files by name pattern
6. search_content(pattern) - Search file contents for a pattern
7. list_dir(path) - List directory contents
8. done(summary) - Task complete, provide summary

To use a tool, respond with EXACTLY this format:
<tool>tool_name</tool>
<args>{{"param": "value"}}</args>

Project path: {project_path}
Current file: {current_file}

Analyze the task, plan your approach, then use tools step by step.
After each tool result, decide the next action. Call done() when finished."#
    );

    let mut conversation = vec![
        serde_json::json!({"role": "system", "content": system_prompt}),
    ];

    // Add file context if available
    if !current_file_content.is_empty() {
        conversation.push(serde_json::json!({
            "role": "system",
            "content": format!("Current file content ({}):\n```\n{}\n```",
                current_file,
                current_file_content.chars().take(4000).collect::<String>()
            )
        }));
    }

    conversation.push(serde_json::json!({"role": "user", "content": task}));

    // Agent loop - max 15 iterations
    for _iteration in 0..15 {
        // Call LLM
        let body = serde_json::json!({
            "messages": conversation,
            "temperature": 0.3,
            "max_tokens": 2048,
            "stream": false,
        });

        let response = client
            .post(format!("{}/v1/chat/completions", url))
            .json(&body)
            .send()
            .await
            .map_err(|e| format!("Agent LLM call failed: {}", e))?;

        let result: serde_json::Value = response.json().await.map_err(|e| e.to_string())?;
        let assistant_msg = result["choices"][0]["message"]["content"]
            .as_str()
            .unwrap_or("")
            .to_string();

        // Parse tool call from response
        if let Some((tool_name, args_str)) = parse_tool_call(&assistant_msg) {
            let args: serde_json::Value = serde_json::from_str(&args_str)
                .unwrap_or(serde_json::json!({}));

            // Emit the tool call step
            emit_step(&app, &response_id, &AgentStep {
                step_type: "tool_call".to_string(),
                tool: Some(tool_name.clone()),
                args: Some(args.clone()),
                result: None,
                content: None,
                timestamp: now_ms(),
            });

            // Execute the tool
            let tool_result = execute_tool(&tool_name, &args, &project_path);

            // Emit the result
            emit_step(&app, &response_id, &AgentStep {
                step_type: "tool_result".to_string(),
                tool: Some(tool_name.clone()),
                args: None,
                result: Some(tool_result.clone()),
                content: None,
                timestamp: now_ms(),
            });

            // Check if done
            if tool_name == "done" {
                let _ = app.emit(
                    "llm-chat-chunk",
                    serde_json::json!({"id": response_id, "chunk": tool_result}),
                );
                let _ = app.emit("llm-chat-done", serde_json::json!({"id": response_id}));
                return Ok(());
            }

            // Add to conversation for next iteration
            conversation.push(serde_json::json!({"role": "assistant", "content": assistant_msg}));
            conversation.push(serde_json::json!({
                "role": "user",
                "content": format!("Tool result:\n{}", tool_result)
            }));
        } else {
            // No tool call - just a response
            let _ = app.emit(
                "llm-chat-chunk",
                serde_json::json!({"id": response_id, "chunk": assistant_msg}),
            );
            let _ = app.emit("llm-chat-done", serde_json::json!({"id": response_id}));
            return Ok(());
        }
    }

    let _ = app.emit(
        "llm-chat-chunk",
        serde_json::json!({"id": response_id, "chunk": "\n\n[Agent reached max iterations]"}),
    );
    let _ = app.emit("llm-chat-done", serde_json::json!({"id": response_id}));
    Ok(())
}

fn parse_tool_call(text: &str) -> Option<(String, String)> {
    let tool_start = text.find("<tool>")?;
    let tool_end = text.find("</tool>")?;
    let tool_name = text[tool_start + 6..tool_end].trim().to_string();

    let args_start = text.find("<args>")?;
    let args_end = text.find("</args>")?;
    let args_str = text[args_start + 6..args_end].trim().to_string();

    Some((tool_name, args_str))
}

fn execute_tool(
    tool_name: &str,
    args: &serde_json::Value,
    project_path: &str,
) -> String {
    match tool_name {
        "read_file" => {
            let path = args["path"].as_str().unwrap_or("");
            let full_path = resolve_path(path, project_path);
            match fs::read_to_string(&full_path) {
                Ok(content) => content.chars().take(8000).collect(),
                Err(e) => format!("Error reading {}: {}", full_path, e),
            }
        }
        "write_file" => {
            let path = args["path"].as_str().unwrap_or("");
            let content = args["content"].as_str().unwrap_or("");
            let full_path = resolve_path(path, project_path);
            match fs::write(&full_path, content) {
                Ok(_) => format!("Successfully wrote to {}", full_path),
                Err(e) => format!("Error writing {}: {}", full_path, e),
            }
        }
        "edit_file" => {
            let path = args["path"].as_str().unwrap_or("");
            let old_text = args["old_text"].as_str().unwrap_or("");
            let new_text = args["new_text"].as_str().unwrap_or("");
            let full_path = resolve_path(path, project_path);
            match fs::read_to_string(&full_path) {
                Ok(content) => {
                    if content.contains(old_text) {
                        let updated = content.replace(old_text, new_text);
                        match fs::write(&full_path, &updated) {
                            Ok(_) => format!("Successfully edited {}", full_path),
                            Err(e) => format!("Error writing {}: {}", full_path, e),
                        }
                    } else {
                        format!("Text not found in {}", full_path)
                    }
                }
                Err(e) => format!("Error reading {}: {}", full_path, e),
            }
        }
        "run_command" => {
            let command = args["command"].as_str().unwrap_or("");
            match Command::new("sh")
                .arg("-c")
                .arg(command)
                .current_dir(project_path)
                .output()
            {
                Ok(output) => {
                    let stdout = String::from_utf8_lossy(&output.stdout);
                    let stderr = String::from_utf8_lossy(&output.stderr);
                    let mut result = String::new();
                    if !stdout.is_empty() {
                        result.push_str(&stdout);
                    }
                    if !stderr.is_empty() {
                        result.push_str("\nSTDERR: ");
                        result.push_str(&stderr);
                    }
                    if result.is_empty() {
                        format!("Command completed with exit code: {}", output.status.code().unwrap_or(-1))
                    } else {
                        result.chars().take(4000).collect()
                    }
                }
                Err(e) => format!("Error running command: {}", e),
            }
        }
        "search_files" => {
            let pattern = args["pattern"].as_str().unwrap_or("");
            let pattern_lower = pattern.to_lowercase();
            let mut results = Vec::new();

            let walker = ignore::WalkBuilder::new(project_path)
                .hidden(true)
                .git_ignore(true)
                .build();

            for entry in walker.into_iter().filter_map(|e| e.ok()) {
                if let Some(name) = entry.file_name().to_str() {
                    if name.to_lowercase().contains(&pattern_lower) {
                        results.push(entry.path().to_string_lossy().to_string());
                    }
                }
                if results.len() >= 20 {
                    break;
                }
            }

            if results.is_empty() {
                "No files found".to_string()
            } else {
                results.join("\n")
            }
        }
        "search_content" => {
            let pattern = args["pattern"].as_str().unwrap_or("");
            let pattern_lower = pattern.to_lowercase();
            let mut results = Vec::new();

            let walker = ignore::WalkBuilder::new(project_path)
                .hidden(true)
                .git_ignore(true)
                .build();

            for entry in walker.into_iter().filter_map(|e| e.ok()) {
                if !entry.path().is_file() {
                    continue;
                }
                if let Ok(content) = fs::read_to_string(entry.path()) {
                    for (i, line) in content.lines().enumerate() {
                        if line.to_lowercase().contains(&pattern_lower) {
                            results.push(format!(
                                "{}:{}: {}",
                                entry.path().to_string_lossy(),
                                i + 1,
                                line.trim()
                            ));
                        }
                    }
                }
                if results.len() >= 30 {
                    break;
                }
            }

            if results.is_empty() {
                "No matches found".to_string()
            } else {
                results.join("\n")
            }
        }
        "list_dir" => {
            let path = args["path"].as_str().unwrap_or(".");
            let full_path = resolve_path(path, project_path);
            match fs::read_dir(&full_path) {
                Ok(entries) => {
                    let mut items: Vec<String> = entries
                        .filter_map(|e| e.ok())
                        .map(|e| {
                            let ft = if e.path().is_dir() { "dir" } else { "file" };
                            format!("[{}] {}", ft, e.file_name().to_string_lossy())
                        })
                        .collect();
                    items.sort();
                    items.join("\n")
                }
                Err(e) => format!("Error listing {}: {}", full_path, e),
            }
        }
        "done" => {
            args["summary"].as_str().unwrap_or("Task completed").to_string()
        }
        _ => format!("Unknown tool: {}", tool_name),
    }
}

fn resolve_path(path: &str, project_path: &str) -> String {
    if path.starts_with('/') {
        path.to_string()
    } else {
        format!("{}/{}", project_path, path)
    }
}
