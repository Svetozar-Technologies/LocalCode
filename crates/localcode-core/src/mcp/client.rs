use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::path::Path;

use crate::CoreResult;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MCPConfig {
    pub servers: HashMap<String, MCPServerConfig>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MCPServerConfig {
    #[serde(default)]
    pub command: Option<String>,
    #[serde(default)]
    pub args: Vec<String>,
    #[serde(default)]
    pub url: Option<String>,
    #[serde(default = "default_transport")]
    pub transport: String,
}

fn default_transport() -> String {
    "stdio".to_string()
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MCPTool {
    pub name: String,
    pub description: String,
    pub input_schema: serde_json::Value,
    pub server: String,
}

pub struct MCPClient {
    config: MCPConfig,
    tools: Vec<MCPTool>,
}

impl MCPClient {
    pub fn new() -> Self {
        Self {
            config: MCPConfig {
                servers: HashMap::new(),
            },
            tools: Vec::new(),
        }
    }

    pub fn load_config(project_path: &str) -> CoreResult<Self> {
        let config_path = Path::new(project_path).join(".localcode").join("mcp.json");

        if !config_path.exists() {
            return Ok(Self::new());
        }

        let content = std::fs::read_to_string(&config_path)?;
        let config: MCPConfig = serde_json::from_str(&content)?;

        Ok(Self {
            config,
            tools: Vec::new(),
        })
    }

    pub fn servers(&self) -> &HashMap<String, MCPServerConfig> {
        &self.config.servers
    }

    pub fn tools(&self) -> &[MCPTool] {
        &self.tools
    }

    /// Initialize connections and discover tools from all servers
    pub async fn discover_tools(&mut self) -> CoreResult<()> {
        self.tools.clear();

        for (name, server) in &self.config.servers {
            match server.transport.as_str() {
                "stdio" => {
                    if let Some(ref command) = server.command {
                        match self.discover_stdio_tools(name, command, &server.args).await {
                            Ok(tools) => self.tools.extend(tools),
                            Err(e) => {
                                log::warn!("Failed to discover tools from {}: {}", name, e);
                            }
                        }
                    }
                }
                "sse" => {
                    if let Some(ref url) = server.url {
                        match self.discover_sse_tools(name, url).await {
                            Ok(tools) => self.tools.extend(tools),
                            Err(e) => {
                                log::warn!("Failed to discover tools from {}: {}", name, e);
                            }
                        }
                    }
                }
                _ => {}
            }
        }

        Ok(())
    }

    async fn discover_stdio_tools(
        &self,
        server_name: &str,
        command: &str,
        args: &[String],
    ) -> CoreResult<Vec<MCPTool>> {
        // MCP stdio transport: spawn process and communicate via JSON-RPC
        let mut child = std::process::Command::new(command)
            .args(args)
            .stdin(std::process::Stdio::piped())
            .stdout(std::process::Stdio::piped())
            .spawn()
            .map_err(|e| crate::CoreError::Other(format!("Failed to start MCP server {}: {}", server_name, e)))?;

        // Send initialize request
        let init_req = serde_json::json!({
            "jsonrpc": "2.0",
            "id": 1,
            "method": "initialize",
            "params": {
                "protocolVersion": "2024-11-05",
                "capabilities": {},
                "clientInfo": {
                    "name": "localcode",
                    "version": env!("CARGO_PKG_VERSION")
                }
            }
        });

        if let Some(ref mut stdin) = child.stdin {
            use std::io::Write;
            let msg = serde_json::to_string(&init_req)?;
            writeln!(stdin, "{}", msg).map_err(|e| crate::CoreError::Other(e.to_string()))?;
        }

        // Send tools/list request
        let list_req = serde_json::json!({
            "jsonrpc": "2.0",
            "id": 2,
            "method": "tools/list",
            "params": {}
        });

        if let Some(ref mut stdin) = child.stdin {
            use std::io::Write;
            let msg = serde_json::to_string(&list_req)?;
            writeln!(stdin, "{}", msg).map_err(|e| crate::CoreError::Other(e.to_string()))?;
        }

        // Read response (simplified — real impl would be async with proper framing)
        let mut tools = Vec::new();

        if let Some(ref mut stdout) = child.stdout {
            use std::io::BufRead;
            let reader = std::io::BufReader::new(stdout);
            for line in reader.lines().take(10).flatten() {
                if let Ok(response) = serde_json::from_str::<serde_json::Value>(&line) {
                    if let Some(result) = response.get("result") {
                        if let Some(tool_list) = result.get("tools").and_then(|t| t.as_array()) {
                            for tool in tool_list {
                                tools.push(MCPTool {
                                    name: tool["name"].as_str().unwrap_or("").to_string(),
                                    description: tool["description"].as_str().unwrap_or("").to_string(),
                                    input_schema: tool["inputSchema"].clone(),
                                    server: server_name.to_string(),
                                });
                            }
                        }
                    }
                }
            }
        }

        let _ = child.kill();
        Ok(tools)
    }

    async fn discover_sse_tools(
        &self,
        server_name: &str,
        url: &str,
    ) -> CoreResult<Vec<MCPTool>> {
        // SSE transport: connect to HTTP endpoint
        let client = reqwest::Client::new();

        let response = client
            .post(format!("{}/tools/list", url))
            .json(&serde_json::json!({
                "jsonrpc": "2.0",
                "id": 1,
                "method": "tools/list",
                "params": {}
            }))
            .send()
            .await?;

        let result: serde_json::Value = response.json().await?;

        let mut tools = Vec::new();
        if let Some(tool_list) = result["result"]["tools"].as_array() {
            for tool in tool_list {
                tools.push(MCPTool {
                    name: tool["name"].as_str().unwrap_or("").to_string(),
                    description: tool["description"].as_str().unwrap_or("").to_string(),
                    input_schema: tool["inputSchema"].clone(),
                    server: server_name.to_string(),
                });
            }
        }

        Ok(tools)
    }

    /// Call a tool on an MCP server
    pub async fn call_tool(
        &self,
        tool_name: &str,
        args: serde_json::Value,
    ) -> CoreResult<String> {
        let tool = self
            .tools
            .iter()
            .find(|t| t.name == tool_name)
            .ok_or_else(|| crate::CoreError::Other(format!("MCP tool not found: {}", tool_name)))?;

        let server = self
            .config
            .servers
            .get(&tool.server)
            .ok_or_else(|| crate::CoreError::Other(format!("MCP server not found: {}", tool.server)))?;

        match server.transport.as_str() {
            "sse" => {
                if let Some(ref url) = server.url {
                    let client = reqwest::Client::new();
                    let response = client
                        .post(format!("{}/tools/call", url))
                        .json(&serde_json::json!({
                            "jsonrpc": "2.0",
                            "id": 1,
                            "method": "tools/call",
                            "params": {
                                "name": tool_name,
                                "arguments": args
                            }
                        }))
                        .send()
                        .await?;

                    let result: serde_json::Value = response.json().await?;
                    Ok(result["result"]["content"][0]["text"]
                        .as_str()
                        .unwrap_or("")
                        .to_string())
                } else {
                    Err(crate::CoreError::Other("No URL for SSE server".to_string()))
                }
            }
            _ => Err(crate::CoreError::Other(
                "Stdio tool calling not yet implemented for persistent connections".to_string(),
            )),
        }
    }
}

impl Default for MCPClient {
    fn default() -> Self {
        Self::new()
    }
}
