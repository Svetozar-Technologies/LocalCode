use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::path::PathBuf;
use std::process::Command;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HookConfig {
    pub command: String,
    #[serde(default = "default_on_failure")]
    pub on_failure: String,
}

fn default_on_failure() -> String {
    "warn".to_string()
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct HooksConfig {
    #[serde(default)]
    pub post_edit: Option<HookConfig>,
    #[serde(default)]
    pub pre_commit: Option<HookConfig>,
    #[serde(default)]
    pub post_commit: Option<HookConfig>,
}

impl HooksConfig {
    pub fn load() -> Self {
        let path = hooks_path();
        if path.exists() {
            if let Ok(content) = std::fs::read_to_string(&path) {
                if let Ok(config) = toml::from_str(&content) {
                    return config;
                }
            }
        }
        Self::default()
    }

    pub fn run_hook(&self, hook_name: &str, context: &HashMap<String, String>) -> bool {
        let hook = match hook_name {
            "post_edit" => &self.post_edit,
            "pre_commit" => &self.pre_commit,
            "post_commit" => &self.post_commit,
            _ => return true,
        };

        if let Some(hook) = hook {
            let mut cmd = hook.command.clone();
            for (key, value) in context {
                cmd = cmd.replace(&format!("{{{}}}", key), value);
            }

            let output = Command::new("sh")
                .arg("-c")
                .arg(&cmd)
                .output();

            match output {
                Ok(out) if out.status.success() => true,
                Ok(_) if hook.on_failure == "block" => {
                    eprintln!("Hook '{}' failed and blocked the action", hook_name);
                    false
                }
                Ok(_) => {
                    eprintln!("Hook '{}' failed (warning)", hook_name);
                    true
                }
                Err(e) => {
                    eprintln!("Hook '{}' error: {}", hook_name, e);
                    hook.on_failure != "block"
                }
            }
        } else {
            true
        }
    }
}

fn hooks_path() -> PathBuf {
    dirs::home_dir()
        .unwrap_or_else(|| PathBuf::from("~"))
        .join(".localcode")
        .join("hooks.toml")
}
