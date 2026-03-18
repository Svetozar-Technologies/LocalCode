use git2::Repository;
use serde::Serialize;

use crate::CoreResult;

#[derive(Debug, Serialize, Clone)]
pub struct CommitDetail {
    pub hash: String,
    pub message: String,
    pub author: String,
    pub email: String,
    pub timestamp: i64,
    pub files_changed: Vec<FileChange>,
}

#[derive(Debug, Serialize, Clone)]
pub struct FileChange {
    pub path: String,
    pub status: String,
}

pub fn git_commit_detail(path: &str, hash: &str) -> CoreResult<CommitDetail> {
    let repo = Repository::discover(path)?;
    let oid = git2::Oid::from_str(hash)?;
    let commit = repo.find_commit(oid)?;

    let tree = commit.tree()?;
    let parent_tree = commit.parent(0).ok().and_then(|p| p.tree().ok());

    let diff = repo.diff_tree_to_tree(parent_tree.as_ref(), Some(&tree), None)?;

    let mut files_changed = Vec::new();
    diff.foreach(
        &mut |delta, _| {
            let path = delta
                .new_file()
                .path()
                .unwrap_or(std::path::Path::new(""))
                .to_string_lossy()
                .to_string();
            let status = match delta.status() {
                git2::Delta::Added => "added",
                git2::Delta::Deleted => "deleted",
                git2::Delta::Modified => "modified",
                git2::Delta::Renamed => "renamed",
                _ => "unknown",
            };
            files_changed.push(FileChange {
                path,
                status: status.to_string(),
            });
            true
        },
        None,
        None,
        None,
    )?;

    let author_sig = commit.author();
    let author_name = author_sig.name().unwrap_or("").to_string();
    let author_email = author_sig.email().unwrap_or("").to_string();

    Ok(CommitDetail {
        hash: commit.id().to_string(),
        message: commit.message().unwrap_or("").to_string(),
        author: author_name,
        email: author_email,
        timestamp: commit.time().seconds(),
        files_changed,
    })
}
