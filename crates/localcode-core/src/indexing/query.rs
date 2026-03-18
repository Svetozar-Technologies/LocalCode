use ignore::WalkBuilder;

use super::chunker;
use super::store::CodeIndex;
use crate::CoreResult;

const CODE_EXTENSIONS: &[&str] = &[
    "rs", "py", "ts", "tsx", "js", "jsx", "go", "java", "c", "cpp", "h", "hpp",
    "rb", "php", "swift", "kt", "scala", "cs", "vue", "svelte", "lua", "sh",
    "bash", "zsh", "sql", "toml", "yaml", "yml", "json", "md", "txt",
];

/// Build or update the index for a project
pub fn build_index(project_path: &str) -> CoreResult<CodeIndex> {
    let index_path = CodeIndex::index_path(project_path);
    let mut index = CodeIndex::load(&index_path).unwrap_or_default();

    let walker = WalkBuilder::new(project_path)
        .hidden(true)
        .git_ignore(true)
        .build();

    for entry in walker {
        let entry = entry.map_err(|e| crate::CoreError::Other(e.to_string()))?;
        let path = entry.path();

        if !path.is_file() {
            continue;
        }

        let ext = path
            .extension()
            .and_then(|e| e.to_str())
            .unwrap_or("");

        if !CODE_EXTENSIONS.contains(&ext) {
            continue;
        }

        let path_str = path.to_string_lossy().to_string();

        // Check if file changed
        let metadata = std::fs::metadata(path)?;
        let modified = metadata
            .modified()
            .ok()
            .and_then(|t| t.duration_since(std::time::UNIX_EPOCH).ok())
            .map(|d| d.as_secs())
            .unwrap_or(0);

        if let Some(&cached_hash) = index.file_hashes.get(&path_str) {
            if cached_hash == modified {
                continue; // File hasn't changed
            }
        }

        // Re-index file
        index.remove_file(&path_str);

        if let Ok(chunks) = chunker::chunk_file(&path_str, 50) {
            for chunk in &chunks {
                index.add_chunk(chunk);
            }
        }

        index.file_hashes.insert(path_str, modified);
    }

    index.save(&index_path)?;
    Ok(index)
}

/// Ensure the index is fresh (rebuild if older than max_age_secs)
pub fn ensure_index_fresh(project_path: &str, max_age_secs: u64) -> CoreResult<CodeIndex> {
    let index_path = CodeIndex::index_path(project_path);
    let needs_rebuild = if index_path.exists() {
        let metadata = std::fs::metadata(&index_path)?;
        let modified = metadata
            .modified()
            .ok()
            .and_then(|t| t.duration_since(std::time::UNIX_EPOCH).ok())
            .map(|d| d.as_secs())
            .unwrap_or(0);
        let now = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap_or_default()
            .as_secs();
        now - modified > max_age_secs
    } else {
        true
    };

    if needs_rebuild {
        build_index(project_path)
    } else {
        CodeIndex::load(&index_path)
    }
}

/// Query the index for relevant code chunks
pub fn query_codebase(
    project_path: &str,
    query: &str,
    top_k: usize,
) -> CoreResult<Vec<String>> {
    let index_path = CodeIndex::index_path(project_path);
    let index = CodeIndex::load(&index_path)?;

    if index.entries.is_empty() {
        return Err(crate::CoreError::Other(
            "No index found. Run indexing first.".to_string(),
        ));
    }

    let results = index.search(query, top_k);

    Ok(results
        .iter()
        .map(|entry| {
            format!(
                "// {}:{}-{}\n{}",
                entry.file, entry.start_line, entry.end_line, entry.content
            )
        })
        .collect())
}
