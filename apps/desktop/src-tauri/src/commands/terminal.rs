use localcode_core::terminal;
use tauri::{AppHandle, Emitter};

pub type TerminalManager = terminal::TerminalManager;

pub fn create_terminal_manager() -> TerminalManager {
    terminal::create_terminal_manager()
}

#[tauri::command]
pub fn spawn_terminal(
    id: String,
    rows: u16,
    cols: u16,
    app: AppHandle,
    state: tauri::State<'_, TerminalManager>,
) -> Result<(), String> {
    let app_handle = app.clone();
    terminal::spawn_terminal(
        &id,
        rows,
        cols,
        &state,
        Box::new(move |data| {
            let _ = app_handle.emit("terminal-output", data);
        }),
    )
    .map_err(|e| e.to_string())
}

#[tauri::command]
pub fn write_terminal(
    id: String,
    data: String,
    state: tauri::State<'_, TerminalManager>,
) -> Result<(), String> {
    terminal::write_terminal(&id, &data, &state).map_err(|e| e.to_string())
}

#[tauri::command]
pub fn resize_terminal(
    id: String,
    rows: u16,
    cols: u16,
    state: tauri::State<'_, TerminalManager>,
) -> Result<(), String> {
    terminal::resize_terminal(&id, rows, cols, &state).map_err(|e| e.to_string())
}

#[tauri::command]
pub fn kill_terminal(
    id: String,
    state: tauri::State<'_, TerminalManager>,
) -> Result<(), String> {
    terminal::kill_terminal(&id, &state).map_err(|e| e.to_string())
}
