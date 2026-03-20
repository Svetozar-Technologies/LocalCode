use serde::{Deserialize, Serialize};

/// API types that plugins can interact with.
/// These define the host functions available to WASM plugins.
/// Request from plugin to host
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type")]
pub enum HostRequest {
    /// Read a file
    ReadFile { path: String },
    /// Write a file
    WriteFile { path: String, content: String },
    /// Execute a command
    Exec { command: String, args: Vec<String> },
    /// HTTP request
    HttpRequest {
        method: String,
        url: String,
        headers: std::collections::HashMap<String, String>,
        body: Option<String>,
    },
    /// Get environment variable
    GetEnv { name: String },
    /// Log a message
    Log { level: String, message: String },
    /// Show notification to user
    Notify { message: String, level: String },
    /// Register a UI panel (desktop only)
    RegisterPanel {
        id: String,
        title: String,
        position: String,
    },
}

/// Response from host to plugin
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type")]
pub enum HostResponse {
    Success { data: serde_json::Value },
    Error { message: String },
}

/// Plugin metadata exposed to the host
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PluginInfo {
    pub name: String,
    pub version: String,
    pub description: String,
    pub tools: Vec<PluginToolInfo>,
    pub commands: Vec<PluginCommandInfo>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PluginToolInfo {
    pub name: String,
    pub description: String,
    pub schema: serde_json::Value,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PluginCommandInfo {
    pub name: String,
    pub description: String,
}

/// Tool call request sent to plugin
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ToolCallRequest {
    pub tool: String,
    pub arguments: serde_json::Value,
}

/// Tool call response from plugin
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ToolCallResponse {
    pub content: String,
    #[serde(default)]
    pub is_error: bool,
}

/// Command execution request sent to plugin
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CommandRequest {
    pub command: String,
    pub args: Vec<String>,
    pub context: CommandContext,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CommandContext {
    pub project_path: Option<String>,
    pub current_file: Option<String>,
    pub selection: Option<String>,
}

/// Command execution response from plugin
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CommandResponse {
    pub output: Option<String>,
    #[serde(default)]
    pub edits: Vec<FileEdit>,
    #[serde(default)]
    pub notifications: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FileEdit {
    pub path: String,
    pub content: String,
}
