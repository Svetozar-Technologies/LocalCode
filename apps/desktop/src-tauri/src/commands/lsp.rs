use localcode_core::lsp::LspManager;
use std::sync::Arc;

pub type LspState = Arc<LspManager>;

pub fn create_lsp_manager() -> LspState {
    Arc::new(LspManager::new())
}

#[derive(serde::Serialize)]
pub struct LspLocationResult {
    pub uri: String,
    pub line: u32,
    pub character: u32,
    pub end_line: u32,
    pub end_character: u32,
}

#[derive(serde::Serialize)]
pub struct LspCompletionResult {
    pub label: String,
    pub kind: Option<u32>,
    pub detail: Option<String>,
    pub insert_text: Option<String>,
}

#[tauri::command]
pub fn lsp_start(
    project_path: String,
    language: String,
    state: tauri::State<'_, LspState>,
) -> Result<(), String> {
    state.set_project_path(&project_path);
    state.start(&language).map_err(|e| e.to_string())
}

#[tauri::command]
pub fn lsp_hover(
    file_path: String,
    line: u32,
    character: u32,
    language: String,
    state: tauri::State<'_, LspState>,
) -> Result<Option<String>, String> {
    state.hover(&language, &file_path, line, character).map_err(|e| e.to_string())
}

#[tauri::command]
pub fn lsp_definition(
    file_path: String,
    line: u32,
    character: u32,
    language: String,
    state: tauri::State<'_, LspState>,
) -> Result<Option<LspLocationResult>, String> {
    state.definition(&language, &file_path, line, character)
        .map(|opt| opt.map(|loc| LspLocationResult {
            uri: loc.uri,
            line: loc.range.start.line,
            character: loc.range.start.character,
            end_line: loc.range.end.line,
            end_character: loc.range.end.character,
        }))
        .map_err(|e| e.to_string())
}

#[tauri::command]
pub fn lsp_references(
    file_path: String,
    line: u32,
    character: u32,
    language: String,
    state: tauri::State<'_, LspState>,
) -> Result<Vec<LspLocationResult>, String> {
    state.references(&language, &file_path, line, character)
        .map(|locs| locs.into_iter().map(|loc| LspLocationResult {
            uri: loc.uri,
            line: loc.range.start.line,
            character: loc.range.start.character,
            end_line: loc.range.end.line,
            end_character: loc.range.end.character,
        }).collect())
        .map_err(|e| e.to_string())
}

#[tauri::command]
pub fn lsp_completions(
    file_path: String,
    line: u32,
    character: u32,
    language: String,
    state: tauri::State<'_, LspState>,
) -> Result<Vec<LspCompletionResult>, String> {
    state.completions(&language, &file_path, line, character)
        .map(|items| items.into_iter().map(|item| LspCompletionResult {
            label: item.label,
            kind: item.kind,
            detail: item.detail,
            insert_text: item.insert_text,
        }).collect())
        .map_err(|e| e.to_string())
}
