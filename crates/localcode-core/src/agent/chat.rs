use rusqlite::{params, Connection};
use serde::{Deserialize, Serialize};
use std::path::PathBuf;

use crate::indexing::embeddings::{
    bm25_score, compute_avg_doc_len, compute_doc_freqs, cosine_similarity, simple_embed,
};
use crate::CoreResult;

// ── Data types ───────────────────────────────────────────────

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ChatSession {
    pub id: String,
    pub project_path: String,
    pub session_id: String,
    pub title: String,
    pub created_at: u64,
    pub updated_at: u64,
    pub message_count: u32,
    pub summary: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ChatMessage {
    pub id: String,
    pub chat_session_id: String,
    pub role: String,
    pub content: String,
    pub timestamp: u64,
    pub agent_steps: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ChatSearchResult {
    pub message_id: String,
    pub chat_session_id: String,
    pub session_title: String,
    pub role: String,
    pub content: String,
    pub timestamp: u64,
    pub score: f32,
}

// ── Internal message with embedding ──────────────────────────

struct MessageWithEmbedding {
    id: String,
    chat_session_id: String,
    session_title: String,
    role: String,
    content: String,
    timestamp: u64,
    embedding: Vec<f32>,
}

// ── ChatStore ────────────────────────────────────────────────

pub struct ChatStore {
    conn: Connection,
}

impl ChatStore {
    pub fn new() -> CoreResult<Self> {
        let db_path = dirs::home_dir()
            .unwrap_or_else(|| PathBuf::from("~"))
            .join(".localcode")
            .join("chat.db");

        Self::open(db_path)
    }

    pub fn open(db_path: PathBuf) -> CoreResult<Self> {
        if let Some(parent) = db_path.parent() {
            std::fs::create_dir_all(parent)?;
        }

        let conn = Connection::open(&db_path)
            .map_err(|e| crate::CoreError::Other(format!("SQLite open error: {}", e)))?;

        conn.execute_batch("PRAGMA journal_mode=WAL; PRAGMA synchronous=NORMAL;")
            .map_err(|e| crate::CoreError::Other(format!("SQLite pragma error: {}", e)))?;

        let store = Self { conn };
        store.run_migrations()?;
        Ok(store)
    }

    fn run_migrations(&self) -> CoreResult<()> {
        self.conn
            .execute_batch(
                "CREATE TABLE IF NOT EXISTS chat_sessions (
                    id TEXT PRIMARY KEY,
                    project_path TEXT NOT NULL,
                    session_id TEXT NOT NULL,
                    title TEXT NOT NULL DEFAULT 'New Chat',
                    created_at INTEGER NOT NULL,
                    updated_at INTEGER NOT NULL,
                    message_count INTEGER NOT NULL DEFAULT 0,
                    summary TEXT NOT NULL DEFAULT ''
                );

                CREATE TABLE IF NOT EXISTS chat_messages (
                    id TEXT PRIMARY KEY,
                    chat_session_id TEXT NOT NULL,
                    role TEXT NOT NULL,
                    content TEXT NOT NULL,
                    timestamp INTEGER NOT NULL,
                    embedding BLOB,
                    agent_steps TEXT,
                    FOREIGN KEY (chat_session_id) REFERENCES chat_sessions(id) ON DELETE CASCADE
                );

                CREATE INDEX IF NOT EXISTS idx_messages_session
                    ON chat_messages(chat_session_id);
                CREATE INDEX IF NOT EXISTS idx_sessions_project
                    ON chat_sessions(project_path);
                CREATE INDEX IF NOT EXISTS idx_sessions_updated
                    ON chat_sessions(updated_at DESC);",
            )
            .map_err(|e| crate::CoreError::Other(format!("Migration error: {}", e)))?;

        // Enable foreign keys
        self.conn
            .execute_batch("PRAGMA foreign_keys = ON;")
            .map_err(|e| crate::CoreError::Other(format!("FK pragma error: {}", e)))?;

        Ok(())
    }

    // ── Session CRUD ─────────────────────────────────────────

    pub fn create_session(
        &self,
        project_path: &str,
        title: &str,
    ) -> CoreResult<ChatSession> {
        let id = uuid::Uuid::new_v4().to_string();
        let session_id = uuid::Uuid::new_v4().to_string();
        let now = now_secs();

        self.conn
            .execute(
                "INSERT INTO chat_sessions (id, project_path, session_id, title, created_at, updated_at, message_count, summary)
                 VALUES (?1, ?2, ?3, ?4, ?5, ?6, 0, '')",
                params![id, project_path, session_id, title, now, now],
            )
            .map_err(|e| crate::CoreError::Other(format!("Insert session error: {}", e)))?;

        Ok(ChatSession {
            id,
            project_path: project_path.to_string(),
            session_id,
            title: title.to_string(),
            created_at: now,
            updated_at: now,
            message_count: 0,
            summary: String::new(),
        })
    }

    pub fn list_sessions(
        &self,
        project_path: Option<&str>,
        limit: usize,
    ) -> CoreResult<Vec<ChatSession>> {
        let mut sessions = Vec::new();

        if let Some(pp) = project_path {
            let mut stmt = self
                .conn
                .prepare(
                    "SELECT id, project_path, session_id, title, created_at, updated_at, message_count, summary
                     FROM chat_sessions WHERE project_path = ?1
                     ORDER BY updated_at DESC LIMIT ?2",
                )
                .map_err(|e| crate::CoreError::Other(format!("Prepare error: {}", e)))?;

            let rows = stmt
                .query_map(params![pp, limit as i64], |row| {
                    Ok(ChatSession {
                        id: row.get(0)?,
                        project_path: row.get(1)?,
                        session_id: row.get(2)?,
                        title: row.get(3)?,
                        created_at: row.get(4)?,
                        updated_at: row.get(5)?,
                        message_count: row.get(6)?,
                        summary: row.get(7)?,
                    })
                })
                .map_err(|e| crate::CoreError::Other(format!("Query error: {}", e)))?;

            for row in rows {
                sessions.push(
                    row.map_err(|e| crate::CoreError::Other(format!("Row error: {}", e)))?,
                );
            }
        } else {
            let mut stmt = self
                .conn
                .prepare(
                    "SELECT id, project_path, session_id, title, created_at, updated_at, message_count, summary
                     FROM chat_sessions ORDER BY updated_at DESC LIMIT ?1",
                )
                .map_err(|e| crate::CoreError::Other(format!("Prepare error: {}", e)))?;

            let rows = stmt
                .query_map(params![limit as i64], |row| {
                    Ok(ChatSession {
                        id: row.get(0)?,
                        project_path: row.get(1)?,
                        session_id: row.get(2)?,
                        title: row.get(3)?,
                        created_at: row.get(4)?,
                        updated_at: row.get(5)?,
                        message_count: row.get(6)?,
                        summary: row.get(7)?,
                    })
                })
                .map_err(|e| crate::CoreError::Other(format!("Query error: {}", e)))?;

            for row in rows {
                sessions.push(
                    row.map_err(|e| crate::CoreError::Other(format!("Row error: {}", e)))?,
                );
            }
        }

        Ok(sessions)
    }

    pub fn delete_session(&self, id: &str) -> CoreResult<bool> {
        let affected = self
            .conn
            .execute("DELETE FROM chat_sessions WHERE id = ?1", params![id])
            .map_err(|e| crate::CoreError::Other(format!("Delete error: {}", e)))?;

        Ok(affected > 0)
    }

    pub fn update_session_title(&self, id: &str, title: &str) -> CoreResult<bool> {
        let affected = self
            .conn
            .execute(
                "UPDATE chat_sessions SET title = ?1, updated_at = ?2 WHERE id = ?3",
                params![title, now_secs(), id],
            )
            .map_err(|e| crate::CoreError::Other(format!("Update error: {}", e)))?;

        Ok(affected > 0)
    }

    // ── Message CRUD ─────────────────────────────────────────

    pub fn add_message(&self, msg: &ChatMessage) -> CoreResult<()> {
        let embedding = simple_embed(&msg.content);
        let embedding_blob = encode_embedding(&embedding);

        self.conn
            .execute(
                "INSERT INTO chat_messages (id, chat_session_id, role, content, timestamp, embedding, agent_steps)
                 VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7)",
                params![
                    msg.id,
                    msg.chat_session_id,
                    msg.role,
                    msg.content,
                    msg.timestamp,
                    embedding_blob,
                    msg.agent_steps,
                ],
            )
            .map_err(|e| crate::CoreError::Other(format!("Insert message error: {}", e)))?;

        // Update session's updated_at and message_count
        self.conn
            .execute(
                "UPDATE chat_sessions SET updated_at = ?1, message_count = message_count + 1 WHERE id = ?2",
                params![now_secs(), msg.chat_session_id],
            )
            .map_err(|e| crate::CoreError::Other(format!("Update session error: {}", e)))?;

        Ok(())
    }

    pub fn update_message_content(
        &self,
        id: &str,
        content: &str,
        agent_steps: Option<&str>,
    ) -> CoreResult<bool> {
        let embedding = simple_embed(content);
        let embedding_blob = encode_embedding(&embedding);

        let affected = self
            .conn
            .execute(
                "UPDATE chat_messages SET content = ?1, embedding = ?2, agent_steps = ?3 WHERE id = ?4",
                params![content, embedding_blob, agent_steps, id],
            )
            .map_err(|e| crate::CoreError::Other(format!("Update message error: {}", e)))?;

        Ok(affected > 0)
    }

    pub fn get_messages(&self, session_id: &str) -> CoreResult<Vec<ChatMessage>> {
        let mut stmt = self
            .conn
            .prepare(
                "SELECT id, chat_session_id, role, content, timestamp, agent_steps
                 FROM chat_messages WHERE chat_session_id = ?1
                 ORDER BY timestamp ASC",
            )
            .map_err(|e| crate::CoreError::Other(format!("Prepare error: {}", e)))?;

        let rows = stmt
            .query_map(params![session_id], |row| {
                Ok(ChatMessage {
                    id: row.get(0)?,
                    chat_session_id: row.get(1)?,
                    role: row.get(2)?,
                    content: row.get(3)?,
                    timestamp: row.get(4)?,
                    agent_steps: row.get(5)?,
                })
            })
            .map_err(|e| crate::CoreError::Other(format!("Query error: {}", e)))?;

        let mut messages = Vec::new();
        for row in rows {
            messages
                .push(row.map_err(|e| crate::CoreError::Other(format!("Row error: {}", e)))?);
        }

        Ok(messages)
    }

    // ── Hybrid Search ────────────────────────────────────────

    pub fn search_messages(
        &self,
        query: &str,
        project_path: Option<&str>,
        top_k: usize,
    ) -> CoreResult<Vec<ChatSearchResult>> {
        // Load all messages with embeddings (optionally filtered by project)
        let entries = self.load_messages_with_embeddings(project_path)?;

        if entries.is_empty() {
            return Ok(Vec::new());
        }

        let query_embedding = simple_embed(query);

        // Build BM25 inputs
        let docs: Vec<String> = entries.iter().map(|e| e.content.clone()).collect();
        let doc_refs: Vec<&str> = docs.iter().map(|s| s.as_str()).collect();
        let doc_freqs = compute_doc_freqs(&doc_refs);
        let avg_doc_len = compute_avg_doc_len(&doc_refs);
        let total_docs = docs.len();

        let cosine_weight: f32 = 0.4;
        let bm25_weight: f32 = 0.6;

        let mut scored: Vec<(f32, usize)> = entries
            .iter()
            .enumerate()
            .map(|(idx, entry)| {
                let cos_sim = cosine_similarity(&query_embedding, &entry.embedding);
                let bm25 =
                    bm25_score(query, &docs[idx], avg_doc_len, total_docs, &doc_freqs);
                let bm25_norm = (bm25 / 10.0).min(1.0);
                let combined = cosine_weight * cos_sim + bm25_weight * bm25_norm;
                (combined, idx)
            })
            .collect();

        scored.sort_by(|a, b| b.0.partial_cmp(&a.0).unwrap_or(std::cmp::Ordering::Equal));

        let results = scored
            .into_iter()
            .take(top_k)
            .filter(|(score, _)| *score > 0.01)
            .map(|(score, idx)| {
                let e = &entries[idx];
                ChatSearchResult {
                    message_id: e.id.clone(),
                    chat_session_id: e.chat_session_id.clone(),
                    session_title: e.session_title.clone(),
                    role: e.role.clone(),
                    content: e.content.clone(),
                    timestamp: e.timestamp,
                    score,
                }
            })
            .collect();

        Ok(results)
    }

    fn load_messages_with_embeddings(
        &self,
        project_path: Option<&str>,
    ) -> CoreResult<Vec<MessageWithEmbedding>> {
        let mut entries = Vec::new();

        let sql = if project_path.is_some() {
            "SELECT m.id, m.chat_session_id, s.title, m.role, m.content, m.timestamp, m.embedding
             FROM chat_messages m
             JOIN chat_sessions s ON m.chat_session_id = s.id
             WHERE s.project_path = ?1 AND m.embedding IS NOT NULL"
        } else {
            "SELECT m.id, m.chat_session_id, s.title, m.role, m.content, m.timestamp, m.embedding
             FROM chat_messages m
             JOIN chat_sessions s ON m.chat_session_id = s.id
             WHERE m.embedding IS NOT NULL"
        };

        let mut stmt = self
            .conn
            .prepare(sql)
            .map_err(|e| crate::CoreError::Other(format!("Prepare error: {}", e)))?;

        let map_row = |row: &rusqlite::Row| -> rusqlite::Result<MessageWithEmbedding> {
            let blob: Vec<u8> = row.get(6)?;
            let embedding = decode_embedding(&blob);
            Ok(MessageWithEmbedding {
                id: row.get(0)?,
                chat_session_id: row.get(1)?,
                session_title: row.get(2)?,
                role: row.get(3)?,
                content: row.get(4)?,
                timestamp: row.get(5)?,
                embedding,
            })
        };

        let rows = if let Some(pp) = project_path {
            stmt.query_map(params![pp], map_row)
        } else {
            stmt.query_map([], map_row)
        }
        .map_err(|e| crate::CoreError::Other(format!("Query error: {}", e)))?;

        for row in rows {
            entries
                .push(row.map_err(|e| crate::CoreError::Other(format!("Row error: {}", e)))?);
        }

        Ok(entries)
    }
}

// ── Embedding encode/decode (little-endian f32 BLOB) ─────────

fn encode_embedding(embedding: &[f32]) -> Vec<u8> {
    let mut bytes = Vec::with_capacity(embedding.len() * 4);
    for &val in embedding {
        bytes.extend_from_slice(&val.to_le_bytes());
    }
    bytes
}

fn decode_embedding(blob: &[u8]) -> Vec<f32> {
    blob.chunks_exact(4)
        .map(|chunk| {
            let arr: [u8; 4] = chunk.try_into().unwrap();
            f32::from_le_bytes(arr)
        })
        .collect()
}

fn now_secs() -> u64 {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap_or_default()
        .as_secs()
}

// ── Tests ────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;

    fn temp_store() -> ChatStore {
        let dir = tempfile::TempDir::new().unwrap();
        let db_path = dir.keep().join("test_chat.db");
        ChatStore::open(db_path).unwrap()
    }

    #[test]
    fn test_create_session_and_list() {
        let store = temp_store();
        let s = store.create_session("/project/a", "Test Chat").unwrap();
        assert!(!s.id.is_empty());
        assert_eq!(s.title, "Test Chat");
        assert_eq!(s.message_count, 0);

        let sessions = store.list_sessions(Some("/project/a"), 10).unwrap();
        assert_eq!(sessions.len(), 1);
        assert_eq!(sessions[0].id, s.id);

        // Different project returns empty
        let sessions = store.list_sessions(Some("/project/b"), 10).unwrap();
        assert!(sessions.is_empty());

        // No project filter returns all
        let sessions = store.list_sessions(None, 10).unwrap();
        assert_eq!(sessions.len(), 1);
    }

    #[test]
    fn test_add_and_get_messages() {
        let store = temp_store();
        let session = store.create_session("/test", "Chat").unwrap();

        let msg1 = ChatMessage {
            id: "msg-1".to_string(),
            chat_session_id: session.id.clone(),
            role: "user".to_string(),
            content: "Hello, how are you?".to_string(),
            timestamp: 1000,
            agent_steps: None,
        };
        let msg2 = ChatMessage {
            id: "msg-2".to_string(),
            chat_session_id: session.id.clone(),
            role: "assistant".to_string(),
            content: "I'm doing well! How can I help you with your code?".to_string(),
            timestamp: 1001,
            agent_steps: Some("[{\"type\":\"thinking\"}]".to_string()),
        };

        store.add_message(&msg1).unwrap();
        store.add_message(&msg2).unwrap();

        let messages = store.get_messages(&session.id).unwrap();
        assert_eq!(messages.len(), 2);
        assert_eq!(messages[0].role, "user");
        assert_eq!(messages[1].role, "assistant");
        assert_eq!(messages[1].agent_steps.as_deref(), Some("[{\"type\":\"thinking\"}]"));

        // message_count updated
        let sessions = store.list_sessions(None, 10).unwrap();
        assert_eq!(sessions[0].message_count, 2);
    }

    #[test]
    fn test_update_message_content() {
        let store = temp_store();
        let session = store.create_session("/test", "Chat").unwrap();

        let msg = ChatMessage {
            id: "msg-upd".to_string(),
            chat_session_id: session.id.clone(),
            role: "assistant".to_string(),
            content: "partial response".to_string(),
            timestamp: 1000,
            agent_steps: None,
        };
        store.add_message(&msg).unwrap();

        let updated = store
            .update_message_content("msg-upd", "full complete response about Rust programming", Some("[{\"type\":\"done\"}]"))
            .unwrap();
        assert!(updated);

        let messages = store.get_messages(&session.id).unwrap();
        assert_eq!(messages[0].content, "full complete response about Rust programming");
        assert_eq!(messages[0].agent_steps.as_deref(), Some("[{\"type\":\"done\"}]"));
    }

    #[test]
    fn test_delete_session_cascades() {
        let store = temp_store();
        let session = store.create_session("/test", "Delete Me").unwrap();

        store
            .add_message(&ChatMessage {
                id: "del-msg-1".to_string(),
                chat_session_id: session.id.clone(),
                role: "user".to_string(),
                content: "message to delete".to_string(),
                timestamp: 1000,
                agent_steps: None,
            })
            .unwrap();

        store
            .add_message(&ChatMessage {
                id: "del-msg-2".to_string(),
                chat_session_id: session.id.clone(),
                role: "assistant".to_string(),
                content: "another message".to_string(),
                timestamp: 1001,
                agent_steps: None,
            })
            .unwrap();

        let deleted = store.delete_session(&session.id).unwrap();
        assert!(deleted);

        // Session gone
        let sessions = store.list_sessions(None, 10).unwrap();
        assert!(sessions.is_empty());

        // Messages cascade-deleted
        let messages = store.get_messages(&session.id).unwrap();
        assert!(messages.is_empty());

        // Delete again returns false
        assert!(!store.delete_session(&session.id).unwrap());
    }

    #[test]
    fn test_update_session_title() {
        let store = temp_store();
        let session = store.create_session("/test", "Old Title").unwrap();

        let updated = store.update_session_title(&session.id, "New Title").unwrap();
        assert!(updated);

        let sessions = store.list_sessions(None, 10).unwrap();
        assert_eq!(sessions[0].title, "New Title");
    }

    #[test]
    fn test_search_relevance() {
        let store = temp_store();
        let session = store.create_session("/test", "Rust Chat").unwrap();

        store
            .add_message(&ChatMessage {
                id: "search-1".to_string(),
                chat_session_id: session.id.clone(),
                role: "user".to_string(),
                content: "How do I implement a binary search tree in Rust?".to_string(),
                timestamp: 1000,
                agent_steps: None,
            })
            .unwrap();

        store
            .add_message(&ChatMessage {
                id: "search-2".to_string(),
                chat_session_id: session.id.clone(),
                role: "assistant".to_string(),
                content: "Here is how to implement a binary search tree using structs and enums in Rust with insert and search methods".to_string(),
                timestamp: 1001,
                agent_steps: None,
            })
            .unwrap();

        store
            .add_message(&ChatMessage {
                id: "search-3".to_string(),
                chat_session_id: session.id.clone(),
                role: "user".to_string(),
                content: "What is the weather like today?".to_string(),
                timestamp: 1002,
                agent_steps: None,
            })
            .unwrap();

        let results = store.search_messages("binary search tree Rust", None, 5).unwrap();
        assert!(!results.is_empty());

        // The binary search tree messages should score higher than weather
        let top_ids: Vec<&str> = results.iter().map(|r| r.message_id.as_str()).collect();
        // At minimum, one of the BST messages should appear before the weather one
        let bst_pos = top_ids.iter().position(|id| *id == "search-1" || *id == "search-2");
        let weather_pos = top_ids.iter().position(|id| *id == "search-3");

        if let (Some(bp), Some(wp)) = (bst_pos, weather_pos) {
            assert!(bp < wp, "BST messages should rank higher than weather");
        }
    }

    #[test]
    fn test_search_with_project_filter() {
        let store = temp_store();

        let s1 = store.create_session("/project-a", "Project A").unwrap();
        let s2 = store.create_session("/project-b", "Project B").unwrap();

        store
            .add_message(&ChatMessage {
                id: "pa-1".to_string(),
                chat_session_id: s1.id.clone(),
                role: "user".to_string(),
                content: "Rust programming question".to_string(),
                timestamp: 1000,
                agent_steps: None,
            })
            .unwrap();

        store
            .add_message(&ChatMessage {
                id: "pb-1".to_string(),
                chat_session_id: s2.id.clone(),
                role: "user".to_string(),
                content: "Rust programming question".to_string(),
                timestamp: 1001,
                agent_steps: None,
            })
            .unwrap();

        let results = store
            .search_messages("Rust programming", Some("/project-a"), 10)
            .unwrap();

        // Only project-a messages
        assert!(results.iter().all(|r| r.chat_session_id == s1.id));
    }

    #[test]
    fn test_persistence_across_reopen() {
        let dir = tempfile::TempDir::new().unwrap();
        let db_path = dir.path().join("persist_test.db");

        // Create and populate
        {
            let store = ChatStore::open(db_path.clone()).unwrap();
            let session = store.create_session("/test", "Persist Test").unwrap();
            store
                .add_message(&ChatMessage {
                    id: "persist-msg".to_string(),
                    chat_session_id: session.id.clone(),
                    role: "user".to_string(),
                    content: "This should survive a reopen".to_string(),
                    timestamp: 1000,
                    agent_steps: None,
                })
                .unwrap();
        }

        // Reopen and verify
        {
            let store = ChatStore::open(db_path).unwrap();
            let sessions = store.list_sessions(None, 10).unwrap();
            assert_eq!(sessions.len(), 1);
            assert_eq!(sessions[0].title, "Persist Test");

            let messages = store.get_messages(&sessions[0].id).unwrap();
            assert_eq!(messages.len(), 1);
            assert_eq!(messages[0].content, "This should survive a reopen");
        }
    }

    #[test]
    fn test_embedding_recomputed_on_update() {
        let store = temp_store();
        let session = store.create_session("/test", "Chat").unwrap();

        store
            .add_message(&ChatMessage {
                id: "emb-test".to_string(),
                chat_session_id: session.id.clone(),
                role: "assistant".to_string(),
                content: "cats and dogs".to_string(),
                timestamp: 1000,
                agent_steps: None,
            })
            .unwrap();

        // Search for original content
        let results1 = store.search_messages("cats dogs", None, 5).unwrap();
        let _score1 = results1
            .iter()
            .find(|r| r.message_id == "emb-test")
            .map(|r| r.score)
            .unwrap_or(0.0);

        // Update to completely different content
        store
            .update_message_content("emb-test", "quantum physics and relativity", None)
            .unwrap();

        // Now search for new content — should score higher
        let results2 = store.search_messages("quantum physics", None, 5).unwrap();
        let score2 = results2
            .iter()
            .find(|r| r.message_id == "emb-test")
            .map(|r| r.score)
            .unwrap_or(0.0);

        // Search for old content — should score lower now
        let results3 = store.search_messages("cats dogs", None, 5).unwrap();
        let score3 = results3
            .iter()
            .find(|r| r.message_id == "emb-test")
            .map(|r| r.score)
            .unwrap_or(0.0);

        assert!(
            score2 > score3,
            "Updated content should match new query better: {} vs {}",
            score2,
            score3
        );
    }

    #[test]
    fn test_list_sessions_limit() {
        let store = temp_store();
        for i in 0..5 {
            store
                .create_session("/test", &format!("Chat {}", i))
                .unwrap();
        }

        let sessions = store.list_sessions(None, 3).unwrap();
        assert_eq!(sessions.len(), 3);
    }

    #[test]
    fn test_sessions_ordered_by_updated_at() {
        let store = temp_store();

        let s1 = store.create_session("/test", "First").unwrap();
        let _s2 = store.create_session("/test", "Second").unwrap();

        // Add a message to s1 to update its updated_at
        store
            .add_message(&ChatMessage {
                id: "order-msg".to_string(),
                chat_session_id: s1.id.clone(),
                role: "user".to_string(),
                content: "hello".to_string(),
                timestamp: 9999,
                agent_steps: None,
            })
            .unwrap();

        let sessions = store.list_sessions(None, 10).unwrap();
        // s1 should be first since it was updated more recently (via add_message)
        assert_eq!(sessions[0].id, s1.id);
    }
}
