use serde::{Deserialize, Serialize};
use std::path::{Path, PathBuf};

use crate::CoreResult;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Config {
    #[serde(default)]
    pub default_provider: String,
    #[serde(default)]
    pub providers: ProvidersConfig,
    #[serde(default)]
    pub editor: EditorConfig,
    #[serde(default)]
    pub agent: AgentConfig,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct ProvidersConfig {
    #[serde(default)]
    pub local: LocalConfig,
    #[serde(default)]
    pub openai: OpenAIConfig,
    #[serde(default)]
    pub anthropic: AnthropicConfig,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LocalConfig {
    pub server_url: String,
    pub model_path: String,
    pub context_size: u32,
    pub gpu_layers: u32,
    #[serde(default)]
    pub active_catalog_model: Option<String>,
}

impl Default for LocalConfig {
    fn default() -> Self {
        Self {
            server_url: "http://127.0.0.1:8081".to_string(),
            model_path: String::new(),
            context_size: 4096,
            gpu_layers: 99,
            active_catalog_model: None,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct OpenAIConfig {
    pub api_key: String,
    pub model: String,
    pub base_url: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct AnthropicConfig {
    pub api_key: String,
    pub model: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EditorConfig {
    pub font_size: u32,
    pub font_family: String,
    pub tab_size: u32,
    pub word_wrap: bool,
    pub minimap: bool,
    pub theme: String,
}

impl Default for EditorConfig {
    fn default() -> Self {
        Self {
            font_size: 14,
            font_family: "'JetBrains Mono', 'Fira Code', 'SF Mono', 'Menlo', monospace"
                .to_string(),
            tab_size: 2,
            word_wrap: false,
            minimap: true,
            theme: "localcode-dark".to_string(),
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AgentConfig {
    pub max_iterations: usize,
    pub auto_approve_reads: bool,
    pub auto_approve_writes: bool,
    pub auto_approve_commands: bool,
}

impl Default for AgentConfig {
    fn default() -> Self {
        Self {
            max_iterations: 15,
            auto_approve_reads: true,
            auto_approve_writes: false,
            auto_approve_commands: false,
        }
    }
}

impl Default for Config {
    fn default() -> Self {
        Self {
            default_provider: "local".to_string(),
            providers: ProvidersConfig::default(),
            editor: EditorConfig::default(),
            agent: AgentConfig::default(),
        }
    }
}

impl Config {
    pub fn config_dir() -> PathBuf {
        dirs::home_dir()
            .unwrap_or_else(|| PathBuf::from("~"))
            .join(".localcode")
    }

    pub fn config_path() -> PathBuf {
        Self::config_dir().join("config.toml")
    }

    pub fn load() -> CoreResult<Self> {
        let path = Self::config_path();
        if path.exists() {
            let content = std::fs::read_to_string(&path)?;
            let config: Config =
                toml::from_str(&content).map_err(|e| crate::CoreError::Config(e.to_string()))?;
            Ok(config)
        } else {
            Ok(Config::default())
        }
    }

    pub fn save(&self) -> CoreResult<()> {
        let path = Self::config_path();
        if let Some(parent) = path.parent() {
            std::fs::create_dir_all(parent)?;
        }
        let content =
            toml::to_string_pretty(self).map_err(|e| crate::CoreError::Config(e.to_string()))?;
        std::fs::write(&path, content)?;
        Ok(())
    }

    /// Load API key from environment variable or config
    pub fn get_openai_key(&self) -> String {
        std::env::var("OPENAI_API_KEY")
            .unwrap_or_else(|_| self.providers.openai.api_key.clone())
    }

    pub fn get_anthropic_key(&self) -> String {
        std::env::var("ANTHROPIC_API_KEY")
            .unwrap_or_else(|_| self.providers.anthropic.api_key.clone())
    }

    pub fn get_openai_model(&self) -> String {
        if self.providers.openai.model.is_empty() {
            "gpt-4o".to_string()
        } else {
            self.providers.openai.model.clone()
        }
    }

    pub fn get_anthropic_model(&self) -> String {
        if self.providers.anthropic.model.is_empty() {
            "claude-sonnet-4-20250514".to_string()
        } else {
            self.providers.anthropic.model.clone()
        }
    }
}
