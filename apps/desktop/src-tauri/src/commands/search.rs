use localcode_core::search;

#[tauri::command]
pub fn search_files(path: String, query: String) -> Result<Vec<String>, String> {
    search::search_files(&path, &query, 100).map_err(|e| e.to_string())
}

#[tauri::command]
pub fn search_content(path: String, pattern: String) -> Result<Vec<search::SearchResult>, String> {
    search::search_content(&path, &pattern, 500).map_err(|e| e.to_string())
}
