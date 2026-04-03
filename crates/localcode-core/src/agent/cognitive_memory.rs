//! Cognitive Memory — Engram-backed long-term memory for LocalCode agents.
//!
//! Replaces the flat JSON MemoryManager with Engram's cognitive memory system:
//! - 5 memory types (semantic, episodic, procedural, prospective, working)
//! - HNSW vector search + BM25 hybrid retrieval
//! - Cognitive mesh graph with 7 link types
//! - WAL-based durability with crash recovery
//! - Ebbinghaus-inspired memory decay

use engram_core::*;
use std::path::{Path, PathBuf};
use std::sync::Arc;

use super::memory::{MemoryManager, ProjectMemory, SessionSummary};
use crate::CoreResult;

/// HuggingFace URLs for all-MiniLM-L6-v2 ONNX model
const MODEL_URL: &str = "https://huggingface.co/sentence-transformers/all-MiniLM-L6-v2/resolve/main/onnx/model.onnx";
const TOKENIZER_URL: &str = "https://huggingface.co/sentence-transformers/all-MiniLM-L6-v2/resolve/main/tokenizer.json";

/// Cognitive memory system wrapping Engram for AI agent long-term memory.
///
/// Provides both the classic `MemoryManager` interface (for backward compat)
/// and the full Engram API for cognitive memory operations.
pub struct CognitiveMemory {
    /// The Engram instance powering all memory operations
    pub engram: Arc<Engram>,
    /// Agent namespace for this project
    agent_ns: AgentNamespace,
    /// Legacy memory manager for project auto-discovery and session tracking
    legacy: MemoryManager,
    /// Project path this memory is bound to
    project_path: String,
}

impl CognitiveMemory {
    /// Open cognitive memory for a project. Data stored at ~/.localcode/engram/
    pub fn open(project_path: &str) -> CoreResult<Self> {
        let data_dir = dirs::home_dir()
            .unwrap_or_else(|| PathBuf::from("~"))
            .join(".localcode")
            .join("engram");

        let engram = Engram::open(&data_dir)
            .map_err(|e| crate::CoreError::Other(format!("Engram init failed: {}", e)))?;

        // Register agent namespace using project path as identifier
        let agent_name = Self::project_agent_name(project_path);
        let agent_ns = engram.register_agent(&agent_name);

        let legacy = MemoryManager::new();

        Ok(Self {
            engram: Arc::new(engram),
            agent_ns,
            legacy,
            project_path: project_path.to_string(),
        })
    }

    /// Create an in-memory instance for testing
    #[cfg(test)]
    pub fn in_memory(project_path: &str) -> CoreResult<Self> {
        let engram = Engram::in_memory()
            .map_err(|e| crate::CoreError::Other(format!("Engram init failed: {}", e)))?;
        let agent_name = Self::project_agent_name(project_path);
        let agent_ns = engram.register_agent(&agent_name);

        Ok(Self {
            engram: Arc::new(engram),
            agent_ns,
            legacy: MemoryManager::new(),
            project_path: project_path.to_string(),
        })
    }

    /// Derive a stable agent name from project path
    fn project_agent_name(project_path: &str) -> String {
        let path = Path::new(project_path);
        let name = path.file_name()
            .and_then(|n| n.to_str())
            .unwrap_or("default");
        format!("project:{}", name)
    }

    /// Agent ID for this project
    pub fn agent_id(&self) -> AgentId {
        self.agent_ns.agent_id
    }

    /// Get shared Engram reference (for tools and other components)
    pub fn engram(&self) -> Arc<Engram> {
        self.engram.clone()
    }

    // ── Store Operations ────────────────────────────────────────────────

    /// Store a learned fact as a semantic memory
    pub fn store_fact(&self, content: &str) -> CoreResult<MemoryId> {
        let node = MemoryNode::new(
            self.agent_ns.agent_id,
            MemoryType::Semantic,
            content,
        )
        .tag("learned");

        self.engram.store(node)
            .map_err(|e| crate::CoreError::Other(format!("Store failed: {}", e)))
    }

    /// Store a coding convention as a high-confidence semantic memory
    pub fn store_convention(&self, content: &str) -> CoreResult<MemoryId> {
        let node = MemoryNode::new(
            self.agent_ns.agent_id,
            MemoryType::Semantic,
            content,
        )
        .tag("convention")
        .with_visibility(Visibility::Global);

        self.engram.store(node)
            .map_err(|e| crate::CoreError::Other(format!("Store failed: {}", e)))
    }

    /// Store a session as an episodic memory
    pub fn store_session(&self, summary: &SessionSummary) -> CoreResult<MemoryId> {
        let content = format!(
            "Session: {}\nFiles: {}\nSummary: {}",
            summary.task,
            summary.files_modified.join(", "),
            summary.summary
        );

        let node = MemoryNode::new(
            self.agent_ns.agent_id,
            MemoryType::Episodic,
            content,
        )
        .tag("session");

        self.engram.store(node)
            .map_err(|e| crate::CoreError::Other(format!("Store failed: {}", e)))
    }

    /// Store a procedural memory (how to do something)
    pub fn store_procedure(&self, content: &str) -> CoreResult<MemoryId> {
        let node = MemoryNode::new(
            self.agent_ns.agent_id,
            MemoryType::Procedural,
            content,
        )
        .tag("procedure");

        self.engram.store(node)
            .map_err(|e| crate::CoreError::Other(format!("Store failed: {}", e)))
    }

    /// Store project metadata (framework, language, etc.) as semantic memory
    pub fn store_project_info(&self, memory: &ProjectMemory) -> CoreResult<()> {
        let mut facts = Vec::new();

        if let Some(ref lang) = memory.language {
            facts.push(format!("Project language: {}", lang));
        }
        if let Some(ref fw) = memory.framework {
            facts.push(format!("Project framework: {}", fw));
        }
        if let Some(ref bs) = memory.build_system {
            facts.push(format!("Build system: {}", bs));
        }
        if let Some(ref tc) = memory.test_command {
            facts.push(format!("Test command: {}", tc));
        }
        if let Some(ref bc) = memory.build_command {
            facts.push(format!("Build command: {}", bc));
        }
        if let Some(ref lc) = memory.lint_command {
            facts.push(format!("Lint command: {}", lc));
        }

        for fact in facts {
            let node = MemoryNode::new(
                self.agent_ns.agent_id,
                MemoryType::Semantic,
                fact,
            )
            .tag("project-info")
            .with_visibility(Visibility::Global);

            self.engram.store(node)
                .map_err(|e| crate::CoreError::Other(format!("Store failed: {}", e)))?;
        }

        Ok(())
    }

    // ── Recall Operations ───────────────────────────────────────────────

    /// Semantic search across all memories for this project
    pub fn recall(&self, query: &str, limit: usize) -> Vec<ScoredMemory> {
        self.engram.recall(
            RecallQuery::new(query)
                .with_agent(self.agent_ns.agent_id)
                .with_limit(limit),
        )
    }

    /// Recall only conventions
    pub fn recall_conventions(&self, limit: usize) -> Vec<ScoredMemory> {
        self.engram.recall(
            RecallQuery::new("coding convention style rule")
                .with_agent(self.agent_ns.agent_id)
                .with_tags(vec!["convention".into()])
                .with_limit(limit),
        )
    }

    /// Recall only learned facts
    pub fn recall_facts(&self, query: &str, limit: usize) -> Vec<ScoredMemory> {
        self.engram.recall(
            RecallQuery::new(query)
                .with_agent(self.agent_ns.agent_id)
                .with_tags(vec!["learned".into()])
                .with_limit(limit),
        )
    }

    /// Recall recent sessions
    pub fn recall_sessions(&self, limit: usize) -> Vec<ScoredMemory> {
        self.engram.recall(
            RecallQuery::new("session task work completed")
                .with_agent(self.agent_ns.agent_id)
                .with_tags(vec!["session".into()])
                .with_limit(limit),
        )
    }

    // ── Working Memory ──────────────────────────────────────────────────

    /// Push content into working memory (auto-expires)
    pub fn push_working(&self, content: &str, ttl_seconds: u64) -> MemoryId {
        self.engram.push_working(self.agent_ns.agent_id, content, ttl_seconds)
    }

    /// Get current working memory context
    pub fn working_context(&self) -> Option<String> {
        self.engram.working_context(&self.agent_ns.agent_id)
    }

    // ── Context Building ────────────────────────────────────────────────

    /// Build rich context string from cognitive memory (replaces MemoryManager::build_context)
    pub fn build_context(&mut self) -> String {
        let mut ctx = String::new();

        // 1. Project file (LOCALCODE.md) — from legacy
        if let Some(content) = MemoryManager::read_project_file(&self.project_path) {
            ctx.push_str("# Project Instructions (LOCALCODE.md)\n");
            ctx.push_str(&content);
            ctx.push('\n');
        }

        // 2. Project rules — from legacy
        if let Some(rules) = MemoryManager::read_rules_file(&self.project_path) {
            ctx.push_str("\n# Project Rules\n");
            ctx.push_str(&rules);
            ctx.push('\n');
        }

        // 3. Project info from Engram (semantic memories with tag "project-info")
        let project_info = self.engram.recall(
            RecallQuery::new("project language framework build test")
                .with_agent(self.agent_ns.agent_id)
                .with_tags(vec!["project-info".into()])
                .with_limit(10),
        );
        if !project_info.is_empty() {
            ctx.push_str("\n# Project Info\n");
            for info in &project_info {
                ctx.push_str(&format!("- {}\n", info.memory.content));
            }
        }

        // 4. Conventions from Engram
        let conventions = self.recall_conventions(10);
        if !conventions.is_empty() {
            ctx.push_str("\n# Coding Conventions\n");
            for conv in &conventions {
                ctx.push_str(&format!("- {}\n", conv.memory.content));
            }
        }

        // 5. Learned facts (top relevant)
        let facts = self.recall_facts("important project knowledge", 10);
        if !facts.is_empty() {
            ctx.push_str("\n# Learned About This Project\n");
            for fact in &facts {
                ctx.push_str(&format!("- {}\n", fact.memory.content));
            }
        }

        // 6. Last session from Engram
        let sessions = self.recall_sessions(1);
        if let Some(last) = sessions.first() {
            ctx.push_str("\n# Previous Session\n");
            ctx.push_str(&format!("{}\n", last.memory.content));
        }

        // 7. Working memory (active context)
        if let Some(wm) = self.working_context() {
            ctx.push_str("\n# Active Context\n");
            ctx.push_str(&wm);
            ctx.push('\n');
        }

        ctx
    }

    /// Initialize from legacy memory: auto-discover + migrate existing data
    pub fn initialize(&mut self) -> CoreResult<()> {
        // Auto-discover project if stale
        if self.legacy.needs_reindex(&self.project_path, 3600) {
            self.legacy.auto_discover_project(&self.project_path);
            let _ = self.legacy.save();
        }

        // Migrate project metadata into Engram (if not already there)
        if let Some(project_mem) = self.legacy.get_project_memory(&self.project_path).cloned() {
            // Only migrate if we have no project-info memories yet
            let existing = self.engram.recall(
                RecallQuery::new("project language framework")
                    .with_agent(self.agent_ns.agent_id)
                    .with_tags(vec!["project-info".into()])
                    .with_limit(1),
            );
            if existing.is_empty() {
                self.store_project_info(&project_mem)?;

                // Migrate conventions
                for conv in &project_mem.conventions {
                    self.store_convention(conv)?;
                }

                // Migrate learned facts
                for fact in &project_mem.learned {
                    self.store_fact(fact)?;
                }

                // Migrate sessions
                for session in &project_mem.sessions {
                    self.store_session(session)?;
                }
            }
        }

        Ok(())
    }

    /// Forget a memory
    pub fn forget(&self, id: &MemoryId) -> CoreResult<()> {
        self.engram.forget(id)
            .map_err(|e| crate::CoreError::Other(format!("Forget failed: {}", e)))
    }

    /// Run consolidation (reflect) to improve memory quality
    pub fn reflect(&self) -> CoreResult<()> {
        self.engram.reflect(&self.agent_ns.agent_id)
            .map_err(|e| crate::CoreError::Other(format!("Reflect failed: {}", e)))?;
        Ok(())
    }

    /// Get memory statistics
    pub fn stats(&self) -> Option<AgentStats> {
        self.engram.agent_stats(&self.agent_ns.agent_id)
    }

    /// Snapshot indices to disk (call on graceful shutdown)
    pub fn snapshot(&self) -> CoreResult<()> {
        self.engram.snapshot_indices()
            .map_err(|e| crate::CoreError::Other(format!("Snapshot failed: {}", e)))
    }

    /// Get the embedding model directory path
    pub fn model_dir() -> PathBuf {
        dirs::home_dir()
            .unwrap_or_else(|| PathBuf::from("~"))
            .join(".localcode")
            .join("models")
            .join("embeddings")
    }

    /// Check if ONNX embedding model is already downloaded
    pub fn has_embedding_model() -> bool {
        let dir = Self::model_dir();
        dir.join("model.onnx").exists() && dir.join("tokenizer.json").exists()
    }

    /// Download the all-MiniLM-L6-v2 ONNX embedding model (~80MB).
    /// Returns the model directory path on success.
    pub async fn download_embedding_model<F: Fn(u64, u64)>(
        progress: F,
    ) -> CoreResult<PathBuf> {
        let dir = Self::model_dir();
        std::fs::create_dir_all(&dir)
            .map_err(|e| crate::CoreError::Other(format!("Create model dir: {}", e)))?;

        let client = reqwest::Client::new();

        // Download tokenizer.json first (small, ~711KB)
        let tokenizer_path = dir.join("tokenizer.json");
        if !tokenizer_path.exists() {
            let resp = client.get(TOKENIZER_URL).send().await
                .map_err(|e| crate::CoreError::Other(format!("Download tokenizer: {}", e)))?;
            let bytes = resp.bytes().await
                .map_err(|e| crate::CoreError::Other(format!("Read tokenizer: {}", e)))?;
            std::fs::write(&tokenizer_path, &bytes)
                .map_err(|e| crate::CoreError::Other(format!("Write tokenizer: {}", e)))?;
        }

        // Download model.onnx (~80MB) with progress
        let model_path = dir.join("model.onnx");
        if !model_path.exists() {
            let resp = client.get(MODEL_URL).send().await
                .map_err(|e| crate::CoreError::Other(format!("Download model: {}", e)))?;
            let total = resp.content_length().unwrap_or(80_000_000);
            let mut downloaded: u64 = 0;

            let mut file = std::fs::File::create(&model_path)
                .map_err(|e| crate::CoreError::Other(format!("Create model file: {}", e)))?;

            let mut stream = resp.bytes_stream();
            use futures::StreamExt;
            while let Some(chunk) = stream.next().await {
                let chunk = chunk
                    .map_err(|e| crate::CoreError::Other(format!("Download chunk: {}", e)))?;
                std::io::Write::write_all(&mut file, &chunk)
                    .map_err(|e| crate::CoreError::Other(format!("Write chunk: {}", e)))?;
                downloaded += chunk.len() as u64;
                progress(downloaded, total);
            }
        }

        Ok(dir)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_store_and_recall_fact() {
        let mem = CognitiveMemory::in_memory("/tmp/test_project").unwrap();

        mem.store_fact("The project uses PostgreSQL for the main database").unwrap();
        mem.store_fact("API endpoints follow REST conventions").unwrap();

        let results = mem.recall("database", 5);
        assert!(!results.is_empty());
        assert!(results[0].memory.content.as_str().contains("PostgreSQL"));
    }

    #[test]
    fn test_store_and_recall_convention() {
        let mem = CognitiveMemory::in_memory("/tmp/test_project").unwrap();

        mem.store_convention("Use snake_case for function names").unwrap();
        mem.store_convention("All public functions must have doc comments").unwrap();

        let results = mem.recall_conventions(10);
        assert_eq!(results.len(), 2);
    }

    #[test]
    fn test_store_session() {
        let mem = CognitiveMemory::in_memory("/tmp/test_project").unwrap();

        mem.store_session(&SessionSummary {
            timestamp: 1000,
            task: "Add authentication middleware".to_string(),
            files_modified: vec!["src/auth.rs".to_string(), "src/main.rs".to_string()],
            tasks_completed: vec!["JWT validation".to_string()],
            summary: "Implemented JWT-based auth middleware with role-based access control".to_string(),
        }).unwrap();

        let sessions = mem.recall_sessions(5);
        assert!(!sessions.is_empty());
        assert!(sessions[0].memory.content.as_str().contains("authentication"));
    }

    #[test]
    fn test_working_memory() {
        let mem = CognitiveMemory::in_memory("/tmp/test_project").unwrap();

        mem.push_working("Currently fixing bug in login flow", 300);
        mem.push_working("Error: null pointer at line 42", 300);

        let ctx = mem.working_context();
        assert!(ctx.is_some());
        let ctx_str = ctx.unwrap();
        assert!(ctx_str.contains("login flow"));
        assert!(ctx_str.contains("null pointer"));
    }

    #[test]
    fn test_build_context() {
        let mut mem = CognitiveMemory::in_memory("/tmp/test_project").unwrap();

        mem.store_convention("Use camelCase for variables").unwrap();
        mem.store_fact("Uses Redis for caching").unwrap();

        let ctx = mem.build_context();
        assert!(ctx.contains("camelCase"));
        assert!(ctx.contains("Redis"));
    }
}
