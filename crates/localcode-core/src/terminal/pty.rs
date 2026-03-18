use portable_pty::{native_pty_system, CommandBuilder, MasterPty, PtySize};
use std::collections::HashMap;
use std::io::{Read, Write};
use std::sync::{Arc, Mutex};

use crate::CoreResult;

pub struct TerminalSession {
    pub master: Box<dyn MasterPty + Send>,
    pub writer: Box<dyn Write + Send>,
}

pub struct TerminalState {
    pub sessions: HashMap<String, TerminalSession>,
}

impl TerminalState {
    pub fn new() -> Self {
        Self {
            sessions: HashMap::new(),
        }
    }
}

impl Default for TerminalState {
    fn default() -> Self {
        Self::new()
    }
}

pub type TerminalManager = Arc<Mutex<TerminalState>>;

pub fn create_terminal_manager() -> TerminalManager {
    Arc::new(Mutex::new(TerminalState::new()))
}

pub fn spawn_terminal(
    id: &str,
    rows: u16,
    cols: u16,
    manager: &TerminalManager,
    on_output: Box<dyn Fn(String) + Send + 'static>,
) -> CoreResult<()> {
    let pty_system = native_pty_system();

    let pair = pty_system
        .openpty(PtySize {
            rows,
            cols,
            pixel_width: 0,
            pixel_height: 0,
        })
        .map_err(|e| crate::CoreError::Other(e.to_string()))?;

    let mut cmd = CommandBuilder::new_default_prog();
    cmd.env("TERM", "xterm-256color");
    cmd.env("COLORTERM", "truecolor");

    let _child = pair
        .slave
        .spawn_command(cmd)
        .map_err(|e| crate::CoreError::Other(e.to_string()))?;

    let writer = pair
        .master
        .take_writer()
        .map_err(|e| crate::CoreError::Other(e.to_string()))?;

    let mut reader = pair
        .master
        .try_clone_reader()
        .map_err(|e| crate::CoreError::Other(e.to_string()))?;

    let term_id = id.to_string();
    std::thread::spawn(move || {
        let mut buf = [0u8; 4096];
        loop {
            match reader.read(&mut buf) {
                Ok(0) => break,
                Ok(n) => {
                    let data = String::from_utf8_lossy(&buf[..n]).to_string();
                    on_output(data);
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

    manager
        .lock()
        .map_err(|e| crate::CoreError::Other(e.to_string()))?
        .sessions
        .insert(id.to_string(), session);

    Ok(())
}

pub fn write_terminal(id: &str, data: &str, manager: &TerminalManager) -> CoreResult<()> {
    let mut state = manager
        .lock()
        .map_err(|e| crate::CoreError::Other(e.to_string()))?;
    if let Some(session) = state.sessions.get_mut(id) {
        session.writer.write_all(data.as_bytes())?;
        session.writer.flush()?;
    }
    Ok(())
}

pub fn resize_terminal(id: &str, rows: u16, cols: u16, manager: &TerminalManager) -> CoreResult<()> {
    let state = manager
        .lock()
        .map_err(|e| crate::CoreError::Other(e.to_string()))?;
    if let Some(session) = state.sessions.get(id) {
        session
            .master
            .resize(PtySize {
                rows,
                cols,
                pixel_width: 0,
                pixel_height: 0,
            })
            .map_err(|e| crate::CoreError::Other(e.to_string()))?;
    }
    Ok(())
}

pub fn kill_terminal(id: &str, manager: &TerminalManager) -> CoreResult<()> {
    let mut state = manager
        .lock()
        .map_err(|e| crate::CoreError::Other(e.to_string()))?;
    state.sessions.remove(id);
    Ok(())
}
