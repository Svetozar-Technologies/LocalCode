use std::collections::HashMap;

/// Compute embeddings using bigram-enhanced bag-of-words (256-dim)
pub fn simple_embed(text: &str) -> Vec<f32> {
    let words: Vec<&str> = text.split_whitespace().collect();
    let mut embedding = vec![0.0f32; 256];

    // Unigram hashing (first 128 dims)
    for word in &words {
        let hash = simple_hash(word);
        let idx = (hash % 128) as usize;
        embedding[idx] += 1.0;
    }

    // Bigram hashing (dims 128-255) for phrase-level signal
    for pair in words.windows(2) {
        let bigram = format!("{} {}", pair[0], pair[1]);
        let hash = simple_hash(&bigram);
        let idx = 128 + (hash % 128) as usize;
        embedding[idx] += 1.0;
    }

    // Normalize
    let magnitude: f32 = embedding.iter().map(|x| x * x).sum::<f32>().sqrt();
    if magnitude > 0.0 {
        for val in &mut embedding {
            *val /= magnitude;
        }
    }

    embedding
}

fn simple_hash(s: &str) -> u64 {
    let mut hash: u64 = 5381;
    for byte in s.bytes() {
        hash = hash.wrapping_mul(33).wrapping_add(byte as u64);
    }
    hash
}

/// Cosine similarity between two vectors
pub fn cosine_similarity(a: &[f32], b: &[f32]) -> f32 {
    if a.len() != b.len() {
        return 0.0;
    }

    let dot: f32 = a.iter().zip(b.iter()).map(|(x, y)| x * y).sum();
    let mag_a: f32 = a.iter().map(|x| x * x).sum::<f32>().sqrt();
    let mag_b: f32 = b.iter().map(|x| x * x).sum::<f32>().sqrt();

    if mag_a == 0.0 || mag_b == 0.0 {
        0.0
    } else {
        dot / (mag_a * mag_b)
    }
}

/// Tokenize text into lowercase words for BM25
fn tokenize(text: &str) -> Vec<String> {
    text.split(|c: char| !c.is_alphanumeric() && c != '_')
        .filter(|w| !w.is_empty())
        .map(|w| w.to_lowercase())
        .collect()
}

/// BM25 scoring for a query against a document
pub fn bm25_score(
    query: &str,
    document: &str,
    avg_doc_len: f32,
    total_docs: usize,
    doc_freqs: &HashMap<String, usize>,
) -> f32 {
    let k1: f32 = 1.2;
    let b: f32 = 0.75;

    let query_terms = tokenize(query);
    let doc_terms = tokenize(document);
    let doc_len = doc_terms.len() as f32;

    if doc_len == 0.0 || avg_doc_len == 0.0 || total_docs == 0 {
        return 0.0;
    }

    // Count term frequencies in document
    let mut tf_map: HashMap<&str, usize> = HashMap::new();
    for term in &doc_terms {
        *tf_map.entry(term.as_str()).or_insert(0) += 1;
    }

    let mut score: f32 = 0.0;
    for term in &query_terms {
        let tf = *tf_map.get(term.as_str()).unwrap_or(&0) as f32;
        let df = *doc_freqs.get(term.as_str()).unwrap_or(&0);

        if tf == 0.0 || df == 0 {
            continue;
        }

        // IDF component: log((N - df + 0.5) / (df + 0.5) + 1)
        let idf = ((total_docs as f32 - df as f32 + 0.5) / (df as f32 + 0.5) + 1.0).ln();

        // TF component with length normalization
        let tf_norm = (tf * (k1 + 1.0)) / (tf + k1 * (1.0 - b + b * (doc_len / avg_doc_len)));

        score += idf * tf_norm;
    }

    score
}

/// Hybrid search combining cosine similarity and BM25
/// Returns Vec<(combined_score, index)> sorted descending
pub fn hybrid_search(
    query_embedding: &[f32],
    query_text: &str,
    entries: &[super::store::IndexEntry],
    avg_doc_len: f32,
    total_docs: usize,
    doc_freqs: &HashMap<String, usize>,
    top_k: usize,
) -> Vec<(f32, usize)> {
    let cosine_weight: f32 = 0.4;
    let bm25_weight: f32 = 0.6;

    let mut scored: Vec<(f32, usize)> = entries
        .iter()
        .enumerate()
        .map(|(idx, entry)| {
            let cos_sim = cosine_similarity(query_embedding, &entry.embedding);
            let bm25 = bm25_score(query_text, &entry.content, avg_doc_len, total_docs, doc_freqs);

            // Normalize BM25 to roughly 0-1 range (cap at 10 for normalization)
            let bm25_norm = (bm25 / 10.0).min(1.0);

            let combined = cosine_weight * cos_sim + bm25_weight * bm25_norm;
            (combined, idx)
        })
        .collect();

    scored.sort_by(|a, b| b.0.partial_cmp(&a.0).unwrap_or(std::cmp::Ordering::Equal));
    scored.into_iter().take(top_k).collect()
}

/// Compute term document frequencies from a set of documents
pub fn compute_doc_freqs(documents: &[&str]) -> HashMap<String, usize> {
    let mut freqs: HashMap<String, usize> = HashMap::new();
    for doc in documents {
        let terms: std::collections::HashSet<String> = tokenize(doc).into_iter().collect();
        for term in terms {
            *freqs.entry(term).or_insert(0) += 1;
        }
    }
    freqs
}

/// Compute average document length
pub fn compute_avg_doc_len(documents: &[&str]) -> f32 {
    if documents.is_empty() {
        return 0.0;
    }
    let total: usize = documents.iter().map(|d| tokenize(d).len()).sum();
    total as f32 / documents.len() as f32
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_simple_embed_dimension() {
        let emb = simple_embed("hello world test");
        assert_eq!(emb.len(), 256);
    }

    #[test]
    fn test_simple_embed_normalized() {
        let emb = simple_embed("fn main() { println!(\"hello\"); }");
        let mag: f32 = emb.iter().map(|x| x * x).sum::<f32>().sqrt();
        assert!((mag - 1.0).abs() < 0.01);
    }

    #[test]
    fn test_cosine_similarity_identical() {
        let a = simple_embed("hello world");
        let sim = cosine_similarity(&a, &a);
        assert!((sim - 1.0).abs() < 0.001);
    }

    #[test]
    fn test_bm25_score_basic() {
        let mut doc_freqs = HashMap::new();
        doc_freqs.insert("hello".to_string(), 2);
        doc_freqs.insert("world".to_string(), 1);

        let score = bm25_score("hello", "hello world hello", 10.0, 5, &doc_freqs);
        assert!(score > 0.0);
    }

    #[test]
    fn test_bm25_no_match() {
        let doc_freqs = HashMap::new();
        let score = bm25_score("xyz", "hello world", 10.0, 5, &doc_freqs);
        assert_eq!(score, 0.0);
    }

    #[test]
    fn test_compute_doc_freqs() {
        let docs = vec!["hello world", "hello rust", "world test"];
        let freqs = compute_doc_freqs(&docs);
        assert_eq!(freqs["hello"], 2);
        assert_eq!(freqs["world"], 2);
        assert_eq!(freqs["rust"], 1);
    }

    #[test]
    fn test_compute_avg_doc_len() {
        let docs = vec!["hello world", "a b c d"];
        let avg = compute_avg_doc_len(&docs);
        assert!((avg - 3.0).abs() < 0.01);
    }
}
