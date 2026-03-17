use ignore::WalkBuilder;
use serde::Serialize;
use std::fs;

#[derive(Debug, Serialize)]
pub struct SearchResult {
    pub file: String,
    pub line: usize,
    pub column: usize,
    pub content: String,
    #[serde(rename = "matchLength")]
    pub match_length: usize,
}

#[tauri::command]
pub fn search_files(path: String, query: String) -> Result<Vec<String>, String> {
    let query_lower = query.to_lowercase();
    let mut results = Vec::new();

    let walker = WalkBuilder::new(&path)
        .hidden(true)
        .git_ignore(true)
        .build();

    for entry in walker {
        let entry = entry.map_err(|e| e.to_string())?;
        if let Some(name) = entry.file_name().to_str() {
            if name.to_lowercase().contains(&query_lower) {
                results.push(entry.path().to_string_lossy().to_string());
            }
        }
        if results.len() >= 100 {
            break;
        }
    }

    Ok(results)
}

#[tauri::command]
pub fn search_content(path: String, pattern: String) -> Result<Vec<SearchResult>, String> {
    let pattern_lower = pattern.to_lowercase();
    let mut results = Vec::new();

    let walker = WalkBuilder::new(&path)
        .hidden(true)
        .git_ignore(true)
        .build();

    for entry in walker {
        let entry = entry.map_err(|e| e.to_string())?;
        let entry_path = entry.path();

        if !entry_path.is_file() {
            continue;
        }

        // Skip binary files by checking extension
        if let Some(ext) = entry_path.extension().and_then(|e| e.to_str()) {
            if matches!(
                ext.to_lowercase().as_str(),
                "png" | "jpg" | "jpeg" | "gif" | "ico" | "svg" | "woff" | "woff2"
                    | "ttf" | "eot" | "mp3" | "mp4" | "avi" | "pdf" | "zip"
                    | "tar" | "gz" | "exe" | "dll" | "so" | "dylib" | "bin"
                    | "gguf"
            ) {
                continue;
            }
        }

        // Read file and search
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

        if results.len() >= 500 {
            break;
        }
    }

    Ok(results)
}
