use git2::Repository;
use serde::Serialize;

use crate::CoreResult;

#[derive(Debug, Serialize, Clone)]
pub struct BlameLine {
    pub line: usize,
    pub hash: String,
    pub author: String,
    pub date: String,
    pub content: String,
}

pub fn git_blame(repo_path: &str, file_path: &str) -> CoreResult<Vec<BlameLine>> {
    let repo = Repository::discover(repo_path)?;

    let blame = repo.blame_file(std::path::Path::new(file_path), None)?;

    let content = std::fs::read_to_string(
        crate::fs::resolve_path(file_path, repo_path),
    )?;
    let lines: Vec<&str> = content.lines().collect();

    let mut result = Vec::new();

    for (i, hunk_line) in lines.iter().enumerate() {
        if let Some(hunk) = blame.get_line(i + 1) {
            let sig = hunk.final_signature();
            result.push(BlameLine {
                line: i + 1,
                hash: hunk.final_commit_id().to_string()[..8].to_string(),
                author: sig.name().unwrap_or("unknown").to_string(),
                date: format!("{}", hunk.final_start_line()),
                content: hunk_line.to_string(),
            });
        }
    }

    Ok(result)
}
