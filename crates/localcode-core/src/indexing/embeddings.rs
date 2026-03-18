use crate::CoreResult;
use crate::CoreError;

/// Compute embeddings using a provider's embed() method, or fallback to simple TF-IDF
pub fn simple_embed(text: &str) -> Vec<f32> {
    // Simple bag-of-words embedding as fallback when no model is available
    // This creates a fixed-size vector from word frequencies
    let words: Vec<&str> = text.split_whitespace().collect();
    let mut embedding = vec![0.0f32; 128];

    for word in &words {
        let hash = simple_hash(word);
        let idx = (hash % 128) as usize;
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
