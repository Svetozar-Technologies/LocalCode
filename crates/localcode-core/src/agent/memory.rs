use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::path::{Path, PathBuf};

use crate::CoreResult;

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct GlobalMemory {
    pub preferences: HashMap<String, String>,
    pub project_memories: HashMap<String, ProjectMemory>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SessionSummary {
    pub timestamp: u64,
    pub task: String,
    pub files_modified: Vec<String>,
    pub tasks_completed: Vec<String>,
    pub summary: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct ProjectMemory {
    pub framework: Option<String>,
    pub build_system: Option<String>,
    pub language: Option<String>,
    pub test_command: Option<String>,
    pub build_command: Option<String>,
    pub lint_command: Option<String>,
    pub file_tree_summary: Option<String>,
    pub conventions: Vec<String>,
    pub learned: Vec<String>,
    pub sessions: Vec<SessionSummary>,
    pub last_indexed: Option<u64>,
}

pub struct MemoryManager {
    global_path: PathBuf,
    global: GlobalMemory,
}

impl MemoryManager {
    pub fn new() -> Self {
        let global_path = dirs::home_dir()
            .unwrap_or_else(|| PathBuf::from("~"))
            .join(".localcode")
            .join("memory")
            .join("global.json");

        let global = Self::load_global(&global_path).unwrap_or_default();

        Self {
            global_path,
            global,
        }
    }

    fn load_global(path: &Path) -> CoreResult<GlobalMemory> {
        if path.exists() {
            let content = std::fs::read_to_string(path)?;
            let memory: GlobalMemory = serde_json::from_str(&content)?;
            Ok(memory)
        } else {
            Ok(GlobalMemory::default())
        }
    }

    pub fn save(&self) -> CoreResult<()> {
        if let Some(parent) = self.global_path.parent() {
            std::fs::create_dir_all(parent)?;
        }
        let content = serde_json::to_string_pretty(&self.global)?;
        std::fs::write(&self.global_path, content)?;
        Ok(())
    }

    pub fn get_project_memory(&self, project_path: &str) -> Option<&ProjectMemory> {
        self.global.project_memories.get(project_path)
    }

    pub fn get_project_memory_mut(&mut self, project_path: &str) -> &mut ProjectMemory {
        self.global
            .project_memories
            .entry(project_path.to_string())
            .or_default()
    }

    pub fn set_project_memory(&mut self, project_path: &str, memory: ProjectMemory) {
        self.global
            .project_memories
            .insert(project_path.to_string(), memory);
    }

    pub fn add_learned(&mut self, project_path: &str, fact: &str) {
        let memory = self
            .global
            .project_memories
            .entry(project_path.to_string())
            .or_default();

        if !memory.learned.contains(&fact.to_string()) {
            memory.learned.push(fact.to_string());
        }
    }

    pub fn add_convention(&mut self, project_path: &str, convention: &str) {
        let memory = self
            .global
            .project_memories
            .entry(project_path.to_string())
            .or_default();

        if !memory.conventions.contains(&convention.to_string()) {
            memory.conventions.push(convention.to_string());
        }
    }

    pub fn save_session_summary(&mut self, project_path: &str, summary: SessionSummary) {
        let memory = self
            .global
            .project_memories
            .entry(project_path.to_string())
            .or_default();

        memory.sessions.push(summary);
        // Keep only last 10 sessions
        if memory.sessions.len() > 10 {
            memory.sessions.drain(0..memory.sessions.len() - 10);
        }
    }

    pub fn set_preference(&mut self, key: &str, value: &str) {
        self.global
            .preferences
            .insert(key.to_string(), value.to_string());
    }

    pub fn get_preference(&self, key: &str) -> Option<&String> {
        self.global.preferences.get(key)
    }

    /// Read LOCALCODE.md from project root
    pub fn read_project_file(project_path: &str) -> Option<String> {
        let path = Path::new(project_path).join("LOCALCODE.md");
        std::fs::read_to_string(path).ok()
    }

    /// Read .localcode/rules.md from project root
    pub fn read_rules_file(project_path: &str) -> Option<String> {
        let path = Path::new(project_path).join(".localcode").join("rules.md");
        std::fs::read_to_string(path).ok()
    }

    /// Auto-discover project characteristics by examining files
    pub fn auto_discover_project(&mut self, project_path: &str) {
        let path = Path::new(project_path);

        // Generate file tree first (before borrowing memory mutably)
        let tree_summary = generate_file_tree_summary(project_path);

        let memory = self
            .global
            .project_memories
            .entry(project_path.to_string())
            .or_default();

        // Detect framework/language/build system from project files
        if path.join("Cargo.toml").exists() {
            memory.language = Some("Rust".to_string());
            memory.build_system = Some("Cargo".to_string());
            if memory.build_command.is_none() {
                memory.build_command = Some("cargo build".to_string());
            }
            if memory.test_command.is_none() {
                memory.test_command = Some("cargo test".to_string());
            }
            if memory.lint_command.is_none() {
                memory.lint_command = Some("cargo clippy".to_string());
            }
        }

        if path.join("package.json").exists() {
            if memory.language.is_none() {
                memory.language = Some("TypeScript/JavaScript".to_string());
            }
            memory.build_system = Some(
                if path.join("bun.lockb").exists() {
                    "Bun".to_string()
                } else if path.join("pnpm-lock.yaml").exists() {
                    "pnpm".to_string()
                } else if path.join("yarn.lock").exists() {
                    "Yarn".to_string()
                } else {
                    "npm".to_string()
                },
            );

            // Detect framework from package.json
            if let Ok(content) = std::fs::read_to_string(path.join("package.json")) {
                if content.contains("\"next\"") {
                    memory.framework = Some("Next.js".to_string());
                } else if content.contains("\"react\"") {
                    memory.framework = Some("React".to_string());
                } else if content.contains("\"vue\"") {
                    memory.framework = Some("Vue".to_string());
                } else if content.contains("\"svelte\"") {
                    memory.framework = Some("Svelte".to_string());
                } else if content.contains("\"angular\"") {
                    memory.framework = Some("Angular".to_string());
                } else if content.contains("\"express\"") || content.contains("\"fastify\"") {
                    memory.framework = Some("Node.js Server".to_string());
                }

                // Check for scripts
                if let Ok(pkg) = serde_json::from_str::<serde_json::Value>(&content) {
                    if let Some(scripts) = pkg.get("scripts").and_then(|s| s.as_object()) {
                        if memory.test_command.is_none() && scripts.contains_key("test") {
                            let pm = memory.build_system.as_deref().unwrap_or("npm");
                            memory.test_command = Some(format!("{} test", pm.to_lowercase()));
                        }
                        if memory.build_command.is_none() && scripts.contains_key("build") {
                            let pm = memory.build_system.as_deref().unwrap_or("npm");
                            memory.build_command = Some(format!("{} run build", pm.to_lowercase()));
                        }
                        if memory.lint_command.is_none() && scripts.contains_key("lint") {
                            let pm = memory.build_system.as_deref().unwrap_or("npm");
                            memory.lint_command = Some(format!("{} run lint", pm.to_lowercase()));
                        }
                    }
                }
            }
        }

        if path.join("pyproject.toml").exists() || path.join("setup.py").exists() {
            if memory.language.is_none() {
                memory.language = Some("Python".to_string());
            }
            if path.join("pyproject.toml").exists() {
                memory.build_system = Some("Poetry/pyproject".to_string());
            }
            if memory.test_command.is_none() {
                if path.join("pytest.ini").exists() || path.join("pyproject.toml").exists() {
                    memory.test_command = Some("pytest".to_string());
                } else {
                    memory.test_command = Some("python -m pytest".to_string());
                }
            }
        }

        if path.join("go.mod").exists() {
            if memory.language.is_none() {
                memory.language = Some("Go".to_string());
            }
            memory.build_system = Some("Go Modules".to_string());
            if memory.test_command.is_none() {
                memory.test_command = Some("go test ./...".to_string());
            }
            if memory.build_command.is_none() {
                memory.build_command = Some("go build ./...".to_string());
            }
        }

        if path.join("Makefile").exists() || path.join("CMakeLists.txt").exists() {
            if memory.build_system.is_none() {
                memory.build_system = Some(
                    if path.join("CMakeLists.txt").exists() {
                        "CMake"
                    } else {
                        "Make"
                    }
                    .to_string(),
                );
            }
        }

        // Detect tauri
        if path.join("tauri.conf.json").exists() || path.join("src-tauri").exists() {
            if memory.framework.is_none() || memory.framework.as_deref() == Some("React") {
                memory.framework = Some(format!(
                    "Tauri + {}",
                    memory.framework.as_deref().unwrap_or("React")
                ));
            }
        }

        memory.file_tree_summary = Some(tree_summary);
        memory.last_indexed = Some(now_secs());
    }

    /// Build rich context string from memory for agent system prompt
    pub fn build_context(&self, project_path: &str) -> String {
        let mut ctx = String::new();

        // Project file (LOCALCODE.md)
        if let Some(content) = Self::read_project_file(project_path) {
            ctx.push_str("# Project Instructions (LOCALCODE.md)\n");
            ctx.push_str(&content);
            ctx.push('\n');
        }

        // Project rules (.localcode/rules.md)
        if let Some(rules) = Self::read_rules_file(project_path) {
            ctx.push_str("\n# Project Rules\n");
            ctx.push_str(&rules);
            ctx.push('\n');
        }

        if let Some(memory) = self.get_project_memory(project_path) {
            // Project info
            let mut project_info = Vec::new();
            if let Some(ref lang) = memory.language {
                project_info.push(format!("Language: {}", lang));
            }
            if let Some(ref fw) = memory.framework {
                project_info.push(format!("Framework: {}", fw));
            }
            if let Some(ref bs) = memory.build_system {
                project_info.push(format!("Build system: {}", bs));
            }
            if let Some(ref tc) = memory.test_command {
                project_info.push(format!("Test: `{}`", tc));
            }
            if let Some(ref bc) = memory.build_command {
                project_info.push(format!("Build: `{}`", bc));
            }
            if let Some(ref lc) = memory.lint_command {
                project_info.push(format!("Lint: `{}`", lc));
            }
            if !project_info.is_empty() {
                ctx.push_str("\n# Project Info\n");
                for info in &project_info {
                    ctx.push_str(&format!("- {}\n", info));
                }
            }

            // File tree (cap at 500 chars to avoid wasting tokens on large dirs)
            if let Some(ref tree) = memory.file_tree_summary {
                if tree.len() <= 500 {
                    ctx.push_str("\n# Project Structure\n```\n");
                    ctx.push_str(tree);
                    ctx.push_str("\n```\n");
                }
            }

            // Conventions
            if !memory.conventions.is_empty() {
                ctx.push_str("\n# Coding Conventions\n");
                for conv in &memory.conventions {
                    ctx.push_str(&format!("- {}\n", conv));
                }
            }

            // Learned facts
            if !memory.learned.is_empty() {
                ctx.push_str("\n# Learned about this project\n");
                for fact in &memory.learned {
                    ctx.push_str(&format!("- {}\n", fact));
                }
            }

            // Last session summary
            if let Some(last) = memory.sessions.last() {
                ctx.push_str("\n# Previous Session\n");
                ctx.push_str(&format!("Task: {}\n", last.task));
                if !last.files_modified.is_empty() {
                    ctx.push_str(&format!(
                        "Files modified: {}\n",
                        last.files_modified.join(", ")
                    ));
                }
                ctx.push_str(&format!("Summary: {}\n", last.summary));
            }
        }

        ctx
    }

    /// Check if project should be re-indexed (stale > max_age_secs)
    pub fn needs_reindex(&self, project_path: &str, max_age_secs: u64) -> bool {
        if let Some(memory) = self.get_project_memory(project_path) {
            if let Some(last) = memory.last_indexed {
                return now_secs() - last > max_age_secs;
            }
        }
        true
    }
}

impl Default for MemoryManager {
    fn default() -> Self {
        Self::new()
    }
}

/// Generate a compact file tree summary for a project
fn generate_file_tree_summary(project_path: &str) -> String {
    let path = Path::new(project_path);
    let mut lines = Vec::new();

    if let Ok(entries) = std::fs::read_dir(path) {
        let mut items: Vec<_> = entries
            .filter_map(|e| e.ok())
            .filter(|e| {
                let name = e.file_name().to_string_lossy().to_string();
                !name.starts_with('.')
                    && name != "node_modules"
                    && name != "target"
                    && name != "dist"
                    && name != "__pycache__"
            })
            .collect();

        items.sort_by(|a, b| {
            let a_dir = a.file_type().map(|t| t.is_dir()).unwrap_or(false);
            let b_dir = b.file_type().map(|t| t.is_dir()).unwrap_or(false);
            b_dir
                .cmp(&a_dir)
                .then_with(|| a.file_name().cmp(&b.file_name()))
        });

        for item in items.iter().take(30) {
            let name = item.file_name().to_string_lossy().to_string();
            let is_dir = item.file_type().map(|t| t.is_dir()).unwrap_or(false);
            if is_dir {
                lines.push(format!("{}/", name));
                // Add immediate children for key dirs
                if let Ok(children) = std::fs::read_dir(item.path()) {
                    let mut child_names: Vec<String> = children
                        .filter_map(|e| e.ok())
                        .filter(|e| {
                            let n = e.file_name().to_string_lossy().to_string();
                            !n.starts_with('.')
                        })
                        .map(|e| {
                            let n = e.file_name().to_string_lossy().to_string();
                            let d = e.file_type().map(|t| t.is_dir()).unwrap_or(false);
                            if d {
                                format!("  {}/", n)
                            } else {
                                format!("  {}", n)
                            }
                        })
                        .take(10)
                        .collect();
                    child_names.sort();
                    lines.extend(child_names);
                }
            } else {
                lines.push(name);
            }
        }
    }

    lines.join("\n")
}

fn now_secs() -> u64 {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap_or_default()
        .as_secs()
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::TempDir;

    /// Helper: create a MemoryManager that stores its global.json in a temp directory
    fn memory_manager_in(dir: &Path) -> MemoryManager {
        let global_path = dir.join("memory").join("global.json");
        MemoryManager {
            global_path,
            global: GlobalMemory::default(),
        }
    }

    #[test]
    fn test_auto_discover_rust_project() {
        let dir = TempDir::new().unwrap();
        // Create a Cargo.toml so the project is detected as Rust
        std::fs::write(dir.path().join("Cargo.toml"), "[package]\nname = \"test\"").unwrap();

        let mut mm = memory_manager_in(dir.path());
        mm.auto_discover_project(dir.path().to_str().unwrap());

        let mem = mm.get_project_memory(dir.path().to_str().unwrap()).unwrap();
        assert_eq!(mem.language.as_deref(), Some("Rust"));
        assert_eq!(mem.build_system.as_deref(), Some("Cargo"));
        assert_eq!(mem.test_command.as_deref(), Some("cargo test"));
    }

    #[test]
    fn test_auto_discover_node_project() {
        let dir = TempDir::new().unwrap();
        // Create a package.json so the project is detected as TypeScript/JavaScript
        std::fs::write(
            dir.path().join("package.json"),
            r#"{"name":"test","dependencies":{}}"#,
        )
        .unwrap();

        let mut mm = memory_manager_in(dir.path());
        mm.auto_discover_project(dir.path().to_str().unwrap());

        let mem = mm.get_project_memory(dir.path().to_str().unwrap()).unwrap();
        let lang = mem.language.as_deref().unwrap();
        assert!(
            lang.contains("TypeScript") || lang.contains("JavaScript"),
            "Expected TypeScript or JavaScript, got: {}",
            lang
        );
    }

    #[test]
    fn test_build_context_includes_framework() {
        let dir = TempDir::new().unwrap();
        let project = dir.path().to_str().unwrap();

        let mut mm = memory_manager_in(dir.path());
        // Manually set a framework on the project memory
        let mem = mm.get_project_memory_mut(project);
        mem.framework = Some("Next.js".to_string());
        mem.language = Some("TypeScript".to_string());

        let ctx = mm.build_context(project);
        assert!(
            ctx.contains("Next.js"),
            "build_context should contain the framework name"
        );
    }

    #[test]
    fn test_save_and_load() {
        let dir = TempDir::new().unwrap();
        let project = "/tmp/fake_project";

        let mut mm = memory_manager_in(dir.path());
        mm.set_preference("theme", "dark");
        let mem = mm.get_project_memory_mut(project);
        mem.language = Some("Rust".to_string());
        mem.conventions.push("use snake_case".to_string());
        mm.save().unwrap();

        // Reload from the same path
        let loaded = MemoryManager::load_global(&mm.global_path).unwrap();
        assert_eq!(loaded.preferences.get("theme").map(|s| s.as_str()), Some("dark"));
        let loaded_mem = loaded.project_memories.get(project).unwrap();
        assert_eq!(loaded_mem.language.as_deref(), Some("Rust"));
        assert!(loaded_mem.conventions.contains(&"use snake_case".to_string()));
    }

    #[test]
    fn test_session_summary_limit() {
        let dir = TempDir::new().unwrap();
        let project = "/tmp/some_project";

        let mut mm = memory_manager_in(dir.path());

        // Add 15 session summaries
        for i in 0..15 {
            mm.save_session_summary(
                project,
                SessionSummary {
                    timestamp: i as u64,
                    task: format!("task {}", i),
                    files_modified: vec![],
                    tasks_completed: vec![],
                    summary: format!("summary {}", i),
                },
            );
        }

        let mem = mm.get_project_memory(project).unwrap();
        assert_eq!(mem.sessions.len(), 10, "Only the last 10 sessions should be kept");
        // The oldest remaining session should be #5 (0-indexed original timestamps 5..14)
        assert_eq!(mem.sessions[0].timestamp, 5);
        assert_eq!(mem.sessions[9].timestamp, 14);
    }
}
