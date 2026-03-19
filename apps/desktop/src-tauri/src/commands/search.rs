use localcode_core::search;
use localcode_core::indexing::query;

#[tauri::command]
pub fn search_files(path: String, query: String) -> Result<Vec<String>, String> {
    search::search_files(&path, &query, 100).map_err(|e| e.to_string())
}

#[tauri::command]
pub fn search_content(path: String, pattern: String) -> Result<Vec<search::SearchResult>, String> {
    search::search_content(&path, &pattern, 500).map_err(|e| e.to_string())
}

#[tauri::command]
pub fn search_codebase(project_path: String, query: String, top_k: Option<usize>) -> Result<Vec<String>, String> {
    let _ = query::index_if_needed(&project_path, 300);
    query::query_codebase(&project_path, &query, top_k.unwrap_or(5)).map_err(|e| e.to_string())
}
