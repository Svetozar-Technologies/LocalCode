use serde::{Deserialize, Serialize};
use std::collections::HashMap;

use super::breakpoints::BreakpointManager;
use super::dap::{DapClient, StackFrame, Variable, Scope};
use crate::CoreResult;

/// Debug session state
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum SessionState {
    Idle,
    Initializing,
    Running,
    Stopped,
    Terminated,
}

/// Debug launch configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LaunchConfig {
    /// Type of debug adapter (e.g., "python", "node", "lldb")
    pub adapter_type: String,
    /// Command to launch the debug adapter
    pub adapter_command: String,
    /// Args for the debug adapter
    #[serde(default)]
    pub adapter_args: Vec<String>,
    /// Program to debug
    pub program: String,
    /// Program arguments
    #[serde(default)]
    pub args: Vec<String>,
    /// Working directory
    #[serde(default)]
    pub cwd: Option<String>,
    /// Environment variables
    #[serde(default)]
    pub env: HashMap<String, String>,
    /// Whether to stop at entry point
    #[serde(default)]
    pub stop_on_entry: bool,
}

/// Known debug adapter configurations
pub fn default_launch_config(adapter_type: &str, program: &str) -> Option<LaunchConfig> {
    match adapter_type {
        "python" | "debugpy" => Some(LaunchConfig {
            adapter_type: "python".to_string(),
            adapter_command: "python".to_string(),
            adapter_args: vec![
                "-m".to_string(),
                "debugpy.adapter".to_string(),
            ],
            program: program.to_string(),
            args: Vec::new(),
            cwd: None,
            env: HashMap::new(),
            stop_on_entry: false,
        }),
        "node" | "javascript" | "typescript" => Some(LaunchConfig {
            adapter_type: "node".to_string(),
            adapter_command: "node".to_string(),
            adapter_args: vec![
                "--inspect-brk".to_string(),
            ],
            program: program.to_string(),
            args: Vec::new(),
            cwd: None,
            env: HashMap::new(),
            stop_on_entry: true,
        }),
        "lldb" | "rust" | "c" | "cpp" => Some(LaunchConfig {
            adapter_type: "lldb".to_string(),
            adapter_command: "lldb-vscode".to_string(),
            adapter_args: Vec::new(),
            program: program.to_string(),
            args: Vec::new(),
            cwd: None,
            env: HashMap::new(),
            stop_on_entry: false,
        }),
        _ => None,
    }
}

/// A debug session managing the full lifecycle
pub struct DebugSession {
    pub state: SessionState,
    pub config: Option<LaunchConfig>,
    pub client: DapClient,
    pub breakpoints: BreakpointManager,
    pub current_thread_id: Option<u64>,
    pub stack_frames: Vec<StackFrame>,
    pub scopes: Vec<Scope>,
    pub variables: Vec<Variable>,
    pub watch_expressions: Vec<WatchExpression>,
    pub output: Vec<OutputLine>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WatchExpression {
    pub id: u64,
    pub expression: String,
    pub value: Option<String>,
    pub var_type: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OutputLine {
    pub category: String,
    pub text: String,
    pub source: Option<String>,
    pub line: Option<u64>,
}

impl DebugSession {
    pub fn new() -> Self {
        Self {
            state: SessionState::Idle,
            config: None,
            client: DapClient::new(),
            breakpoints: BreakpointManager::new(),
            current_thread_id: None,
            stack_frames: Vec::new(),
            scopes: Vec::new(),
            variables: Vec::new(),
            watch_expressions: Vec::new(),
            output: Vec::new(),
        }
    }

    /// Start a debug session with the given configuration
    pub fn start(&mut self, config: LaunchConfig) -> CoreResult<()> {
        self.state = SessionState::Initializing;
        self.config = Some(config.clone());
        self.output.clear();
        self.stack_frames.clear();

        // Launch the debug adapter
        self.client
            .launch_adapter(&config.adapter_command, &config.adapter_args)?;

        // Send initialize
        self.client.initialize()?;

        self.state = SessionState::Initializing;
        Ok(())
    }

    /// Launch the debuggee after initialization
    pub fn launch_debuggee(&mut self) -> CoreResult<()> {
        let config = self
            .config
            .as_ref()
            .ok_or_else(|| crate::CoreError::Other("No launch configuration".to_string()))?
            .clone();

        self.client.launch(
            &config.program,
            &config.args,
            config.cwd.as_deref(),
            if config.env.is_empty() {
                None
            } else {
                Some(&config.env)
            },
        )?;

        // Set breakpoints for all files
        for file in self.breakpoints.files_with_breakpoints() {
            let lines = self.breakpoints.enabled_lines(file);
            self.client.set_breakpoints(file, &lines)?;
        }

        // Configuration done
        self.client.configuration_done()?;
        self.state = SessionState::Running;

        Ok(())
    }

    /// Continue execution
    pub fn continue_execution(&mut self) -> CoreResult<()> {
        if let Some(thread_id) = self.current_thread_id {
            self.client.continue_execution(thread_id)?;
            self.state = SessionState::Running;
        }
        Ok(())
    }

    /// Step over
    pub fn step_over(&mut self) -> CoreResult<()> {
        if let Some(thread_id) = self.current_thread_id {
            self.client.step_over(thread_id)?;
            self.state = SessionState::Running;
        }
        Ok(())
    }

    /// Step into
    pub fn step_into(&mut self) -> CoreResult<()> {
        if let Some(thread_id) = self.current_thread_id {
            self.client.step_into(thread_id)?;
            self.state = SessionState::Running;
        }
        Ok(())
    }

    /// Step out
    pub fn step_out(&mut self) -> CoreResult<()> {
        if let Some(thread_id) = self.current_thread_id {
            self.client.step_out(thread_id)?;
            self.state = SessionState::Running;
        }
        Ok(())
    }

    /// Pause execution
    pub fn pause(&mut self) -> CoreResult<()> {
        if let Some(thread_id) = self.current_thread_id {
            self.client.pause(thread_id)?;
        }
        Ok(())
    }

    /// Stop the debug session
    pub fn stop(&mut self) -> CoreResult<()> {
        self.client.disconnect(true)?;
        self.state = SessionState::Terminated;
        Ok(())
    }

    /// Add a watch expression
    pub fn add_watch(&mut self, expression: &str) -> u64 {
        let id = self.watch_expressions.len() as u64;
        self.watch_expressions.push(WatchExpression {
            id,
            expression: expression.to_string(),
            value: None,
            var_type: None,
        });
        id
    }

    /// Remove a watch expression
    pub fn remove_watch(&mut self, id: u64) {
        self.watch_expressions.retain(|w| w.id != id);
    }

    /// Evaluate an expression in the current context
    pub fn evaluate(&mut self, expression: &str) -> CoreResult<()> {
        let frame_id = self.stack_frames.first().map(|f| f.id);
        self.client.evaluate(expression, frame_id, "watch")
    }

    /// Toggle breakpoint at a line
    pub fn toggle_breakpoint(&mut self, file: &str, line: u64) -> CoreResult<bool> {
        let added = self.breakpoints.toggle_breakpoint(file, line);

        // If we're in a debug session, update the adapter
        if self.client.is_running() {
            let lines = self.breakpoints.enabled_lines(file);
            self.client.set_breakpoints(file, &lines)?;
        }

        Ok(added)
    }

    pub fn is_active(&self) -> bool {
        matches!(
            self.state,
            SessionState::Initializing | SessionState::Running | SessionState::Stopped
        )
    }
}

impl Default for DebugSession {
    fn default() -> Self {
        Self::new()
    }
}
