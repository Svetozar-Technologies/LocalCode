use git2::Repository;
use serde::Serialize;

use crate::CoreResult;

#[derive(Debug, Serialize, Clone)]
pub struct GitFileStatus {
    pub path: String,
    pub status: String,
}

#[derive(Debug, Serialize, Clone)]
pub struct GitLogEntry {
    pub hash: String,
    pub message: String,
    pub author: String,
    pub timestamp: i64,
}

pub fn git_status(path: &str) -> CoreResult<Vec<GitFileStatus>> {
    let repo = Repository::discover(path)?;
    let statuses = repo.statuses(None)?;

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

pub fn git_branch(path: &str) -> CoreResult<String> {
    let repo = Repository::discover(path)?;
    let head = repo.head()?;
    Ok(head.shorthand().unwrap_or("HEAD").to_string())
}

pub fn git_log(path: &str, count: usize) -> CoreResult<Vec<GitLogEntry>> {
    let repo = Repository::discover(path)?;
    let mut revwalk = repo.revwalk()?;
    revwalk.push_head()?;

    let mut entries = Vec::new();

    for oid in revwalk {
        let oid = oid?;
        let commit = repo.find_commit(oid)?;

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

pub fn git_diff(path: &str) -> CoreResult<String> {
    let repo = Repository::discover(path)?;
    let diff = repo.diff_index_to_workdir(None, None)?;

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
    })?;

    Ok(diff_text)
}
