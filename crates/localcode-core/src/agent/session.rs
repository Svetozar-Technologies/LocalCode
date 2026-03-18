use serde::{Deserialize, Serialize};
use std::path::{Path, PathBuf};

use crate::CoreResult;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SessionState {
    pub project_path: String,
    pub started_at: u64,
    pub files_modified: Vec<String>,
    pub tasks_completed: Vec<String>,
    pub conversation_summary: String,
}

impl SessionState {
    pub fn new(project_path: &str) -> Self {
        Self {
            project_path: project_path.to_string(),
            started_at: now_secs(),
            files_modified: Vec::new(),
            tasks_completed: Vec::new(),
            conversation_summary: String::new(),
        }
    }

    pub fn add_file_modified(&mut self, path: &str) {
        if !self.files_modified.contains(&path.to_string()) {
            self.files_modified.push(path.to_string());
        }
    }

    pub fn add_task_completed(&mut self, task: &str) {
        self.tasks_completed.push(task.to_string());
    }

    /// Get the session file path for a project
    fn session_path(project_path: &str) -> PathBuf {
        let hash = simple_hash(project_path);
        dirs::home_dir()
            .unwrap_or_else(|| PathBuf::from("~"))
            .join(".localcode")
            .join("projects")
            .join(hash)
            .join("last_session.json")
    }

    /// Save session state to disk
    pub fn save(&self) -> CoreResult<()> {
        let path = Self::session_path(&self.project_path);
        if let Some(parent) = path.parent() {
            std::fs::create_dir_all(parent)?;
        }
        let content = serde_json::to_string_pretty(self)?;
        std::fs::write(&path, content)?;
        Ok(())
    }

    /// Load last session state for a project
    pub fn load(project_path: &str) -> CoreResult<Option<Self>> {
        let path = Self::session_path(project_path);
        if path.exists() {
            let content = std::fs::read_to_string(&path)?;
            let session: SessionState = serde_json::from_str(&content)?;
            Ok(Some(session))
        } else {
            Ok(None)
        }
    }
}

/// Simple hash function for project path (avoids sha2 dependency)
fn simple_hash(input: &str) -> String {
    // Use a simple FNV-1a hash to create a short stable identifier
    let mut hash: u64 = 0xcbf29ce484222325;
    for byte in input.bytes() {
        hash ^= byte as u64;
        hash = hash.wrapping_mul(0x100000001b3);
    }
    format!("{:016x}", hash)
}

fn now_secs() -> u64 {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap_or_default()
        .as_secs()
}
