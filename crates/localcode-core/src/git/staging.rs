use git2::{Repository, Signature};

use crate::CoreResult;

pub fn git_init(path: &str) -> CoreResult<()> {
    Repository::init(path)?;
    Ok(())
}

pub fn git_add(path: &str, files: &[String]) -> CoreResult<()> {
    let repo = Repository::discover(path)?;
    let mut index = repo.index()?;

    for file in files {
        index.add_path(std::path::Path::new(file))?;
    }

    index.write()?;
    Ok(())
}

pub fn git_add_all(path: &str) -> CoreResult<()> {
    let repo = Repository::discover(path)?;
    let mut index = repo.index()?;

    index.add_all(["*"].iter(), git2::IndexAddOption::DEFAULT, None)?;
    index.write()?;
    Ok(())
}

pub fn git_commit(path: &str, message: &str) -> CoreResult<String> {
    let repo = Repository::discover(path)?;
    let mut index = repo.index()?;
    let oid = index.write_tree()?;
    let tree = repo.find_tree(oid)?;

    let sig = repo
        .signature()
        .unwrap_or_else(|_| Signature::now("LocalCode", "localcode@local").unwrap());

    let parent = if let Ok(head) = repo.head() {
        Some(repo.find_commit(head.target().unwrap())?)
    } else {
        None
    };

    let parents: Vec<&git2::Commit> = parent.iter().collect();

    let commit_oid = repo.commit(Some("HEAD"), &sig, &sig, message, &tree, &parents)?;

    Ok(commit_oid.to_string()[..8].to_string())
}

pub fn git_unstage(path: &str, files: &[String]) -> CoreResult<()> {
    let repo = Repository::discover(path)?;

    if let Ok(head) = repo.head() {
        let head_commit = head.peel_to_commit()?;
        let head_tree = head_commit.tree()?;
        repo.reset_default(Some(head_commit.as_object()), files.iter().map(|f| std::path::Path::new(f)))?;
        let _ = head_tree; // suppress unused warning
    } else {
        let mut index = repo.index()?;
        for file in files {
            index.remove_path(std::path::Path::new(file))?;
        }
        index.write()?;
    }

    Ok(())
}
