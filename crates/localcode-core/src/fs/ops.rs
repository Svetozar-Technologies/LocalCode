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
    // Auto-create parent directories
    if let Some(parent) = Path::new(path).parent() {
        if !parent.exists() {
            fs::create_dir_all(parent)?;
        }
    }
    Ok(fs::write(path, content)?)
}

pub fn create_file(path: &str) -> CoreResult<()> {
    // Auto-create parent directories
    if let Some(parent) = Path::new(path).parent() {
        if !parent.exists() {
            fs::create_dir_all(parent)?;
        }
    }
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

/// Resolve a path relative to the project root.
/// All paths are sandboxed to the project directory:
/// - Relative paths are joined with project_path
/// - Absolute paths are stripped to relative (e.g. "/src/main.rs" → "src/main.rs")
/// - `~` paths are treated as relative (stripped)
/// - Path traversal (`../`) beyond project root is blocked
pub fn resolve_path(path: &str, project_path: &str) -> String {
    use std::path::{Component, PathBuf};

    // Strip leading indicators that local LLMs sometimes produce
    let cleaned = path
        .trim()
        .strip_prefix("~/").unwrap_or(
            path.trim()
                .strip_prefix("~").unwrap_or(
                    path.trim()
                        .strip_prefix("/").unwrap_or(path.trim())
                )
        );

    // Handle empty or "." path
    if cleaned.is_empty() || cleaned == "." {
        return project_path.to_string();
    }

    // Build safe relative path — resolve ".." components but prevent escaping
    let mut resolved = PathBuf::new();
    for component in std::path::Path::new(cleaned).components() {
        match component {
            Component::Normal(c) => resolved.push(c),
            Component::CurDir => {} // skip "."
            Component::ParentDir => {
                // Allow going up within relative path, but not beyond root
                if !resolved.pop() {
                    // Already at root — ignore the ".."
                }
            }
            _ => {} // skip RootDir, Prefix
        }
    }

    format!("{}/{}", project_path, resolved.display())
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::TempDir;

    #[test]
    fn test_resolve_relative_path() {
        let base = "/project";
        let result = resolve_path("src/main.rs", base);
        assert_eq!(result, "/project/src/main.rs");
    }

    #[test]
    fn test_resolve_dot_path() {
        let base = "/project";
        let result = resolve_path(".", base);
        assert_eq!(result, "/project");
    }

    #[test]
    fn test_resolve_blocks_traversal() {
        let base = "/project";
        let result = resolve_path("../../../etc/passwd", base);
        assert!(result.starts_with("/project"));
        assert!(!result.contains(".."));
    }

    #[test]
    fn test_resolve_empty_path() {
        let base = "/project";
        let result = resolve_path("", base);
        assert_eq!(result, "/project");
    }

    #[test]
    fn test_write_file_creates_parents() {
        let dir = TempDir::new().unwrap();
        let path = dir.path().join("a/b/c/file.txt");
        write_file(path.to_str().unwrap(), "hello").unwrap();
        assert_eq!(std::fs::read_to_string(&path).unwrap(), "hello");
    }

    #[test]
    fn test_write_file_overwrites() {
        let dir = TempDir::new().unwrap();
        let path = dir.path().join("test.txt");
        write_file(path.to_str().unwrap(), "first").unwrap();
        write_file(path.to_str().unwrap(), "second").unwrap();
        assert_eq!(std::fs::read_to_string(&path).unwrap(), "second");
    }

    #[test]
    fn test_read_file_content() {
        let dir = TempDir::new().unwrap();
        let path = dir.path().join("test.txt");
        std::fs::write(&path, "hello world").unwrap();
        let content = read_file(path.to_str().unwrap()).unwrap();
        assert_eq!(content, "hello world");
    }

    #[test]
    fn test_create_file_new() {
        let dir = TempDir::new().unwrap();
        let path = dir.path().join("new.txt");
        create_file(path.to_str().unwrap()).unwrap();
        assert!(path.exists());
    }

    #[test]
    fn test_delete_entry_file() {
        let dir = TempDir::new().unwrap();
        let path = dir.path().join("to_delete.txt");
        std::fs::write(&path, "bye").unwrap();
        delete_entry(path.to_str().unwrap()).unwrap();
        assert!(!path.exists());
    }

    #[test]
    fn test_read_dir_returns_entries() {
        let dir = TempDir::new().unwrap();
        std::fs::write(dir.path().join("file1.txt"), "a").unwrap();
        std::fs::create_dir(dir.path().join("subdir")).unwrap();
        let entries = read_dir(dir.path().to_str().unwrap()).unwrap();
        assert!(entries.len() >= 2);
        assert!(entries.iter().any(|e| e.name == "file1.txt"));
        assert!(entries.iter().any(|e| e.name == "subdir" && e.is_dir));
    }

    #[test]
    fn test_edit_file_replacement() {
        let dir = TempDir::new().unwrap();
        let path = dir.path().join("edit.txt");
        std::fs::write(&path, "hello world").unwrap();
        let result = edit_file(path.to_str().unwrap(), "world", "rust").unwrap();
        assert!(result);
        assert_eq!(std::fs::read_to_string(&path).unwrap(), "hello rust");
    }

    #[test]
    fn test_rename_entry() {
        let dir = TempDir::new().unwrap();
        let old_path = dir.path().join("old.txt");
        let new_path = dir.path().join("new.txt");
        std::fs::write(&old_path, "content").unwrap();
        rename_entry(old_path.to_str().unwrap(), new_path.to_str().unwrap()).unwrap();
        assert!(!old_path.exists());
        assert!(new_path.exists());
    }
}
