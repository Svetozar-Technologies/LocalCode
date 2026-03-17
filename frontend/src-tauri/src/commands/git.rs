use git2::Repository;
use serde::Serialize;

#[derive(Debug, Serialize)]
pub struct GitFileStatus {
    pub path: String,
    pub status: String,
}

#[derive(Debug, Serialize)]
pub struct GitLogEntry {
    pub hash: String,
    pub message: String,
    pub author: String,
    pub timestamp: i64,
}

#[tauri::command]
pub fn git_status(path: String) -> Result<Vec<GitFileStatus>, String> {
    let repo = Repository::discover(&path).map_err(|e| e.to_string())?;
    let statuses = repo.statuses(None).map_err(|e| e.to_string())?;

    let mut results = Vec::new();

    for entry in statuses.iter() {
        let status = entry.status();
        let file_path = entry.path().unwrap_or("").to_string();

        let status_str = if status.is_wt_modified() || status.is_index_modified() {
            "modified"
        } else if status.is_wt_new() {
            "untracked"
        } else if status.is_index_new() {
            "added"
        } else if status.is_wt_deleted() || status.is_index_deleted() {
            "deleted"
        } else if status.is_wt_renamed() || status.is_index_renamed() {
            "renamed"
        } else {
            continue;
        };

        results.push(GitFileStatus {
            path: file_path,
            status: status_str.to_string(),
        });
    }

    Ok(results)
}

#[tauri::command]
pub fn git_branch(path: String) -> Result<String, String> {
    let repo = Repository::discover(&path).map_err(|e| e.to_string())?;
    let head = repo.head().map_err(|e| e.to_string())?;

    Ok(head
        .shorthand()
        .unwrap_or("HEAD")
        .to_string())
}

#[tauri::command]
pub fn git_log(path: String, count: usize) -> Result<Vec<GitLogEntry>, String> {
    let repo = Repository::discover(&path).map_err(|e| e.to_string())?;
    let mut revwalk = repo.revwalk().map_err(|e| e.to_string())?;
    revwalk.push_head().map_err(|e| e.to_string())?;

    let mut entries = Vec::new();

    for oid in revwalk {
        let oid = oid.map_err(|e| e.to_string())?;
        let commit = repo.find_commit(oid).map_err(|e| e.to_string())?;

        entries.push(GitLogEntry {
            hash: oid.to_string()[..8].to_string(),
            message: commit.summary().unwrap_or("").to_string(),
            author: commit.author().name().unwrap_or("").to_string(),
            timestamp: commit.time().seconds(),
        });

        if entries.len() >= count {
            break;
        }
    }

    Ok(entries)
}

#[tauri::command]
pub fn git_diff(path: String) -> Result<String, String> {
    let repo = Repository::discover(&path).map_err(|e| e.to_string())?;
    let diff = repo
        .diff_index_to_workdir(None, None)
        .map_err(|e| e.to_string())?;

    let mut diff_text = String::new();
    diff.print(git2::DiffFormat::Patch, |_delta, _hunk, line| {
        let prefix = match line.origin() {
            '+' => "+",
            '-' => "-",
            ' ' => " ",
            _ => "",
        };
        diff_text.push_str(prefix);
        diff_text.push_str(&String::from_utf8_lossy(line.content()));
        true
    })
    .map_err(|e| e.to_string())?;

    Ok(diff_text)
}
