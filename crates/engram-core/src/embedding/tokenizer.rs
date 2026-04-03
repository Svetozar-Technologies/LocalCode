/// Simple whitespace tokenizer for keyword extraction.
///
/// For production use with ONNX MiniLM, this would be replaced
/// with the HuggingFace tokenizers library for WordPiece tokenization.

/// Tokenize text for keyword matching (lowercased, stopwords removed)
pub fn tokenize_keywords(text: &str) -> Vec<String> {
    let stopwords = [
        "a", "an", "the", "is", "are", "was", "were", "be", "been", "being",
        "have", "has", "had", "do", "does", "did", "will", "would", "could",
        "should", "may", "might", "shall", "can", "to", "of", "in", "for",
        "on", "with", "at", "by", "from", "as", "into", "through", "during",
        "before", "after", "above", "below", "between", "out", "off", "over",
        "under", "again", "further", "then", "once", "here", "there", "when",
        "where", "why", "how", "all", "each", "every", "both", "few", "more",
        "most", "other", "some", "such", "no", "nor", "not", "only", "own",
        "same", "so", "than", "too", "very", "just", "because", "but", "and",
        "or", "if", "while", "about", "up", "it", "its", "he", "she", "they",
        "them", "this", "that", "these", "those", "i", "me", "my", "we", "our",
    ];

    text.to_lowercase()
        .split(|c: char| !c.is_alphanumeric())
        .filter(|w| !w.is_empty() && w.len() > 1 && !stopwords.contains(w))
        .map(String::from)
        .collect()
}

/// Extract n-grams from text
pub fn extract_ngrams(text: &str, n: usize) -> Vec<String> {
    let words = tokenize_keywords(text);
    if words.len() < n {
        return vec![words.join(" ")];
    }

    words
        .windows(n)
        .map(|w| w.join(" "))
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_tokenize_keywords() {
        let tokens = tokenize_keywords("The user is a vegetarian who likes Italian food");
        assert!(tokens.contains(&"user".to_string()));
        assert!(tokens.contains(&"vegetarian".to_string()));
        assert!(tokens.contains(&"italian".to_string()));
        assert!(tokens.contains(&"food".to_string()));
        // Stopwords should be removed
        assert!(!tokens.contains(&"the".to_string()));
        assert!(!tokens.contains(&"is".to_string()));
        assert!(!tokens.contains(&"a".to_string()));
    }

    #[test]
    fn test_ngrams() {
        let ngrams = extract_ngrams("user likes vegetarian food", 2);
        assert!(ngrams.contains(&"user likes".to_string()));
        assert!(ngrams.contains(&"likes vegetarian".to_string()));
    }
}
