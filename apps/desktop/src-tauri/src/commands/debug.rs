use std::sync::{Arc, Mutex};

use localcode_core::debug::session::{DebugSession, default_launch_config};
use tauri::{AppHandle, Emitter};

pub type DebugManager = Arc<Mutex<DebugSession>>;

pub fn create_debug_manager() -> DebugManager {
    Arc::new(Mutex::new(DebugSession::new()))
}

#[tauri::command]
pub fn debug_start(
    path: String,
    program: String,
    adapter_type: String,
    app: AppHandle,
    state: tauri::State<'_, DebugManager>,
) -> Result<(), String> {
    let config = default_launch_config(&adapter_type, &program)
        .ok_or_else(|| format!("Unsupported debug adapter type: {}", adapter_type))?;

    // Update cwd to project path
    let mut config = config;
    config.cwd = Some(path);

    let mut session = state.lock().map_err(|e| e.to_string())?;
    session.start(config).map_err(|e| e.to_string())?;

    // Spawn event reader in background
    let manager = state.inner().clone();
    let app_handle = app.clone();
    tokio::spawn(async move {
        loop {
            let msg = {
                let mut session = match manager.lock() {
                    Ok(s) => s,
                    Err(_) => break,
                };
                if !session.client.is_running() {
                    break;
                }
                match session.client.read_message() {
                    Ok(msg) => msg,
                    Err(_) => break,
                }
            };

            match msg.msg_type.as_str() {
                "event" => {
                    if let Some(event_name) = &msg.event {
                        match event_name.as_str() {
                            "stopped" => {
                                let _ = app_handle.emit("debug-stopped", &msg.body);
                            }
                            "output" => {
                                let _ = app_handle.emit("debug-output", &msg.body);
                            }
                            "terminated" | "exited" => {
                                let _ = app_handle.emit("debug-terminated", &msg.body);
                                break;
                            }
                            "initialized" => {
                                // Launch the debuggee after initialization
                                let mut session = match manager.lock() {
                                    Ok(s) => s,
                                    Err(_) => break,
                                };
                                let _ = session.launch_debuggee();
                            }
                            _ => {}
                        }
                    }
                }
                "response" => {
                    // Handle responses if needed
                }
                _ => {}
            }
        }
    });

    Ok(())
}

#[tauri::command]
pub fn debug_stop(state: tauri::State<'_, DebugManager>) -> Result<(), String> {
    let mut session = state.lock().map_err(|e| e.to_string())?;
    session.stop().map_err(|e| e.to_string())
}

#[tauri::command]
pub fn debug_continue(state: tauri::State<'_, DebugManager>) -> Result<(), String> {
    let mut session = state.lock().map_err(|e| e.to_string())?;
    session.continue_execution().map_err(|e| e.to_string())
}

#[tauri::command]
pub fn debug_step_over(state: tauri::State<'_, DebugManager>) -> Result<(), String> {
    let mut session = state.lock().map_err(|e| e.to_string())?;
    session.step_over().map_err(|e| e.to_string())
}

#[tauri::command]
pub fn debug_step_into(state: tauri::State<'_, DebugManager>) -> Result<(), String> {
    let mut session = state.lock().map_err(|e| e.to_string())?;
    session.step_into().map_err(|e| e.to_string())
}

#[tauri::command]
pub fn debug_step_out(state: tauri::State<'_, DebugManager>) -> Result<(), String> {
    let mut session = state.lock().map_err(|e| e.to_string())?;
    session.step_out().map_err(|e| e.to_string())
}

#[tauri::command]
pub fn debug_pause(state: tauri::State<'_, DebugManager>) -> Result<(), String> {
    let mut session = state.lock().map_err(|e| e.to_string())?;
    session.pause().map_err(|e| e.to_string())
}
