use ignore::WalkBuilder;

use crate::CoreResult;

pub fn search_files(path: &str, query: &str, max_results: usize) -> CoreResult<Vec<String>> {
    let query_lower = query.to_lowercase();
    let mut results = Vec::new();

    let walker = WalkBuilder::new(path)
        .hidden(true)
        .git_ignore(true)
        .build();

    for entry in walker {
        let entry = entry.map_err(|e| crate::CoreError::Other(e.to_string()))?;
        if let Some(name) = entry.file_name().to_str() {
            if name.to_lowercase().contains(&query_lower) {
                results.push(entry.path().to_string_lossy().to_string());
            }
        }
        if results.len() >= max_results {
            break;
        }
    }

    Ok(results)
}

pub fn glob_files(path: &str, pattern: &str, max_results: usize) -> CoreResult<Vec<String>> {
    let walker = WalkBuilder::new(path)
        .hidden(true)
        .git_ignore(true)
        .build();

    let mut results = Vec::new();
    let pattern_lower = pattern.to_lowercase();

    for entry in walker {
        let entry = entry.map_err(|e| crate::CoreError::Other(e.to_string()))?;
        let path_str = entry.path().to_string_lossy().to_string();

        if path_str.to_lowercase().contains(&pattern_lower) {
            results.push(path_str);
        }
        if results.len() >= max_results {
            break;
        }
    }

    Ok(results)
}
