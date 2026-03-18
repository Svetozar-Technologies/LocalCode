use serde::Serialize;
use std::fs;
use std::path::Path;

use crate::CoreResult;

#[derive(Debug, Serialize, Clone)]
pub struct FileEntry {
    pub name: String,
    pub path: String,
    pub is_dir: bool,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub children: Option<Vec<FileEntry>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub expanded: Option<bool>,
}

fn should_ignore(name: &str) -> bool {
    matches!(
        name,
        ".git"
            | "node_modules"
            | "target"
            | ".DS_Store"
            | "__pycache__"
            | ".venv"
            | "venv"
            | ".mypy_cache"
            | ".pytest_cache"
            | ".next"
            | "dist"
            | "build"
            | ".idea"
            | ".vscode"
            | "Thumbs.db"
    )
}

pub fn read_dir(path: &str) -> CoreResult<Vec<FileEntry>> {
    let path = Path::new(path);
    if !path.exists() {
        return Err(crate::CoreError::Other(format!(
            "Path does not exist: {}",
            path.display()
        )));
    }
    if !path.is_dir() {
        return Err(crate::CoreError::Other(format!(
            "Path is not a directory: {}",
            path.display()
        )));
    }

    let mut entries: Vec<FileEntry> = Vec::new();

    for entry in fs::read_dir(path)? {
        let entry = entry?;
        let name = entry.file_name().to_string_lossy().to_string();

        if should_ignore(&name) {
            continue;
        }

        let file_type = entry.file_type()?;
        let entry_path = entry.path().to_string_lossy().to_string();

        entries.push(FileEntry {
            name,
            path: entry_path,
            is_dir: file_type.is_dir(),
            children: if file_type.is_dir() {
                Some(Vec::new())
            } else {
                None
            },
            expanded: Some(false),
        });
    }

    entries.sort_by(|a, b| {
        if a.is_dir == b.is_dir {
            a.name.to_lowercase().cmp(&b.name.to_lowercase())
        } else if a.is_dir {
            std::cmp::Ordering::Less
        } else {
            std::cmp::Ordering::Greater
        }
    });

    Ok(entries)
}

pub fn read_file(path: &str) -> CoreResult<String> {
    Ok(fs::read_to_string(path)?)
}

pub fn write_file(path: &str, content: &str) -> CoreResult<()> {
    Ok(fs::write(path, content)?)
}

pub fn create_file(path: &str) -> CoreResult<()> {
    fs::File::create(path)?;
    Ok(())
}

pub fn create_dir(path: &str) -> CoreResult<()> {
    Ok(fs::create_dir_all(path)?)
}

pub fn delete_entry(path: &str) -> CoreResult<()> {
    let p = Path::new(path);
    if p.is_dir() {
        fs::remove_dir_all(p)?;
    } else {
        fs::remove_file(p)?;
    }
    Ok(())
}

pub fn rename_entry(old_path: &str, new_path: &str) -> CoreResult<()> {
    Ok(fs::rename(old_path, new_path)?)
}

pub fn edit_file(path: &str, old_text: &str, new_text: &str) -> CoreResult<bool> {
    let content = fs::read_to_string(path)?;
    if content.contains(old_text) {
        let updated = content.replace(old_text, new_text);
        fs::write(path, &updated)?;
        Ok(true)
    } else {
        Ok(false)
    }
}

pub fn resolve_path(path: &str, project_path: &str) -> String {
    if path.starts_with('/') {
        path.to_string()
    } else {
        format!("{}/{}", project_path, path)
    }
}
