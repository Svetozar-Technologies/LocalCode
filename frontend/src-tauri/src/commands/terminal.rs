use portable_pty::{native_pty_system, CommandBuilder, MasterPty, PtySize};
use std::collections::HashMap;
use std::io::{Read, Write};
use std::sync::{Arc, Mutex};
use tauri::{AppHandle, Emitter};

pub struct TerminalState {
    sessions: HashMap<String, TerminalSession>,
}

struct TerminalSession {
    master: Box<dyn MasterPty + Send>,
    writer: Box<dyn Write + Send>,
}

impl TerminalState {
    pub fn new() -> Self {
        Self {
            sessions: HashMap::new(),
        }
    }
}

pub type TerminalManager = Arc<Mutex<TerminalState>>;

pub fn create_terminal_manager() -> TerminalManager {
    Arc::new(Mutex::new(TerminalState::new()))
}

#[tauri::command]
pub fn spawn_terminal(
    id: String,
    rows: u16,
    cols: u16,
    app: AppHandle,
    state: tauri::State<'_, TerminalManager>,
) -> Result<(), String> {
    let pty_system = native_pty_system();

    let pair = pty_system
        .openpty(PtySize {
            rows,
            cols,
            pixel_width: 0,
            pixel_height: 0,
        })
        .map_err(|e| e.to_string())?;

    let mut cmd = CommandBuilder::new_default_prog();
    // Set TERM for proper color support
    cmd.env("TERM", "xterm-256color");
    cmd.env("COLORTERM", "truecolor");

    let _child = pair.slave.spawn_command(cmd).map_err(|e| e.to_string())?;

    let writer = pair.master.take_writer().map_err(|e| e.to_string())?;

    // Spawn reader thread to forward PTY output to frontend
    let mut reader = pair.master.try_clone_reader().map_err(|e| e.to_string())?;
    let term_id = id.clone();
    let app_handle = app.clone();
    std::thread::spawn(move || {
        let mut buf = [0u8; 4096];
        loop {
            match reader.read(&mut buf) {
                Ok(0) => break,
                Ok(n) => {
                    let data = String::from_utf8_lossy(&buf[..n]).to_string();
                    let _ = app_handle.emit("terminal-output", data);
                }
                Err(_) => break,
            }
        }
        log::info!("Terminal {} reader thread exited", term_id);
    });

    let session = TerminalSession {
        master: pair.master,
        writer,
    };

    state
        .lock()
        .map_err(|e| e.to_string())?
        .sessions
        .insert(id, session);

    Ok(())
}

#[tauri::command]
pub fn write_terminal(
    id: String,
    data: String,
    state: tauri::State<'_, TerminalManager>,
) -> Result<(), String> {
    let mut manager = state.lock().map_err(|e| e.to_string())?;
    if let Some(session) = manager.sessions.get_mut(&id) {
        session
            .writer
            .write_all(data.as_bytes())
            .map_err(|e| e.to_string())?;
        session.writer.flush().map_err(|e| e.to_string())?;
    }
    Ok(())
}

#[tauri::command]
pub fn resize_terminal(
    id: String,
    rows: u16,
    cols: u16,
    state: tauri::State<'_, TerminalManager>,
) -> Result<(), String> {
    let manager = state.lock().map_err(|e| e.to_string())?;
    if let Some(session) = manager.sessions.get(&id) {
        session
            .master
            .resize(PtySize {
                rows,
                cols,
                pixel_width: 0,
                pixel_height: 0,
            })
            .map_err(|e| e.to_string())?;
    }
    Ok(())
}

#[tauri::command]
pub fn kill_terminal(
    id: String,
    state: tauri::State<'_, TerminalManager>,
) -> Result<(), String> {
    let mut manager = state.lock().map_err(|e| e.to_string())?;
    manager.sessions.remove(&id);
    Ok(())
}
