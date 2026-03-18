use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::io::{BufRead, BufReader, Write};
use std::process::{Child, Command, Stdio};
use std::sync::atomic::{AtomicU64, Ordering};

use crate::CoreResult;

/// DAP (Debug Adapter Protocol) client for communicating with debug adapters
pub struct DapClient {
    process: Option<Child>,
    seq: AtomicU64,
    capabilities: ServerCapabilities,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct ServerCapabilities {
    #[serde(default)]
    pub supports_configuration_done_request: bool,
    #[serde(default)]
    pub supports_function_breakpoints: bool,
    #[serde(default)]
    pub supports_conditional_breakpoints: bool,
    #[serde(default)]
    pub supports_hit_conditional_breakpoints: bool,
    #[serde(default)]
    pub supports_evaluate_for_hovers: bool,
    #[serde(default)]
    pub supports_step_back: bool,
    #[serde(default)]
    pub supports_set_variable: bool,
    #[serde(default)]
    pub supports_restart_frame: bool,
    #[serde(default)]
    pub supports_goto_targets_request: bool,
    #[serde(default)]
    pub supports_step_in_targets_request: bool,
    #[serde(default)]
    pub supports_completions_request: bool,
    #[serde(default)]
    pub supports_terminate_request: bool,
}

/// DAP protocol message
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DapMessage {
    pub seq: u64,
    #[serde(rename = "type")]
    pub msg_type: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub command: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub event: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub body: Option<serde_json::Value>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub request_seq: Option<u64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub success: Option<bool>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub message: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub arguments: Option<serde_json::Value>,
}

/// Events emitted by the debug adapter
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "event")]
pub enum DapEvent {
    Initialized,
    Stopped {
        reason: String,
        thread_id: Option<u64>,
        description: Option<String>,
    },
    Continued {
        thread_id: u64,
    },
    Exited {
        exit_code: i64,
    },
    Terminated,
    Thread {
        reason: String,
        thread_id: u64,
    },
    Output {
        category: Option<String>,
        output: String,
        source: Option<DapSource>,
        line: Option<u64>,
    },
    Breakpoint {
        reason: String,
        breakpoint: DapBreakpoint,
    },
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DapSource {
    pub name: Option<String>,
    pub path: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DapBreakpoint {
    pub id: Option<u64>,
    pub verified: bool,
    pub line: Option<u64>,
    pub source: Option<DapSource>,
    pub message: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StackFrame {
    pub id: u64,
    pub name: String,
    pub source: Option<DapSource>,
    pub line: u64,
    pub column: u64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Variable {
    pub name: String,
    pub value: String,
    #[serde(rename = "type")]
    pub var_type: Option<String>,
    pub variables_reference: u64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Scope {
    pub name: String,
    pub variables_reference: u64,
    pub expensive: bool,
}

impl DapClient {
    pub fn new() -> Self {
        Self {
            process: None,
            seq: AtomicU64::new(1),
            capabilities: ServerCapabilities::default(),
        }
    }

    fn next_seq(&self) -> u64 {
        self.seq.fetch_add(1, Ordering::SeqCst)
    }

    /// Launch a debug adapter process
    pub fn launch_adapter(&mut self, command: &str, args: &[String]) -> CoreResult<()> {
        let child = Command::new(command)
            .args(args)
            .stdin(Stdio::piped())
            .stdout(Stdio::piped())
            .stderr(Stdio::piped())
            .spawn()
            .map_err(|e| {
                crate::CoreError::Other(format!("Failed to launch debug adapter: {}", e))
            })?;

        self.process = Some(child);
        Ok(())
    }

    /// Send a DAP request
    pub fn send_request(
        &mut self,
        command: &str,
        arguments: Option<serde_json::Value>,
    ) -> CoreResult<()> {
        let msg = DapMessage {
            seq: self.next_seq(),
            msg_type: "request".to_string(),
            command: Some(command.to_string()),
            event: None,
            body: None,
            request_seq: None,
            success: None,
            message: None,
            arguments,
        };

        let json = serde_json::to_string(&msg)?;
        let content = format!("Content-Length: {}\r\n\r\n{}", json.len(), json);

        if let Some(ref mut child) = self.process {
            if let Some(ref mut stdin) = child.stdin {
                stdin
                    .write_all(content.as_bytes())
                    .map_err(|e| crate::CoreError::Other(format!("DAP write error: {}", e)))?;
                stdin
                    .flush()
                    .map_err(|e| crate::CoreError::Other(format!("DAP flush error: {}", e)))?;
            }
        }

        Ok(())
    }

    /// Read a DAP message from the adapter
    pub fn read_message(&mut self) -> CoreResult<DapMessage> {
        let child = self
            .process
            .as_mut()
            .ok_or_else(|| crate::CoreError::Other("No debug adapter running".to_string()))?;

        let stdout = child.stdout.as_mut().ok_or_else(|| {
            crate::CoreError::Other("No stdout from debug adapter".to_string())
        })?;

        let mut reader = BufReader::new(stdout);
        let mut content_length: usize = 0;

        // Read headers
        loop {
            let mut line = String::new();
            reader
                .read_line(&mut line)
                .map_err(|e| crate::CoreError::Other(format!("DAP read error: {}", e)))?;

            let line = line.trim();
            if line.is_empty() {
                break;
            }

            if let Some(len) = line.strip_prefix("Content-Length: ") {
                content_length = len
                    .parse()
                    .map_err(|e| crate::CoreError::Other(format!("Invalid content length: {}", e)))?;
            }
        }

        // Read body
        let mut body = vec![0u8; content_length];
        std::io::Read::read_exact(&mut reader, &mut body)
            .map_err(|e| crate::CoreError::Other(format!("DAP read body error: {}", e)))?;

        let msg: DapMessage = serde_json::from_slice(&body)?;
        Ok(msg)
    }

    /// Send initialize request
    pub fn initialize(&mut self) -> CoreResult<()> {
        self.send_request(
            "initialize",
            Some(serde_json::json!({
                "clientID": "localcode",
                "clientName": "LocalCode",
                "adapterID": "localcode",
                "pathFormat": "path",
                "linesStartAt1": true,
                "columnsStartAt1": true,
                "supportsVariableType": true,
                "supportsVariablePaging": false,
                "supportsRunInTerminalRequest": false,
                "locale": "en-US"
            })),
        )
    }

    /// Send launch request for a program
    pub fn launch(
        &mut self,
        program: &str,
        args: &[String],
        cwd: Option<&str>,
        env: Option<&HashMap<String, String>>,
    ) -> CoreResult<()> {
        let mut body = serde_json::json!({
            "program": program,
            "args": args,
            "noDebug": false,
        });

        if let Some(cwd) = cwd {
            body["cwd"] = serde_json::Value::String(cwd.to_string());
        }

        if let Some(env) = env {
            body["env"] = serde_json::to_value(env)?;
        }

        self.send_request("launch", Some(body))
    }

    /// Send attach request
    pub fn attach(&mut self, port: Option<u16>, pid: Option<u64>) -> CoreResult<()> {
        let mut body = serde_json::json!({});

        if let Some(port) = port {
            body["port"] = serde_json::Value::Number(port.into());
        }
        if let Some(pid) = pid {
            body["processId"] = serde_json::Value::Number(pid.into());
        }

        self.send_request("attach", Some(body))
    }

    /// Set breakpoints for a source file
    pub fn set_breakpoints(
        &mut self,
        path: &str,
        lines: &[u64],
    ) -> CoreResult<()> {
        let breakpoints: Vec<serde_json::Value> = lines
            .iter()
            .map(|&line| serde_json::json!({ "line": line }))
            .collect();

        self.send_request(
            "setBreakpoints",
            Some(serde_json::json!({
                "source": { "path": path },
                "breakpoints": breakpoints,
            })),
        )
    }

    /// Send configurationDone to indicate client is ready
    pub fn configuration_done(&mut self) -> CoreResult<()> {
        self.send_request("configurationDone", None)
    }

    /// Continue execution
    pub fn continue_execution(&mut self, thread_id: u64) -> CoreResult<()> {
        self.send_request(
            "continue",
            Some(serde_json::json!({ "threadId": thread_id })),
        )
    }

    /// Step over
    pub fn step_over(&mut self, thread_id: u64) -> CoreResult<()> {
        self.send_request(
            "next",
            Some(serde_json::json!({ "threadId": thread_id })),
        )
    }

    /// Step into
    pub fn step_into(&mut self, thread_id: u64) -> CoreResult<()> {
        self.send_request(
            "stepIn",
            Some(serde_json::json!({ "threadId": thread_id })),
        )
    }

    /// Step out
    pub fn step_out(&mut self, thread_id: u64) -> CoreResult<()> {
        self.send_request(
            "stepOut",
            Some(serde_json::json!({ "threadId": thread_id })),
        )
    }

    /// Pause execution
    pub fn pause(&mut self, thread_id: u64) -> CoreResult<()> {
        self.send_request(
            "pause",
            Some(serde_json::json!({ "threadId": thread_id })),
        )
    }

    /// Get stack trace
    pub fn stack_trace(&mut self, thread_id: u64) -> CoreResult<()> {
        self.send_request(
            "stackTrace",
            Some(serde_json::json!({
                "threadId": thread_id,
                "startFrame": 0,
                "levels": 20,
            })),
        )
    }

    /// Get scopes for a frame
    pub fn scopes(&mut self, frame_id: u64) -> CoreResult<()> {
        self.send_request(
            "scopes",
            Some(serde_json::json!({ "frameId": frame_id })),
        )
    }

    /// Get variables for a scope
    pub fn variables(&mut self, variables_reference: u64) -> CoreResult<()> {
        self.send_request(
            "variables",
            Some(serde_json::json!({ "variablesReference": variables_reference })),
        )
    }

    /// Evaluate an expression
    pub fn evaluate(
        &mut self,
        expression: &str,
        frame_id: Option<u64>,
        context: &str,
    ) -> CoreResult<()> {
        let mut body = serde_json::json!({
            "expression": expression,
            "context": context,
        });

        if let Some(fid) = frame_id {
            body["frameId"] = serde_json::Value::Number(fid.into());
        }

        self.send_request("evaluate", Some(body))
    }

    /// Disconnect from the debug adapter
    pub fn disconnect(&mut self, terminate: bool) -> CoreResult<()> {
        self.send_request(
            "disconnect",
            Some(serde_json::json!({
                "restart": false,
                "terminateDebuggee": terminate,
            })),
        )?;

        if let Some(ref mut child) = self.process {
            let _ = child.kill();
        }
        self.process = None;

        Ok(())
    }

    pub fn is_running(&self) -> bool {
        self.process.is_some()
    }

    pub fn capabilities(&self) -> &ServerCapabilities {
        &self.capabilities
    }
}

impl Default for DapClient {
    fn default() -> Self {
        Self::new()
    }
}

impl Drop for DapClient {
    fn drop(&mut self) {
        if let Some(ref mut child) = self.process {
            let _ = child.kill();
        }
    }
}
