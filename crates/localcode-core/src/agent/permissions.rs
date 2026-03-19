use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PermissionConfig {
    #[serde(default)]
    pub allowed_paths: Vec<String>,
    #[serde(default)]
    pub denied_paths: Vec<String>,
    #[serde(default)]
    pub allowed_commands: Vec<String>,
    #[serde(default)]
    pub denied_commands: Vec<String>,
    #[serde(default = "default_auto_approve")]
    pub auto_approve_reads: bool,
    #[serde(default)]
    pub auto_approve_writes: bool,
    #[serde(default)]
    pub auto_approve_commands: bool,
}

fn default_auto_approve() -> bool {
    true
}

impl Default for PermissionConfig {
    fn default() -> Self {
        Self {
            allowed_paths: Vec::new(),
            denied_paths: vec![
                "~/.ssh".to_string(),
                "~/.gnupg".to_string(),
                "~/.aws/credentials".to_string(),
            ],
            allowed_commands: Vec::new(),
            denied_commands: vec![
                "rm -rf /".to_string(),
                "sudo rm".to_string(),
            ],
            auto_approve_reads: true,
            auto_approve_writes: false,
            auto_approve_commands: false,
        }
    }
}

#[derive(Debug, Clone, PartialEq)]
pub enum Permission {
    Allow,
    Deny,
    Ask,
}

impl PermissionConfig {
    pub fn check_file_read(&self, path: &str) -> Permission {
        if self.is_denied_path(path) {
            return Permission::Deny;
        }
        if self.auto_approve_reads {
            return Permission::Allow;
        }
        Permission::Ask
    }

    pub fn check_file_write(&self, path: &str) -> Permission {
        if self.is_denied_path(path) {
            return Permission::Deny;
        }
        if self.auto_approve_writes {
            return Permission::Allow;
        }
        if !self.allowed_paths.is_empty() && self.is_allowed_path(path) {
            return Permission::Allow;
        }
        Permission::Ask
    }

    pub fn check_command(&self, command: &str) -> Permission {
        for denied in &self.denied_commands {
            if command.contains(denied) {
                return Permission::Deny;
            }
        }
        if self.auto_approve_commands {
            return Permission::Allow;
        }
        if !self.allowed_commands.is_empty() {
            for allowed in &self.allowed_commands {
                if command.starts_with(allowed) {
                    return Permission::Allow;
                }
            }
        }
        Permission::Ask
    }

    fn is_denied_path(&self, path: &str) -> bool {
        let expanded = expand_tilde(path);
        self.denied_paths.iter().any(|denied| {
            let denied_expanded = expand_tilde(denied);
            expanded.starts_with(&denied_expanded)
        })
    }

    fn is_allowed_path(&self, path: &str) -> bool {
        let expanded = expand_tilde(path);
        self.allowed_paths.iter().any(|allowed| {
            let allowed_expanded = expand_tilde(allowed);
            expanded.starts_with(&allowed_expanded)
        })
    }
}

fn expand_tilde(path: &str) -> String {
    if path.starts_with('~') {
        if let Some(home) = dirs::home_dir() {
            return path.replacen('~', &home.to_string_lossy(), 1);
        }
    }
    path.to_string()
}
