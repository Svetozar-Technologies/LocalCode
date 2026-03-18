use ignore::WalkBuilder;
use serde::Serialize;
use std::fs;

use crate::CoreResult;

#[derive(Debug, Serialize, Clone)]
pub struct SearchResult {
    pub file: String,
    pub line: usize,
    pub column: usize,
    pub content: String,
    #[serde(rename = "matchLength")]
    pub match_length: usize,
}

const BINARY_EXTENSIONS: &[&str] = &[
    "png", "jpg", "jpeg", "gif", "ico", "svg", "woff", "woff2", "ttf", "eot", "mp3", "mp4",
    "avi", "pdf", "zip", "tar", "gz", "exe", "dll", "so", "dylib", "bin", "gguf",
];

pub fn search_content(
    path: &str,
    pattern: &str,
    max_results: usize,
) -> CoreResult<Vec<SearchResult>> {
    let pattern_lower = pattern.to_lowercase();
    let mut results = Vec::new();

    let walker = WalkBuilder::new(path)
        .hidden(true)
        .git_ignore(true)
        .build();

    for entry in walker {
        let entry = entry.map_err(|e| crate::CoreError::Other(e.to_string()))?;
        let entry_path = entry.path();

        if !entry_path.is_file() {
            continue;
        }

        if let Some(ext) = entry_path.extension().and_then(|e| e.to_str()) {
            if BINARY_EXTENSIONS.contains(&ext.to_lowercase().as_str()) {
                continue;
            }
        }

        if let Ok(content) = fs::read_to_string(entry_path) {
            for (line_num, line) in content.lines().enumerate() {
                let line_lower = line.to_lowercase();
                if let Some(col) = line_lower.find(&pattern_lower) {
                    results.push(SearchResult {
                        file: entry_path.to_string_lossy().to_string(),
                        line: line_num + 1,
                        column: col + 1,
                        content: line.trim().to_string(),
                        match_length: pattern.len(),
                    });
                }
            }
        }

        if results.len() >= max_results {
            break;
        }
    }

    Ok(results)
}
