use std::collections::HashMap;
use std::path::Path;

use super::manifest::{LoadedPlugin, PluginCapabilities};
use crate::CoreResult;

/// Plugin host manages loaded plugins and their execution
pub struct PluginHost {
    plugins: HashMap<String, LoadedPlugin>,
}

/// Result of a plugin function call
#[derive(Debug, Clone)]
pub struct PluginCallResult {
    pub output: String,
    pub error: Option<String>,
}

impl PluginHost {
    pub fn new() -> Self {
        Self {
            plugins: HashMap::new(),
        }
    }

    /// Load all plugins from global and project directories
    pub fn load_plugins(&mut self, project_path: Option<&str>) -> CoreResult<()> {
        // Load global plugins
        let global_dir = super::manifest::global_plugins_dir();
        if let Ok(plugins) = super::manifest::discover_plugins(&global_dir) {
            for plugin in plugins {
                log::info!("Loaded global plugin: {}", plugin.manifest.name);
                self.plugins
                    .insert(plugin.manifest.name.clone(), plugin);
            }
        }

        // Load project-local plugins
        if let Some(project) = project_path {
            let project_dir = super::manifest::project_plugins_dir(project);
            if let Ok(plugins) = super::manifest::discover_plugins(&project_dir) {
                for plugin in plugins {
                    log::info!("Loaded project plugin: {}", plugin.manifest.name);
                    self.plugins
                        .insert(plugin.manifest.name.clone(), plugin);
                }
            }
        }

        Ok(())
    }

    /// Get all loaded plugins
    pub fn plugins(&self) -> &HashMap<String, LoadedPlugin> {
        &self.plugins
    }

    /// Get a specific plugin
    pub fn get_plugin(&self, name: &str) -> Option<&LoadedPlugin> {
        self.plugins.get(name)
    }

    /// Enable or disable a plugin
    pub fn set_enabled(&mut self, name: &str, enabled: bool) -> CoreResult<()> {
        let plugin = self
            .plugins
            .get_mut(name)
            .ok_or_else(|| crate::CoreError::Other(format!("Plugin not found: {}", name)))?;
        plugin.enabled = enabled;
        Ok(())
    }

    /// Call a plugin function (WASM execution)
    ///
    /// This is a placeholder for actual WASM runtime integration.
    /// In production, this would use Extism or wasmtime to execute
    /// the plugin's WASM binary with the provided input.
    pub fn call_plugin(
        &self,
        plugin_name: &str,
        function: &str,
        input: &str,
    ) -> CoreResult<PluginCallResult> {
        let plugin = self
            .plugins
            .get(plugin_name)
            .ok_or_else(|| crate::CoreError::Other(format!("Plugin not found: {}", plugin_name)))?;

        if !plugin.enabled {
            return Err(crate::CoreError::Other(format!(
                "Plugin {} is disabled",
                plugin_name
            )));
        }

        // Verify capabilities before execution
        self.verify_capabilities(&plugin.manifest.capabilities, function)?;

        // TODO: Actual WASM execution via Extism
        // For now, return a placeholder indicating the plugin system is ready
        // but WASM runtime is not yet integrated
        log::info!(
            "Plugin call: {}::{} with WASM at {:?}",
            plugin_name,
            function,
            plugin.wasm_path
        );

        Ok(PluginCallResult {
            output: format!(
                "Plugin '{}' function '{}' called. WASM runtime pending integration.",
                plugin_name, function
            ),
            error: None,
        })
    }

    /// Call a plugin-provided tool (for agent integration)
    pub fn call_tool(
        &self,
        plugin_name: &str,
        tool_name: &str,
        args: serde_json::Value,
    ) -> CoreResult<String> {
        let input = serde_json::json!({
            "tool": tool_name,
            "arguments": args,
        });

        let result = self.call_plugin(plugin_name, "handle_tool_call", &input.to_string())?;

        if let Some(err) = result.error {
            return Err(crate::CoreError::Other(err));
        }

        Ok(result.output)
    }

    /// Get all tools registered by all enabled plugins
    pub fn all_plugin_tools(&self) -> Vec<(&str, &super::manifest::PluginToolDef)> {
        let mut tools = Vec::new();
        for (name, plugin) in &self.plugins {
            if !plugin.enabled {
                continue;
            }
            for tool in &plugin.manifest.tools {
                tools.push((name.as_str(), tool));
            }
        }
        tools
    }

    /// Get all commands registered by all enabled plugins
    pub fn all_plugin_commands(&self) -> Vec<(&str, &super::manifest::PluginCommand)> {
        let mut commands = Vec::new();
        for (name, plugin) in &self.plugins {
            if !plugin.enabled {
                continue;
            }
            for cmd in &plugin.manifest.commands {
                commands.push((name.as_str(), cmd));
            }
        }
        commands
    }

    fn verify_capabilities(
        &self,
        capabilities: &PluginCapabilities,
        function: &str,
    ) -> CoreResult<()> {
        // Basic capability verification — in production this would be
        // enforced at the WASM sandbox level
        log::debug!(
            "Verifying capabilities for function '{}': fs_read={}, fs_write={}, exec={}, network={}",
            function,
            capabilities.fs_read,
            capabilities.fs_write,
            capabilities.exec,
            capabilities.network
        );
        Ok(())
    }
}

impl Default for PluginHost {
    fn default() -> Self {
        Self::new()
    }
}
