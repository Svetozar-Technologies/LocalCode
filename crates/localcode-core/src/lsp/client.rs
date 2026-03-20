use serde::{Deserialize, Serialize};
use std::io::{BufRead, BufReader, Read, Write};
use std::process::{Child, Command, Stdio};
use std::sync::atomic::{AtomicI64, Ordering};
use std::sync::Mutex;

use crate::CoreResult;
use crate::CoreError;

/// JSON-RPC message for LSP communication
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct JsonRpcMessage {
    pub jsonrpc: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub id: Option<serde_json::Value>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub method: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub params: Option<serde_json::Value>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub result: Option<serde_json::Value>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub error: Option<serde_json::Value>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LspLocation {
    pub uri: String,
    pub range: LspRange,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LspRange {
    pub start: LspPosition,
    pub end: LspPosition,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LspPosition {
    pub line: u32,
    pub character: u32,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LspCompletionItem {
    pub label: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub kind: Option<u32>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub detail: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub documentation: Option<serde_json::Value>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub insert_text: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LspDiagnostic {
    pub range: LspRange,
    pub severity: Option<u32>,
    pub message: String,
    pub source: Option<String>,
    pub code: Option<serde_json::Value>,
}

/// LSP Client that communicates with a language server over stdio
pub struct LspClient {
    process: Mutex<Option<Child>>,
    next_id: AtomicI64,
    language: String,
    initialized: Mutex<bool>,
}

impl LspClient {
    pub fn new(command: &str, args: &[&str], language: &str) -> CoreResult<Self> {
        let process = Command::new(command)
            .args(args)
            .stdin(Stdio::piped())
            .stdout(Stdio::piped())
            .stderr(Stdio::null())
            .spawn()
            .map_err(|e| CoreError::Other(format!("Failed to start LSP server '{}': {}", command, e)))?;

        Ok(Self {
            process: Mutex::new(Some(process)),
            next_id: AtomicI64::new(1),
            language: language.to_string(),
            initialized: Mutex::new(false),
        })
    }

    pub fn language(&self) -> &str {
        &self.language
    }

    fn next_id(&self) -> i64 {
        self.next_id.fetch_add(1, Ordering::SeqCst)
    }

    fn send_request(&self, method: &str, params: serde_json::Value) -> CoreResult<serde_json::Value> {
        let id = self.next_id();
        let msg = serde_json::json!({
            "jsonrpc": "2.0",
            "id": id,
            "method": method,
            "params": params,
        });

        let content = serde_json::to_string(&msg)?;
        let header = format!("Content-Length: {}\r\n\r\n", content.len());

        let mut guard = self.process.lock().map_err(|e| CoreError::Other(e.to_string()))?;
        let process = guard.as_mut().ok_or_else(|| CoreError::Other("LSP process not running".to_string()))?;

        // Write request
        if let Some(ref mut stdin) = process.stdin {
            stdin.write_all(header.as_bytes())?;
            stdin.write_all(content.as_bytes())?;
            stdin.flush()?;
        }

        // Read response
        if let Some(ref mut stdout) = process.stdout {
            let mut reader = BufReader::new(stdout);
            let mut header_line = String::new();
            let mut content_length: usize = 0;

            // Read headers
            loop {
                header_line.clear();
                reader.read_line(&mut header_line)?;
                let trimmed = header_line.trim();
                if trimmed.is_empty() {
                    break;
                }
                if let Some(len_str) = trimmed.strip_prefix("Content-Length: ") {
                    content_length = len_str.parse().unwrap_or(0);
                }
            }

            if content_length > 0 {
                let mut body = vec![0u8; content_length];
                reader.read_exact(&mut body)?;
                let response: serde_json::Value = serde_json::from_slice(&body)?;
                if let Some(result) = response.get("result") {
                    return Ok(result.clone());
                }
                if let Some(error) = response.get("error") {
                    return Err(CoreError::Other(format!("LSP error: {}", error)));
                }
                return Ok(response);
            }
        }

        Err(CoreError::Other("No response from LSP server".to_string()))
    }

    fn send_notification(&self, method: &str, params: serde_json::Value) -> CoreResult<()> {
        let msg = serde_json::json!({
            "jsonrpc": "2.0",
            "method": method,
            "params": params,
        });

        let content = serde_json::to_string(&msg)?;
        let header = format!("Content-Length: {}\r\n\r\n", content.len());

        let mut guard = self.process.lock().map_err(|e| CoreError::Other(e.to_string()))?;
        let process = guard.as_mut().ok_or_else(|| CoreError::Other("LSP process not running".to_string()))?;

        if let Some(ref mut stdin) = process.stdin {
            stdin.write_all(header.as_bytes())?;
            stdin.write_all(content.as_bytes())?;
            stdin.flush()?;
        }

        Ok(())
    }

    /// Initialize the LSP server
    pub fn initialize(&self, project_path: &str) -> CoreResult<serde_json::Value> {
        let root_uri = format!("file://{}", project_path);
        let result = self.send_request("initialize", serde_json::json!({
            "processId": std::process::id(),
            "rootUri": root_uri,
            "capabilities": {
                "textDocument": {
                    "hover": { "contentFormat": ["plaintext", "markdown"] },
                    "completion": {
                        "completionItem": {
                            "snippetSupport": true,
                            "documentationFormat": ["plaintext", "markdown"]
                        }
                    },
                    "definition": {},
                    "references": {},
                    "publishDiagnostics": { "relatedInformation": true }
                },
                "workspace": {
                    "workspaceFolders": true
                }
            },
            "workspaceFolders": [{
                "uri": root_uri,
                "name": project_path.split('/').next_back().unwrap_or("project")
            }]
        }))?;

        // Send initialized notification
        self.send_notification("initialized", serde_json::json!({}))?;

        let mut init = self.initialized.lock().map_err(|e| CoreError::Other(e.to_string()))?;
        *init = true;

        Ok(result)
    }

    /// Notify the server that a file was opened
    pub fn did_open(&self, file_path: &str, language_id: &str, content: &str) -> CoreResult<()> {
        self.send_notification("textDocument/didOpen", serde_json::json!({
            "textDocument": {
                "uri": format!("file://{}", file_path),
                "languageId": language_id,
                "version": 1,
                "text": content,
            }
        }))
    }

    /// Get hover information at a position
    pub fn hover(&self, file_path: &str, line: u32, character: u32) -> CoreResult<Option<String>> {
        let result = self.send_request("textDocument/hover", serde_json::json!({
            "textDocument": { "uri": format!("file://{}", file_path) },
            "position": { "line": line, "character": character }
        }))?;

        if result.is_null() {
            return Ok(None);
        }

        let contents = result.get("contents");
        if let Some(contents) = contents {
            if let Some(value) = contents.get("value") {
                return Ok(Some(value.as_str().unwrap_or("").to_string()));
            }
            if let Some(s) = contents.as_str() {
                return Ok(Some(s.to_string()));
            }
        }

        Ok(None)
    }

    /// Get definition location
    pub fn definition(&self, file_path: &str, line: u32, character: u32) -> CoreResult<Option<LspLocation>> {
        let result = self.send_request("textDocument/definition", serde_json::json!({
            "textDocument": { "uri": format!("file://{}", file_path) },
            "position": { "line": line, "character": character }
        }))?;

        if result.is_null() {
            return Ok(None);
        }

        // Can be a single location or an array
        if result.is_array() {
            if let Some(first) = result.get(0) {
                let loc: LspLocation = serde_json::from_value(first.clone())?;
                return Ok(Some(loc));
            }
        } else {
            let loc: LspLocation = serde_json::from_value(result)?;
            return Ok(Some(loc));
        }

        Ok(None)
    }

    /// Find references
    pub fn references(&self, file_path: &str, line: u32, character: u32) -> CoreResult<Vec<LspLocation>> {
        let result = self.send_request("textDocument/references", serde_json::json!({
            "textDocument": { "uri": format!("file://{}", file_path) },
            "position": { "line": line, "character": character },
            "context": { "includeDeclaration": true }
        }))?;

        if result.is_null() || !result.is_array() {
            return Ok(vec![]);
        }

        let locations: Vec<LspLocation> = serde_json::from_value(result)?;
        Ok(locations)
    }

    /// Get completions
    pub fn completions(&self, file_path: &str, line: u32, character: u32) -> CoreResult<Vec<LspCompletionItem>> {
        let result = self.send_request("textDocument/completion", serde_json::json!({
            "textDocument": { "uri": format!("file://{}", file_path) },
            "position": { "line": line, "character": character }
        }))?;

        if result.is_null() {
            return Ok(vec![]);
        }

        // Can be CompletionList or array of CompletionItem
        if let Some(items) = result.get("items") {
            let items: Vec<LspCompletionItem> = serde_json::from_value(items.clone())?;
            return Ok(items);
        }

        if result.is_array() {
            let items: Vec<LspCompletionItem> = serde_json::from_value(result)?;
            return Ok(items);
        }

        Ok(vec![])
    }

    /// Shutdown and exit
    pub fn shutdown(&self) -> CoreResult<()> {
        let _ = self.send_request("shutdown", serde_json::json!(null));
        let _ = self.send_notification("exit", serde_json::json!(null));

        let mut guard = self.process.lock().map_err(|e| CoreError::Other(e.to_string()))?;
        if let Some(mut process) = guard.take() {
            let _ = process.kill();
            let _ = process.wait();
        }

        Ok(())
    }
}

impl Drop for LspClient {
    fn drop(&mut self) {
        let _ = self.shutdown();
    }
}

