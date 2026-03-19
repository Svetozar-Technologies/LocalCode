use localcode_core::git;

#[tauri::command]
pub fn git_status(path: String) -> Result<Vec<git::GitFileStatus>, String> {
    git::git_status(&path).map_err(|e| e.to_string())
}

#[tauri::command]
pub fn git_branch(path: String) -> Result<String, String> {
    git::git_branch(&path).map_err(|e| e.to_string())
}

#[tauri::command]
pub fn git_log(path: String, count: usize) -> Result<Vec<git::GitLogEntry>, String> {
    git::git_log(&path, count).map_err(|e| e.to_string())
}

#[tauri::command]
pub fn git_diff(path: String) -> Result<String, String> {
    git::git_diff(&path).map_err(|e| e.to_string())
}

#[tauri::command]
pub fn git_add(path: String, files: Vec<String>) -> Result<(), String> {
    git::staging::git_add(&path, &files).map_err(|e| e.to_string())
}

#[tauri::command]
pub fn git_add_all(path: String) -> Result<(), String> {
    git::staging::git_add_all(&path).map_err(|e| e.to_string())
}

#[tauri::command]
pub fn git_commit(path: String, message: String) -> Result<String, String> {
    git::staging::git_commit(&path, &message).map_err(|e| e.to_string())
}

#[tauri::command]
pub fn git_push(path: String, remote: String, branch: String) -> Result<String, String> {
    git::remote::git_push(&path, &remote, &branch).map_err(|e| e.to_string())
}

#[tauri::command]
pub fn git_pull(path: String, remote: String, branch: String) -> Result<String, String> {
    git::remote::git_pull(&path, &remote, &branch).map_err(|e| e.to_string())
}

#[tauri::command]
pub fn git_list_branches(path: String) -> Result<Vec<String>, String> {
    git::remote::git_list_branches(&path).map_err(|e| e.to_string())
}

#[tauri::command]
pub fn git_unstage(path: String, files: Vec<String>) -> Result<(), String> {
    git::staging::git_unstage(&path, &files).map_err(|e| e.to_string())
}

#[tauri::command]
pub fn git_init(path: String) -> Result<(), String> {
    git::staging::git_init(&path).map_err(|e| e.to_string())
}

#[tauri::command]
pub fn git_blame(path: String, file_path: String) -> Result<Vec<git::BlameLine>, String> {
    git::blame::git_blame(&path, &file_path).map_err(|e| e.to_string())
}
