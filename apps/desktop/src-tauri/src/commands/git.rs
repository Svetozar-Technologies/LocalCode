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
pub fn git_log(path: String, count: usize, file_path: Option<String>) -> Result<Vec<git::GitLogEntry>, String> {
    let entries = git::git_log(&path, count).map_err(|e| e.to_string())?;
    if let Some(ref fp) = file_path {
        // Filter by running git log -- <file> via command
        Ok(filter_commits_by_file(&path, entries, fp))
    } else {
        Ok(entries)
    }
}

#[tauri::command]
pub fn git_file_log(path: String, file_path: String, count: usize) -> Result<Vec<git::GitLogEntry>, String> {
    let entries = git::git_log(&path, count).map_err(|e| e.to_string())?;
    Ok(filter_commits_by_file(&path, entries, &file_path))
}

/// Filter git log entries to only those that touched a specific file.
/// Uses `git log --format=%H -- <file>` to get the list of relevant commit hashes.
fn filter_commits_by_file(repo_path: &str, entries: Vec<git::GitLogEntry>, file_path: &str) -> Vec<git::GitLogEntry> {
    // Try to get a relative path
    let rel_path = if file_path.starts_with(repo_path) {
        file_path.strip_prefix(repo_path).unwrap_or(file_path).trim_start_matches('/')
    } else {
        file_path
    };

    // Run git log to get hashes of commits that touched this file
    let output = std::process::Command::new("git")
        .args(["log", "--format=%H", "--follow", "--", rel_path])
        .current_dir(repo_path)
        .output();

    match output {
        Ok(out) if out.status.success() => {
            let stdout = String::from_utf8_lossy(&out.stdout);
            let relevant_hashes: std::collections::HashSet<&str> = stdout.lines().collect();
            entries.into_iter().filter(|e| relevant_hashes.contains(e.hash.as_str())).collect()
        }
        _ => entries, // Fallback: return all
    }
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
