use ignore::WalkBuilder;
use serde::Serialize;
use std::fs;

use crate::CoreResult;

#[derive(Debug, Serialize, Clone)]
pub struct ReplaceResult {
    pub file: String,
    pub replacements: usize,
}

pub fn search_and_replace(
    path: &str,
    search: &str,
    replace: &str,
    max_files: usize,
) -> CoreResult<Vec<ReplaceResult>> {
    let mut results = Vec::new();

    let walker = WalkBuilder::new(path)
        .hidden(true)
        .git_ignore(true)
        .build();

    for entry in walker {
        let entry = entry.map_err(|e| crate::CoreError::Other(e.to_string()))?;
        let entry_path = entry.path();

        if !entry_path.is_file() {
            continue;
        }

        if let Ok(content) = fs::read_to_string(entry_path) {
            let count = content.matches(search).count();
            if count > 0 {
                let updated = content.replace(search, replace);
                fs::write(entry_path, &updated)?;
                results.push(ReplaceResult {
                    file: entry_path.to_string_lossy().to_string(),
                    replacements: count,
                });
            }
        }

        if results.len() >= max_files {
            break;
        }
    }

    Ok(results)
}
