use serde::{Deserialize, Serialize};
use std::path::{Path, PathBuf};

use crate::CoreResult;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PluginManifest {
    pub name: String,
    pub version: String,
    #[serde(default)]
    pub description: String,
    #[serde(default)]
    pub author: String,
    #[serde(default)]
    pub license: String,
    /// Path to the WASM binary relative to manifest
    pub main: String,
    #[serde(default)]
    pub capabilities: PluginCapabilities,
    #[serde(default)]
    pub commands: Vec<PluginCommand>,
    #[serde(default)]
    pub tools: Vec<PluginToolDef>,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct PluginCapabilities {
    /// Can read files
    #[serde(default)]
    pub fs_read: bool,
    /// Can write files
    #[serde(default)]
    pub fs_write: bool,
    /// Can execute commands
    #[serde(default)]
    pub exec: bool,
    /// Can make network requests
    #[serde(default)]
    pub network: bool,
    /// Can access environment variables
    #[serde(default)]
    pub env: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PluginCommand {
    pub name: String,
    pub description: String,
    #[serde(default)]
    pub keybinding: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PluginToolDef {
    pub name: String,
    pub description: String,
    pub parameters_schema: serde_json::Value,
}

#[derive(Debug, Clone)]
pub struct LoadedPlugin {
    pub manifest: PluginManifest,
    pub path: PathBuf,
    pub wasm_path: PathBuf,
    pub enabled: bool,
}

/// Discover plugins in a directory
pub fn discover_plugins(plugins_dir: &Path) -> CoreResult<Vec<LoadedPlugin>> {
    let mut plugins = Vec::new();

    if !plugins_dir.exists() {
        return Ok(plugins);
    }

    let entries = std::fs::read_dir(plugins_dir)
        .map_err(|e| crate::CoreError::Other(format!("Failed to read plugins dir: {}", e)))?;

    for entry in entries {
        let entry = entry.map_err(|e| crate::CoreError::Other(e.to_string()))?;
        let path = entry.path();

        if !path.is_dir() {
            continue;
        }

        let manifest_path = path.join("plugin.json");
        if !manifest_path.exists() {
            continue;
        }

        match load_plugin_manifest(&path) {
            Ok(plugin) => plugins.push(plugin),
            Err(e) => {
                log::warn!("Failed to load plugin at {:?}: {}", path, e);
            }
        }
    }

    Ok(plugins)
}

fn load_plugin_manifest(plugin_dir: &Path) -> CoreResult<LoadedPlugin> {
    let manifest_path = plugin_dir.join("plugin.json");
    let content = std::fs::read_to_string(&manifest_path)?;
    let manifest: PluginManifest = serde_json::from_str(&content)?;

    let wasm_path = plugin_dir.join(&manifest.main);
    if !wasm_path.exists() {
        return Err(crate::CoreError::Other(format!(
            "WASM binary not found: {}",
            wasm_path.display()
        )));
    }

    Ok(LoadedPlugin {
        manifest,
        path: plugin_dir.to_path_buf(),
        wasm_path,
        enabled: true,
    })
}

/// Get the global plugins directory
pub fn global_plugins_dir() -> PathBuf {
    dirs::home_dir()
        .unwrap_or_else(|| PathBuf::from("."))
        .join(".localcode")
        .join("plugins")
}

/// Get the project-local plugins directory
pub fn project_plugins_dir(project_path: &str) -> PathBuf {
    Path::new(project_path).join(".localcode").join("plugins")
}
