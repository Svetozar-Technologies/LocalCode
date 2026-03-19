use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::path::{Path, PathBuf};

use crate::CoreResult;
use crate::indexing::embeddings::{
    bm25_score, compute_avg_doc_len, compute_doc_freqs, cosine_similarity, simple_embed,
};

// ── SessionState (kept for backward compat) ──────────────────

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

// ── Persistent Session ───────────────────────────────────────

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Session {
    pub id: String,
    pub project_path: String,
    pub project_name: String,
    pub started_at: u64,
    pub ended_at: Option<u64>,
    pub files_modified: Vec<String>,
    pub tasks_completed: Vec<String>,
    pub conversation_summary: String,
    pub title: String,
    pub tags: Vec<String>,
}

#[derive(Debug, Serialize, Deserialize, Default)]
pub struct SessionIndex {
    pub sessions: Vec<SessionIndexEntry>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SessionIndexEntry {
    pub id: String,
    pub project_path: String,
    pub project_name: String,
    pub started_at: u64,
    pub title: String,
    pub tags: Vec<String>,
}

#[derive(Debug, Clone)]
pub struct SessionSearchResult {
    pub score: f32,
    pub entry: SessionIndexEntry,
}

// ── SessionStore ─────────────────────────────────────────────

pub struct SessionStore {
    base_dir: PathBuf,
    index: SessionIndex,
    embeddings: Vec<(String, Vec<f32>)>, // (session_id, embedding)
}

impl SessionStore {
    pub fn new() -> Self {
        let base_dir = dirs::home_dir()
            .unwrap_or_else(|| PathBuf::from("~"))
            .join(".localcode")
            .join("sessions");

        let index = Self::load_index(&base_dir).unwrap_or_default();
        let embeddings = Self::load_embeddings(&base_dir).unwrap_or_default();

        Self {
            base_dir,
            index,
            embeddings,
        }
    }

    /// Create a SessionStore with a custom base directory (for testing)
    #[cfg(test)]
    pub fn with_base_dir(base_dir: PathBuf) -> Self {
        let index = Self::load_index(&base_dir).unwrap_or_default();
        let embeddings = Self::load_embeddings(&base_dir).unwrap_or_default();
        Self {
            base_dir,
            index,
            embeddings,
        }
    }

    fn index_path(base_dir: &Path) -> PathBuf {
        base_dir.join("index.json")
    }

    fn embeddings_path(base_dir: &Path) -> PathBuf {
        base_dir.join("embeddings.bin")
    }

    fn session_path(&self, id: &str) -> PathBuf {
        self.base_dir.join(format!("{}.json", id))
    }

    fn load_index(base_dir: &Path) -> CoreResult<SessionIndex> {
        let path = Self::index_path(base_dir);
        if path.exists() {
            let content = std::fs::read_to_string(&path)?;
            let index: SessionIndex = serde_json::from_str(&content)?;
            Ok(index)
        } else {
            Ok(SessionIndex::default())
        }
    }

    fn save_index(&self) -> CoreResult<()> {
        std::fs::create_dir_all(&self.base_dir)?;
        let content = serde_json::to_string_pretty(&self.index)?;
        std::fs::write(Self::index_path(&self.base_dir), content)?;
        Ok(())
    }

    fn load_embeddings(base_dir: &Path) -> CoreResult<Vec<(String, Vec<f32>)>> {
        let path = Self::embeddings_path(base_dir);
        if path.exists() {
            let content = std::fs::read_to_string(&path)?;
            let data: Vec<(String, Vec<f32>)> = serde_json::from_str(&content)?;
            Ok(data)
        } else {
            Ok(Vec::new())
        }
    }

    fn save_embeddings(&self) -> CoreResult<()> {
        std::fs::create_dir_all(&self.base_dir)?;
        let content = serde_json::to_string(&self.embeddings)?;
        std::fs::write(Self::embeddings_path(&self.base_dir), content)?;
        Ok(())
    }

    /// Save a session persistently
    pub fn save_session(&mut self, session: &Session) -> CoreResult<()> {
        // 1. Write {uuid}.json
        std::fs::create_dir_all(&self.base_dir)?;
        let session_json = serde_json::to_string_pretty(session)?;
        std::fs::write(self.session_path(&session.id), session_json)?;

        // 2. Compute embedding from: title + summary + tags + files_modified
        let embed_text = format!(
            "{} {} {} {}",
            session.title,
            session.conversation_summary,
            session.tags.join(" "),
            session.files_modified.join(" ")
        );
        let embedding = simple_embed(&embed_text);
        self.embeddings.push((session.id.clone(), embedding));

        // 3. Add to index
        self.index.sessions.push(SessionIndexEntry {
            id: session.id.clone(),
            project_path: session.project_path.clone(),
            project_name: session.project_name.clone(),
            started_at: session.started_at,
            title: session.title.clone(),
            tags: session.tags.clone(),
        });

        // 4. Persist
        self.save_index()?;
        self.save_embeddings()?;

        Ok(())
    }

    /// Hybrid semantic search over sessions
    pub fn search(&self, query: &str, top_k: usize) -> Vec<SessionSearchResult> {
        if self.index.sessions.is_empty() {
            return Vec::new();
        }

        let query_embedding = simple_embed(query);

        // Build searchable text for each session
        let docs: Vec<String> = self
            .index
            .sessions
            .iter()
            .map(|e| {
                format!(
                    "{} {} {} {}",
                    e.title,
                    e.project_name,
                    e.tags.join(" "),
                    e.project_path
                )
            })
            .collect();

        let doc_refs: Vec<&str> = docs.iter().map(|s| s.as_str()).collect();
        let doc_freqs = compute_doc_freqs(&doc_refs);
        let avg_doc_len = compute_avg_doc_len(&doc_refs);
        let total_docs = docs.len();

        let cosine_weight: f32 = 0.4;
        let bm25_weight: f32 = 0.6;

        let mut scored: Vec<(f32, usize)> = self
            .index
            .sessions
            .iter()
            .enumerate()
            .map(|(idx, _entry)| {
                // Cosine similarity from embeddings
                let cos_sim = self
                    .embeddings
                    .iter()
                    .find(|(id, _)| id == &self.index.sessions[idx].id)
                    .map(|(_, emb)| cosine_similarity(&query_embedding, emb))
                    .unwrap_or(0.0);

                // BM25 score
                let bm25 = bm25_score(query, &docs[idx], avg_doc_len, total_docs, &doc_freqs);
                let bm25_norm = (bm25 / 10.0).min(1.0);

                let combined = cosine_weight * cos_sim + bm25_weight * bm25_norm;
                (combined, idx)
            })
            .collect();

        scored.sort_by(|a, b| b.0.partial_cmp(&a.0).unwrap_or(std::cmp::Ordering::Equal));

        scored
            .into_iter()
            .take(top_k)
            .filter(|(score, _)| *score > 0.01)
            .map(|(score, idx)| SessionSearchResult {
                score,
                entry: self.index.sessions[idx].clone(),
            })
            .collect()
    }

    /// Filter sessions by project path
    pub fn search_by_project(&self, project_path: &str) -> Vec<SessionIndexEntry> {
        self.index
            .sessions
            .iter()
            .filter(|e| e.project_path == project_path)
            .cloned()
            .collect()
    }

    /// Filter sessions by date range
    pub fn search_by_date_range(&self, from: u64, to: u64) -> Vec<SessionIndexEntry> {
        self.index
            .sessions
            .iter()
            .filter(|e| e.started_at >= from && e.started_at <= to)
            .cloned()
            .collect()
    }

    /// Get full session data by ID
    pub fn get_session(&self, id: &str) -> Option<Session> {
        let path = self.session_path(id);
        if path.exists() {
            let content = std::fs::read_to_string(&path).ok()?;
            serde_json::from_str(&content).ok()
        } else {
            None
        }
    }

    /// Delete a session by ID
    pub fn delete_session(&mut self, id: &str) -> bool {
        let initial_len = self.index.sessions.len();
        self.index.sessions.retain(|e| e.id != id);

        if self.index.sessions.len() == initial_len {
            return false; // Not found
        }

        // Remove embedding
        self.embeddings.retain(|(eid, _)| eid != id);

        // Delete file
        let path = self.session_path(id);
        let _ = std::fs::remove_file(&path);

        // Persist changes
        let _ = self.save_index();
        let _ = self.save_embeddings();

        true
    }

    /// List recent sessions sorted by started_at desc
    pub fn list_recent(&self, count: usize) -> Vec<SessionIndexEntry> {
        let mut sessions = self.index.sessions.clone();
        sessions.sort_by(|a, b| b.started_at.cmp(&a.started_at));
        sessions.into_iter().take(count).collect()
    }
}

impl Default for SessionStore {
    fn default() -> Self {
        Self::new()
    }
}

// ── Auto-tagging helpers ─────────────────────────────────────

/// Stopwords to filter from tag extraction
const STOPWORDS: &[&str] = &[
    "the", "a", "an", "and", "or", "but", "in", "on", "at", "to", "for", "of", "with",
    "by", "from", "is", "was", "are", "were", "be", "been", "being", "have", "has", "had",
    "do", "does", "did", "will", "would", "could", "should", "may", "might", "can",
    "this", "that", "these", "those", "i", "we", "you", "he", "she", "it", "they",
    "my", "our", "your", "his", "her", "its", "their", "me", "us", "him", "them",
    "not", "no", "so", "if", "then", "than", "too", "very", "just", "about",
];

pub fn auto_generate_tags(session: &Session) -> Vec<String> {
    let mut tags = vec![];

    // Project name
    if !session.project_name.is_empty() {
        tags.push(session.project_name.clone().to_lowercase());
    }

    // Language from file extensions
    for file in &session.files_modified {
        if let Some(ext) = Path::new(file).extension().and_then(|e| e.to_str()) {
            let lang = match ext {
                "py" => "python",
                "rs" => "rust",
                "js" => "javascript",
                "ts" => "typescript",
                "go" => "go",
                "java" => "java",
                "c" | "h" => "c",
                "cpp" | "cc" | "hpp" => "cpp",
                "rb" => "ruby",
                "swift" => "swift",
                "kt" => "kotlin",
                "html" | "htm" => "html",
                "css" | "scss" => "css",
                _ => "",
            };
            if !lang.is_empty() && !tags.contains(&lang.to_string()) {
                tags.push(lang.to_string());
            }
        }
    }

    // Extract key words from summary (top 5 by frequency, excluding stopwords)
    let mut word_freq: HashMap<String, usize> = HashMap::new();
    let text = format!(
        "{} {}",
        session.conversation_summary,
        session.tasks_completed.join(" ")
    );
    for word in text.split_whitespace() {
        let w = word
            .trim_matches(|c: char| !c.is_alphanumeric())
            .to_lowercase();
        if w.len() > 2 && !STOPWORDS.contains(&w.as_str()) {
            *word_freq.entry(w).or_insert(0) += 1;
        }
    }

    let mut freq_words: Vec<(String, usize)> = word_freq.into_iter().collect();
    freq_words.sort_by(|a, b| b.1.cmp(&a.1));

    for (word, _) in freq_words.into_iter().take(5) {
        if !tags.contains(&word) {
            tags.push(word);
        }
    }

    tags
}

pub fn auto_generate_title(session: &Session) -> String {
    session
        .tasks_completed
        .first()
        .cloned()
        .unwrap_or_else(|| {
            if session.conversation_summary.is_empty() {
                "Untitled session".to_string()
            } else {
                session
                    .conversation_summary
                    .chars()
                    .take(60)
                    .collect::<String>()
            }
        })
}

/// Create a persistent Session from a SessionState
pub fn session_from_state(state: &SessionState, summary: &str) -> Session {
    let project_name = Path::new(&state.project_path)
        .file_name()
        .and_then(|n| n.to_str())
        .unwrap_or("unknown")
        .to_string();

    let mut session = Session {
        id: uuid::Uuid::new_v4().to_string(),
        project_path: state.project_path.clone(),
        project_name,
        started_at: state.started_at,
        ended_at: Some(now_secs()),
        files_modified: state.files_modified.clone(),
        tasks_completed: state.tasks_completed.clone(),
        conversation_summary: summary.to_string(),
        title: String::new(),
        tags: Vec::new(),
    };

    session.title = auto_generate_title(&session);
    session.tags = auto_generate_tags(&session);

    session
}

// ── Utilities ────────────────────────────────────────────────

fn simple_hash(input: &str) -> String {
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_new_session() {
        let session = SessionState::new("/home/user/project");
        assert_eq!(session.project_path, "/home/user/project");
        assert!(session.files_modified.is_empty());
        assert!(session.tasks_completed.is_empty());
        assert!(session.conversation_summary.is_empty());
        assert!(session.started_at > 0);
    }

    #[test]
    fn test_add_file_modified() {
        let mut session = SessionState::new("/project");
        session.add_file_modified("src/main.rs");
        session.add_file_modified("src/lib.rs");
        session.add_file_modified("src/main.rs");

        assert_eq!(session.files_modified.len(), 2);
        assert!(session.files_modified.contains(&"src/main.rs".to_string()));
        assert!(session.files_modified.contains(&"src/lib.rs".to_string()));
    }

    #[test]
    fn test_add_task_completed() {
        let mut session = SessionState::new("/project");
        session.add_task_completed("Created main module");
        session.add_task_completed("Added tests");

        assert_eq!(session.tasks_completed.len(), 2);
        assert_eq!(session.tasks_completed[0], "Created main module");
        assert_eq!(session.tasks_completed[1], "Added tests");
    }

    #[test]
    fn test_save_and_load() {
        let mut session = SessionState::new("/test/project");
        session.add_file_modified("src/main.rs");
        session.add_task_completed("setup project");
        session.conversation_summary = "We set up the project structure.".to_string();

        let json = serde_json::to_string_pretty(&session).unwrap();

        let loaded: SessionState = serde_json::from_str(&json).unwrap();
        assert_eq!(loaded.project_path, "/test/project");
        assert_eq!(loaded.files_modified, vec!["src/main.rs".to_string()]);
        assert_eq!(loaded.tasks_completed, vec!["setup project".to_string()]);
        assert_eq!(
            loaded.conversation_summary,
            "We set up the project structure."
        );
        assert_eq!(loaded.started_at, session.started_at);
    }

    #[test]
    fn test_session_path_deterministic() {
        let path1 = SessionState::session_path("/my/project");
        let path2 = SessionState::session_path("/my/project");
        assert_eq!(path1, path2);

        let path3 = SessionState::session_path("/other/project");
        assert_ne!(path1, path3);
    }

    #[test]
    fn test_auto_generate_title() {
        let session = Session {
            id: "test".to_string(),
            project_path: "/test".to_string(),
            project_name: "test".to_string(),
            started_at: 0,
            ended_at: None,
            files_modified: vec![],
            tasks_completed: vec!["Built snake game with pygame".to_string()],
            conversation_summary: "We created a snake game.".to_string(),
            title: String::new(),
            tags: vec![],
        };

        let title = auto_generate_title(&session);
        assert_eq!(title, "Built snake game with pygame");
    }

    #[test]
    fn test_auto_generate_title_from_summary() {
        let session = Session {
            id: "test".to_string(),
            project_path: "/test".to_string(),
            project_name: "test".to_string(),
            started_at: 0,
            ended_at: None,
            files_modified: vec![],
            tasks_completed: vec![],
            conversation_summary: "We refactored the authentication module to use JWT tokens instead of session cookies.".to_string(),
            title: String::new(),
            tags: vec![],
        };

        let title = auto_generate_title(&session);
        assert_eq!(title.len(), 60);
    }

    #[test]
    fn test_auto_generate_tags() {
        let session = Session {
            id: "test".to_string(),
            project_path: "/home/user/my-app".to_string(),
            project_name: "my-app".to_string(),
            started_at: 0,
            ended_at: None,
            files_modified: vec!["game.py".to_string(), "utils.py".to_string()],
            tasks_completed: vec!["Created snake game".to_string()],
            conversation_summary: "Built a snake game using pygame library".to_string(),
            title: String::new(),
            tags: vec![],
        };

        let tags = auto_generate_tags(&session);
        assert!(tags.contains(&"my-app".to_string()));
        assert!(tags.contains(&"python".to_string()));
        assert!(tags.iter().any(|t| t == "snake" || t == "game" || t == "pygame"));
    }

    #[test]
    fn test_session_store_save_and_search() {
        let dir = tempfile::TempDir::new().unwrap();
        let mut store = SessionStore::with_base_dir(dir.path().join("sessions"));

        let session = Session {
            id: uuid::Uuid::new_v4().to_string(),
            project_path: "/home/user/snake-game".to_string(),
            project_name: "snake-game".to_string(),
            started_at: 1700000000,
            ended_at: Some(1700003600),
            files_modified: vec!["snake.py".to_string()],
            tasks_completed: vec!["Built snake game".to_string()],
            conversation_summary: "Created a snake game using pygame with keyboard controls".to_string(),
            title: "Built snake game".to_string(),
            tags: vec!["python".to_string(), "game".to_string(), "pygame".to_string()],
        };

        store.save_session(&session).unwrap();

        // Search should find it
        let results = store.search("snake game", 5);
        assert!(!results.is_empty());
        assert_eq!(results[0].entry.id, session.id);

        // Get by ID
        let loaded = store.get_session(&session.id).unwrap();
        assert_eq!(loaded.title, "Built snake game");

        // List recent
        let recent = store.list_recent(10);
        assert_eq!(recent.len(), 1);

        // Search by project
        let proj = store.search_by_project("/home/user/snake-game");
        assert_eq!(proj.len(), 1);
    }

    #[test]
    fn test_session_store_delete() {
        let dir = tempfile::TempDir::new().unwrap();
        let mut store = SessionStore::with_base_dir(dir.path().join("sessions"));

        let session = Session {
            id: "test-delete-id".to_string(),
            project_path: "/test".to_string(),
            project_name: "test".to_string(),
            started_at: 1700000000,
            ended_at: None,
            files_modified: vec![],
            tasks_completed: vec![],
            conversation_summary: "Test session".to_string(),
            title: "Test".to_string(),
            tags: vec![],
        };

        store.save_session(&session).unwrap();
        assert_eq!(store.list_recent(10).len(), 1);

        let deleted = store.delete_session("test-delete-id");
        assert!(deleted);
        assert_eq!(store.list_recent(10).len(), 0);

        // Deleting again returns false
        assert!(!store.delete_session("test-delete-id"));
    }

    #[test]
    fn test_session_from_state() {
        let mut state = SessionState::new("/home/user/myproject");
        state.add_file_modified("main.py");
        state.add_task_completed("Created hello world");

        let session = session_from_state(&state, "Built a hello world program in Python");
        assert_eq!(session.project_name, "myproject");
        assert!(!session.id.is_empty());
        assert_eq!(session.title, "Created hello world");
        assert!(session.tags.contains(&"python".to_string()));
        assert!(session.ended_at.is_some());
    }

    #[test]
    fn test_session_store_search_by_project() {
        let dir = tempfile::TempDir::new().unwrap();
        let mut store = SessionStore::with_base_dir(dir.path().join("sessions"));

        // Save sessions for two different projects
        let s1 = Session {
            id: "proj-a-1".to_string(),
            project_path: "/home/user/project-a".to_string(),
            project_name: "project-a".to_string(),
            started_at: 1700000000,
            ended_at: None,
            files_modified: vec![],
            tasks_completed: vec![],
            conversation_summary: "Session for project A".to_string(),
            title: "Project A work".to_string(),
            tags: vec![],
        };
        let s2 = Session {
            id: "proj-b-1".to_string(),
            project_path: "/home/user/project-b".to_string(),
            project_name: "project-b".to_string(),
            started_at: 1700001000,
            ended_at: None,
            files_modified: vec![],
            tasks_completed: vec![],
            conversation_summary: "Session for project B".to_string(),
            title: "Project B work".to_string(),
            tags: vec![],
        };
        let s3 = Session {
            id: "proj-a-2".to_string(),
            project_path: "/home/user/project-a".to_string(),
            project_name: "project-a".to_string(),
            started_at: 1700002000,
            ended_at: None,
            files_modified: vec![],
            tasks_completed: vec![],
            conversation_summary: "Second session for project A".to_string(),
            title: "Project A more work".to_string(),
            tags: vec![],
        };

        store.save_session(&s1).unwrap();
        store.save_session(&s2).unwrap();
        store.save_session(&s3).unwrap();

        let proj_a = store.search_by_project("/home/user/project-a");
        assert_eq!(proj_a.len(), 2);

        let proj_b = store.search_by_project("/home/user/project-b");
        assert_eq!(proj_b.len(), 1);

        let proj_c = store.search_by_project("/nonexistent");
        assert_eq!(proj_c.len(), 0);
    }

    #[test]
    fn test_session_store_search_by_date_range() {
        let dir = tempfile::TempDir::new().unwrap();
        let mut store = SessionStore::with_base_dir(dir.path().join("sessions"));

        for i in 0..5 {
            let s = Session {
                id: format!("date-{}", i),
                project_path: "/test".to_string(),
                project_name: "test".to_string(),
                started_at: 1700000000 + (i * 1000),
                ended_at: None,
                files_modified: vec![],
                tasks_completed: vec![],
                conversation_summary: format!("Session {}", i),
                title: format!("Session {}", i),
                tags: vec![],
            };
            store.save_session(&s).unwrap();
        }

        // Range covering sessions 1-3 (timestamps 1700001000 to 1700003000)
        let results = store.search_by_date_range(1700001000, 1700003000);
        assert_eq!(results.len(), 3);

        // Range covering none
        let results = store.search_by_date_range(1800000000, 1900000000);
        assert_eq!(results.len(), 0);
    }

    #[test]
    fn test_session_store_list_recent_ordering() {
        let dir = tempfile::TempDir::new().unwrap();
        let mut store = SessionStore::with_base_dir(dir.path().join("sessions"));

        // Save sessions with different timestamps (out of order)
        for (i, ts) in [(2, 1700002000u64), (0, 1700000000), (1, 1700001000)] {
            let s = Session {
                id: format!("order-{}", i),
                project_path: "/test".to_string(),
                project_name: "test".to_string(),
                started_at: ts,
                ended_at: None,
                files_modified: vec![],
                tasks_completed: vec![],
                conversation_summary: String::new(),
                title: format!("Session {}", i),
                tags: vec![],
            };
            store.save_session(&s).unwrap();
        }

        let recent = store.list_recent(10);
        assert_eq!(recent.len(), 3);
        // Most recent first
        assert_eq!(recent[0].id, "order-2");
        assert_eq!(recent[1].id, "order-1");
        assert_eq!(recent[2].id, "order-0");
    }

    #[test]
    fn test_session_store_persistence_across_reload() {
        let dir = tempfile::TempDir::new().unwrap();
        let store_path = dir.path().join("sessions");

        // Save a session
        {
            let mut store = SessionStore::with_base_dir(store_path.clone());
            let s = Session {
                id: "persist-test".to_string(),
                project_path: "/test".to_string(),
                project_name: "test".to_string(),
                started_at: 1700000000,
                ended_at: None,
                files_modified: vec!["main.rs".to_string()],
                tasks_completed: vec!["Built app".to_string()],
                conversation_summary: "We built an app".to_string(),
                title: "Built app".to_string(),
                tags: vec!["rust".to_string()],
            };
            store.save_session(&s).unwrap();
        }

        // Reload from disk
        let store2 = SessionStore::with_base_dir(store_path);
        assert_eq!(store2.list_recent(10).len(), 1);

        let loaded = store2.get_session("persist-test").unwrap();
        assert_eq!(loaded.title, "Built app");
        assert_eq!(loaded.tags, vec!["rust".to_string()]);
        assert_eq!(loaded.files_modified, vec!["main.rs".to_string()]);

        // Search still works after reload
        let results = store2.search("built app rust", 5);
        assert!(!results.is_empty());
        assert_eq!(results[0].entry.id, "persist-test");
    }

    #[test]
    fn test_session_store_search_empty_store() {
        let dir = tempfile::TempDir::new().unwrap();
        let store = SessionStore::with_base_dir(dir.path().join("sessions"));

        let results = store.search("anything", 5);
        assert!(results.is_empty());

        let recent = store.list_recent(10);
        assert!(recent.is_empty());
    }

    #[test]
    fn test_auto_generate_title_untitled() {
        let session = Session {
            id: "test".to_string(),
            project_path: "/test".to_string(),
            project_name: "test".to_string(),
            started_at: 0,
            ended_at: None,
            files_modified: vec![],
            tasks_completed: vec![],
            conversation_summary: String::new(),
            title: String::new(),
            tags: vec![],
        };
        assert_eq!(auto_generate_title(&session), "Untitled session");
    }

    #[test]
    fn test_auto_generate_tags_multiple_languages() {
        let session = Session {
            id: "test".to_string(),
            project_path: "/test/fullstack".to_string(),
            project_name: "fullstack".to_string(),
            started_at: 0,
            ended_at: None,
            files_modified: vec![
                "app.ts".to_string(),
                "style.css".to_string(),
                "server.py".to_string(),
            ],
            tasks_completed: vec![],
            conversation_summary: "Built a fullstack app".to_string(),
            title: String::new(),
            tags: vec![],
        };

        let tags = auto_generate_tags(&session);
        assert!(tags.contains(&"typescript".to_string()));
        assert!(tags.contains(&"css".to_string()));
        assert!(tags.contains(&"python".to_string()));
    }
}
