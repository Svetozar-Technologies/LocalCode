use localcode_core::agent::chat::{ChatMessage, ChatSearchResult, ChatSession, ChatStore};
use std::sync::{Arc, Mutex};

pub type ChatManager = Arc<Mutex<ChatStore>>;

pub fn create_chat_manager() -> ChatManager {
    let store = ChatStore::new().expect("Failed to initialize ChatStore");
    Arc::new(Mutex::new(store))
}

#[tauri::command]
pub fn chat_create_session(
    project_path: String,
    title: String,
    state: tauri::State<'_, ChatManager>,
) -> Result<ChatSession, String> {
    let store = state.lock().map_err(|e| e.to_string())?;
    store
        .create_session(&project_path, &title)
        .map_err(|e| e.to_string())
}

#[tauri::command]
pub fn chat_list_sessions(
    project_path: Option<String>,
    limit: Option<usize>,
    state: tauri::State<'_, ChatManager>,
) -> Result<Vec<ChatSession>, String> {
    let store = state.lock().map_err(|e| e.to_string())?;
    store
        .list_sessions(project_path.as_deref(), limit.unwrap_or(50))
        .map_err(|e| e.to_string())
}

#[tauri::command]
pub fn chat_get_messages(
    session_id: String,
    state: tauri::State<'_, ChatManager>,
) -> Result<Vec<ChatMessage>, String> {
    let store = state.lock().map_err(|e| e.to_string())?;
    store.get_messages(&session_id).map_err(|e| e.to_string())
}

#[tauri::command]
pub fn chat_add_message(
    id: String,
    chat_session_id: String,
    role: String,
    content: String,
    timestamp: u64,
    agent_steps: Option<String>,
    state: tauri::State<'_, ChatManager>,
) -> Result<(), String> {
    let store = state.lock().map_err(|e| e.to_string())?;
    let msg = ChatMessage {
        id,
        chat_session_id,
        role,
        content,
        timestamp,
        agent_steps,
    };
    store.add_message(&msg).map_err(|e| e.to_string())
}

#[tauri::command]
pub fn chat_update_message(
    id: String,
    content: String,
    agent_steps: Option<String>,
    state: tauri::State<'_, ChatManager>,
) -> Result<bool, String> {
    let store = state.lock().map_err(|e| e.to_string())?;
    store
        .update_message_content(&id, &content, agent_steps.as_deref())
        .map_err(|e| e.to_string())
}

#[tauri::command]
pub fn chat_delete_session(
    id: String,
    state: tauri::State<'_, ChatManager>,
) -> Result<bool, String> {
    let store = state.lock().map_err(|e| e.to_string())?;
    store.delete_session(&id).map_err(|e| e.to_string())
}

#[tauri::command]
pub fn chat_update_session_title(
    id: String,
    title: String,
    state: tauri::State<'_, ChatManager>,
) -> Result<bool, String> {
    let store = state.lock().map_err(|e| e.to_string())?;
    store
        .update_session_title(&id, &title)
        .map_err(|e| e.to_string())
}

#[tauri::command]
pub fn chat_search(
    query: String,
    project_path: Option<String>,
    top_k: Option<usize>,
    state: tauri::State<'_, ChatManager>,
) -> Result<Vec<ChatSearchResult>, String> {
    let store = state.lock().map_err(|e| e.to_string())?;
    store
        .search_messages(&query, project_path.as_deref(), top_k.unwrap_or(10))
        .map_err(|e| e.to_string())
}
