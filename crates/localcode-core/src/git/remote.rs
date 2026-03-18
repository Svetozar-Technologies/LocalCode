use std::process::Command;

use crate::CoreResult;

pub fn git_push(path: &str, remote: &str, branch: &str) -> CoreResult<String> {
    let output = Command::new("git")
        .args(["push", remote, branch])
        .current_dir(path)
        .output()?;

    let stdout = String::from_utf8_lossy(&output.stdout).to_string();
    let stderr = String::from_utf8_lossy(&output.stderr).to_string();

    if output.status.success() {
        Ok(format!("{}{}", stdout, stderr))
    } else {
        Err(crate::CoreError::Other(format!("git push failed: {}", stderr)))
    }
}

pub fn git_pull(path: &str, remote: &str, branch: &str) -> CoreResult<String> {
    let output = Command::new("git")
        .args(["pull", remote, branch])
        .current_dir(path)
        .output()?;

    let stdout = String::from_utf8_lossy(&output.stdout).to_string();
    let stderr = String::from_utf8_lossy(&output.stderr).to_string();

    if output.status.success() {
        Ok(format!("{}{}", stdout, stderr))
    } else {
        Err(crate::CoreError::Other(format!("git pull failed: {}", stderr)))
    }
}

pub fn git_fetch(path: &str, remote: &str) -> CoreResult<String> {
    let output = Command::new("git")
        .args(["fetch", remote])
        .current_dir(path)
        .output()?;

    let stdout = String::from_utf8_lossy(&output.stdout).to_string();
    let stderr = String::from_utf8_lossy(&output.stderr).to_string();

    if output.status.success() {
        Ok(format!("{}{}", stdout, stderr))
    } else {
        Err(crate::CoreError::Other(format!("git fetch failed: {}", stderr)))
    }
}

pub fn git_create_branch(path: &str, name: &str) -> CoreResult<()> {
    let repo = git2::Repository::discover(path)?;
    let head = repo.head()?;
    let commit = head.peel_to_commit()?;
    repo.branch(name, &commit, false)?;
    Ok(())
}

pub fn git_switch_branch(path: &str, name: &str) -> CoreResult<()> {
    let output = Command::new("git")
        .args(["checkout", name])
        .current_dir(path)
        .output()?;

    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr).to_string();
        return Err(crate::CoreError::Other(format!(
            "git checkout failed: {}",
            stderr
        )));
    }
    Ok(())
}

pub fn git_list_branches(path: &str) -> CoreResult<Vec<String>> {
    let repo = git2::Repository::discover(path)?;
    let branches = repo.branches(Some(git2::BranchType::Local))?;

    let mut names = Vec::new();
    for branch in branches {
        let (branch, _) = branch?;
        if let Some(name) = branch.name()? {
            names.push(name.to_string());
        }
    }
    Ok(names)
}
