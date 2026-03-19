use std::collections::HashMap;
use std::sync::Mutex;

use crate::CoreResult;
use crate::CoreError;
use super::client::LspClient;

/// Language server configuration
struct LspServerConfig {
    command: &'static str,
    args: &'static [&'static str],
}

/// Get the LSP server configuration for a language
fn get_server_config(language: &str) -> Option<LspServerConfig> {
    match language {
        "typescript" | "typescriptreact" | "javascript" | "javascriptreact" => Some(LspServerConfig {
            command: "typescript-language-server",
            args: &["--stdio"],
        }),
        "python" => Some(LspServerConfig {
            command: "pyright-langserver",
            args: &["--stdio"],
        }),
        "rust" => Some(LspServerConfig {
            command: "rust-analyzer",
            args: &[],
        }),
        "go" => Some(LspServerConfig {
            command: "gopls",
            args: &["serve"],
        }),
        "c" | "cpp" => Some(LspServerConfig {
            command: "clangd",
            args: &[],
        }),
        "java" => Some(LspServerConfig {
            command: "jdtls",
            args: &[],
        }),
        _ => None,
    }
}

/// Manages LSP clients for different languages
pub struct LspManager {
    clients: Mutex<HashMap<String, LspClient>>,
    project_path: Mutex<Option<String>>,
}

impl LspManager {
    pub fn new() -> Self {
        Self {
            clients: Mutex::new(HashMap::new()),
            project_path: Mutex::new(None),
        }
    }

    /// Set the project path for LSP initialization
    pub fn set_project_path(&self, path: &str) {
        if let Ok(mut pp) = self.project_path.lock() {
            *pp = Some(path.to_string());
        }
    }

    /// Start an LSP server for a language (lazy — only when needed)
    pub fn start(&self, language: &str) -> CoreResult<()> {
        let mut clients = self.clients.lock().map_err(|e| CoreError::Other(e.to_string()))?;

        // Already running
        if clients.contains_key(language) {
            return Ok(());
        }

        let config = get_server_config(language)
            .ok_or_else(|| CoreError::Other(format!("No LSP server configured for language: {}", language)))?;

        let client = LspClient::new(config.command, config.args, language)?;

        // Initialize with project path
        let project_path = self.project_path.lock()
            .map_err(|e| CoreError::Other(e.to_string()))?
            .clone()
            .unwrap_or_else(|| ".".to_string());

        client.initialize(&project_path)?;
        clients.insert(language.to_string(), client);

        Ok(())
    }

    /// Get hover information
    pub fn hover(&self, language: &str, file_path: &str, line: u32, character: u32) -> CoreResult<Option<String>> {
        let clients = self.clients.lock().map_err(|e| CoreError::Other(e.to_string()))?;
        if let Some(client) = clients.get(language) {
            client.hover(file_path, line, character)
        } else {
            Err(CoreError::Other(format!("LSP server for {} is not running", language)))
        }
    }

    /// Get definition location
    pub fn definition(&self, language: &str, file_path: &str, line: u32, character: u32) -> CoreResult<Option<super::client::LspLocation>> {
        let clients = self.clients.lock().map_err(|e| CoreError::Other(e.to_string()))?;
        if let Some(client) = clients.get(language) {
            client.definition(file_path, line, character)
        } else {
            Err(CoreError::Other(format!("LSP server for {} is not running", language)))
        }
    }

    /// Find references
    pub fn references(&self, language: &str, file_path: &str, line: u32, character: u32) -> CoreResult<Vec<super::client::LspLocation>> {
        let clients = self.clients.lock().map_err(|e| CoreError::Other(e.to_string()))?;
        if let Some(client) = clients.get(language) {
            client.references(file_path, line, character)
        } else {
            Err(CoreError::Other(format!("LSP server for {} is not running", language)))
        }
    }

    /// Get completions
    pub fn completions(&self, language: &str, file_path: &str, line: u32, character: u32) -> CoreResult<Vec<super::client::LspCompletionItem>> {
        let clients = self.clients.lock().map_err(|e| CoreError::Other(e.to_string()))?;
        if let Some(client) = clients.get(language) {
            client.completions(file_path, line, character)
        } else {
            Err(CoreError::Other(format!("LSP server for {} is not running", language)))
        }
    }

    /// Notify server about file open
    pub fn did_open(&self, language: &str, file_path: &str, content: &str) -> CoreResult<()> {
        let clients = self.clients.lock().map_err(|e| CoreError::Other(e.to_string()))?;
        if let Some(client) = clients.get(language) {
            client.did_open(file_path, language, content)
        } else {
            Ok(()) // Not an error if server isn't running
        }
    }

    /// Shutdown all LSP servers
    pub fn shutdown_all(&self) {
        if let Ok(mut clients) = self.clients.lock() {
            for (_, client) in clients.drain() {
                let _ = client.shutdown();
            }
        }
    }
}

impl Drop for LspManager {
    fn drop(&mut self) {
        self.shutdown_all();
    }
}
