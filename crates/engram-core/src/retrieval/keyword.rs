//! BM25-inspired keyword matching for full-text search.
//!
//! Provides lexical matching as a complement to vector similarity,
//! catching exact keyword matches that embeddings might miss.

use crate::memory::types::*;
use std::collections::HashMap;
use std::io::{Read as IoRead, Write as IoWrite};
use std::path::Path;

/// BM25 parameters
const K1: f32 = 1.2;
const B: f32 = 0.75;

/// Simple tokenizer: lowercase, split on whitespace and punctuation
pub fn tokenize(text: &str) -> Vec<String> {
    text.to_lowercase()
        .split(|c: char| !c.is_alphanumeric() && c != '\'')
        .filter(|s| !s.is_empty() && s.len() > 1)
        .map(String::from)
        .collect()
}

/// Calculate BM25 score for a document against a query
pub fn bm25_score(
    doc_tokens: &[String],
    query_tokens: &[String],
    avg_doc_len: f32,
    total_docs: usize,
    doc_frequencies: &HashMap<String, usize>,
) -> f32 {
    let doc_len = doc_tokens.len() as f32;

    // Count term frequencies in document
    let mut tf: HashMap<&str, usize> = HashMap::new();
    for token in doc_tokens {
        *tf.entry(token.as_str()).or_insert(0) += 1;
    }

    let mut score = 0.0f32;

    for query_term in query_tokens {
        let term_freq = *tf.get(query_term.as_str()).unwrap_or(&0) as f32;
        if term_freq == 0.0 {
            continue;
        }

        // IDF component
        let df = *doc_frequencies.get(query_term.as_str()).unwrap_or(&0) as f32;
        let idf = ((total_docs as f32 - df + 0.5) / (df + 0.5) + 1.0).ln();

        // TF component with length normalization
        let tf_norm = (term_freq * (K1 + 1.0)) / (term_freq + K1 * (1.0 - B + B * doc_len / avg_doc_len));

        score += idf * tf_norm;
    }

    score
}

/// Simple keyword scorer that computes term overlap ratio
pub fn keyword_overlap_score(memory_content: &str, query: &str) -> f32 {
    let mem_tokens: std::collections::HashSet<String> = tokenize(memory_content).into_iter().collect();
    let query_tokens: std::collections::HashSet<String> = tokenize(query).into_iter().collect();

    if query_tokens.is_empty() {
        return 0.0;
    }

    let overlap = mem_tokens.intersection(&query_tokens).count() as f32;
    overlap / query_tokens.len() as f32
}

/// Keyword index for fast lookup
pub struct KeywordIndex {
    /// Token -> set of memory IDs that contain it
    inverted_index: HashMap<String, Vec<MemoryId>>,
    /// Memory ID -> token count (for BM25 doc length)
    doc_lengths: HashMap<MemoryId, usize>,
    /// Precomputed term frequencies per document: MemoryId -> (term -> count)
    doc_term_freqs: HashMap<MemoryId, HashMap<String, usize>>,
    /// Total documents
    total_docs: usize,
    /// Sum of all doc lengths
    total_tokens: usize,
}

impl KeywordIndex {
    pub fn new() -> Self {
        Self {
            inverted_index: HashMap::new(),
            doc_lengths: HashMap::new(),
            doc_term_freqs: HashMap::new(),
            total_docs: 0,
            total_tokens: 0,
        }
    }

    /// Index a memory's content
    pub fn add(&mut self, id: MemoryId, content: &str) {
        let tokens = tokenize(content);
        self.doc_lengths.insert(id, tokens.len());
        self.total_docs += 1;
        self.total_tokens += tokens.len();

        // Precompute term frequencies for this document
        let mut tf: HashMap<String, usize> = HashMap::new();
        for token in &tokens {
            *tf.entry(token.clone()).or_insert(0) += 1;
        }
        self.doc_term_freqs.insert(id, tf);

        for token in tokens {
            self.inverted_index
                .entry(token)
                .or_default()
                .push(id);
        }
    }

    /// Remove a memory from the index
    pub fn remove(&mut self, id: &MemoryId) {
        if let Some(len) = self.doc_lengths.remove(id) {
            self.total_docs -= 1;
            self.total_tokens -= len;
        }
        self.doc_term_freqs.remove(id);
        for ids in self.inverted_index.values_mut() {
            ids.retain(|i| i != id);
        }
    }

    /// Search the index and return scored results
    pub fn search(&self, query: &str, limit: usize) -> Vec<(MemoryId, f32)> {
        let query_tokens = tokenize(query);
        if query_tokens.is_empty() {
            return Vec::new();
        }

        // Find candidate documents
        let mut candidates: HashMap<MemoryId, f32> = HashMap::new();
        let avg_doc_len = if self.total_docs > 0 {
            self.total_tokens as f32 / self.total_docs as f32
        } else {
            1.0
        };

        // Build document frequency map for query terms
        let mut df: HashMap<String, usize> = HashMap::new();
        for token in &query_tokens {
            if let Some(ids) = self.inverted_index.get(token) {
                df.insert(token.clone(), ids.len());
                for id in ids {
                    candidates.entry(*id).or_insert(0.0);
                }
            }
        }

        // Score each candidate using precomputed term frequencies
        let mut results: Vec<(MemoryId, f32)> = candidates
            .keys()
            .map(|id| {
                let doc_len = *self.doc_lengths.get(id).unwrap_or(&1) as f32;
                let doc_tf = self.doc_term_freqs.get(id);

                let mut score = 0.0f32;
                for token in &query_tokens {
                    let term_freq = doc_tf
                        .and_then(|tf| tf.get(token.as_str()))
                        .copied()
                        .unwrap_or(0) as f32;
                    if term_freq == 0.0 {
                        continue;
                    }
                    let doc_freq = *df.get(token.as_str()).unwrap_or(&0) as f32;
                    let idf = ((self.total_docs as f32 - doc_freq + 0.5) / (doc_freq + 0.5) + 1.0).ln();
                    let tf_norm = (term_freq * (K1 + 1.0)) / (term_freq + K1 * (1.0 - B + B * doc_len / avg_doc_len));
                    score += idf * tf_norm;
                }

                (*id, score)
            })
            .collect();

        results.sort_by(|a, b| b.1.partial_cmp(&a.1).unwrap_or(std::cmp::Ordering::Equal));
        results.truncate(limit);
        results
    }

    pub fn len(&self) -> usize {
        self.total_docs
    }

    pub fn is_empty(&self) -> bool {
        self.total_docs == 0
    }

    /// Save the index to a file
    pub fn save(&self, path: impl AsRef<Path>) -> EngramResult<()> {
        let snapshot = KeywordSnapshot {
            inverted_index: self.inverted_index.clone(),
            doc_lengths: self.doc_lengths.clone(),
            doc_term_freqs: self.doc_term_freqs.clone(),
            total_docs: self.total_docs,
            total_tokens: self.total_tokens,
        };
        let bytes = bincode::serialize(&snapshot)
            .map_err(|e| EngramError::Serialization(e.to_string()))?;
        let mut file = std::fs::File::create(path.as_ref())?;
        file.write_all(&bytes)?;
        Ok(())
    }

    /// Load the index from a file
    pub fn load(path: impl AsRef<Path>) -> EngramResult<Self> {
        let mut file = std::fs::File::open(path.as_ref())?;
        let mut bytes = Vec::new();
        file.read_to_end(&mut bytes)?;
        let snapshot: KeywordSnapshot = bincode::deserialize(&bytes)
            .map_err(|e| EngramError::Serialization(e.to_string()))?;
        Ok(Self {
            inverted_index: snapshot.inverted_index,
            doc_lengths: snapshot.doc_lengths,
            doc_term_freqs: snapshot.doc_term_freqs,
            total_docs: snapshot.total_docs,
            total_tokens: snapshot.total_tokens,
        })
    }
}

#[derive(serde::Serialize, serde::Deserialize)]
struct KeywordSnapshot {
    inverted_index: HashMap<String, Vec<MemoryId>>,
    doc_lengths: HashMap<MemoryId, usize>,
    doc_term_freqs: HashMap<MemoryId, HashMap<String, usize>>,
    total_docs: usize,
    total_tokens: usize,
}

impl Default for KeywordIndex {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use uuid::Uuid;

    #[test]
    fn test_tokenize() {
        let tokens = tokenize("Hello, World! This is a test.");
        assert!(tokens.contains(&"hello".to_string()));
        assert!(tokens.contains(&"world".to_string()));
        assert!(tokens.contains(&"test".to_string()));
    }

    #[test]
    fn test_keyword_overlap() {
        let score = keyword_overlap_score("the user is vegetarian and likes pasta", "vegetarian food");
        assert!(score > 0.0);

        let score2 = keyword_overlap_score("quantum physics equations", "vegetarian food");
        assert_eq!(score2, 0.0);
    }

    #[test]
    fn test_keyword_index() {
        let mut index = KeywordIndex::new();

        let id1 = Uuid::now_v7();
        let id2 = Uuid::now_v7();
        let id3 = Uuid::now_v7();

        index.add(id1, "the user is vegetarian and allergic to nuts");
        index.add(id2, "booked a flight to paris for next week");
        index.add(id3, "user prefers vegetarian restaurants");

        let results = index.search("vegetarian food preferences", 10);
        assert!(results.len() >= 2);
        // Vegetarian-related docs should score higher
        assert!(results.iter().any(|(id, _)| *id == id1));
        assert!(results.iter().any(|(id, _)| *id == id3));
    }

    #[test]
    fn test_keyword_index_remove() {
        let mut index = KeywordIndex::new();
        let id = Uuid::now_v7();

        index.add(id, "test document");
        assert_eq!(index.len(), 1);

        index.remove(&id);
        assert_eq!(index.len(), 0);
    }
}
