use serde::{Deserialize, Serialize};

use crate::CoreResult;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CodeChunk {
    pub file: String,
    pub start_line: usize,
    pub end_line: usize,
    pub content: String,
    pub kind: ChunkKind,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum ChunkKind {
    Function,
    Class,
    Module,
    Block,
}

/// Split file into meaningful code chunks (function-level)
pub fn chunk_file(file_path: &str, max_chunk_lines: usize) -> CoreResult<Vec<CodeChunk>> {
    let content = std::fs::read_to_string(file_path)?;
    let lines: Vec<&str> = content.lines().collect();

    if lines.is_empty() {
        return Ok(Vec::new());
    }

    let mut chunks = Vec::new();
    let mut current_start = 0;
    let mut brace_depth = 0;
    let mut in_function = false;

    for (i, line) in lines.iter().enumerate() {
        let trimmed = line.trim();

        // Detect function/class starts
        let is_def = trimmed.starts_with("fn ")
            || trimmed.starts_with("pub fn ")
            || trimmed.starts_with("async fn ")
            || trimmed.starts_with("pub async fn ")
            || trimmed.starts_with("def ")
            || trimmed.starts_with("async def ")
            || trimmed.starts_with("class ")
            || trimmed.starts_with("function ")
            || trimmed.starts_with("export function ")
            || trimmed.starts_with("func ");

        if is_def && !in_function {
            // Save previous block if non-empty
            if i > current_start {
                let block: String = lines[current_start..i].join("\n");
                if !block.trim().is_empty() {
                    chunks.push(CodeChunk {
                        file: file_path.to_string(),
                        start_line: current_start + 1,
                        end_line: i,
                        content: block,
                        kind: ChunkKind::Block,
                    });
                }
            }
            current_start = i;
            in_function = true;
            brace_depth = 0;
        }

        // Track braces
        for ch in trimmed.chars() {
            if ch == '{' {
                brace_depth += 1;
            } else if ch == '}' {
                brace_depth -= 1;
            }
        }

        // Python: detect dedent (simplified)
        let is_python = file_path.ends_with(".py");

        // End of function (brace-based languages)
        if in_function && !is_python && brace_depth <= 0 && i > current_start {
            let block: String = lines[current_start..=i].join("\n");
            chunks.push(CodeChunk {
                file: file_path.to_string(),
                start_line: current_start + 1,
                end_line: i + 1,
                content: block,
                kind: ChunkKind::Function,
            });
            current_start = i + 1;
            in_function = false;
        }

        // Max chunk size fallback
        if i - current_start >= max_chunk_lines {
            let block: String = lines[current_start..=i].join("\n");
            chunks.push(CodeChunk {
                file: file_path.to_string(),
                start_line: current_start + 1,
                end_line: i + 1,
                content: block,
                kind: if in_function {
                    ChunkKind::Function
                } else {
                    ChunkKind::Block
                },
            });
            current_start = i + 1;
            in_function = false;
            brace_depth = 0;
        }
    }

    // Remaining
    if current_start < lines.len() {
        let block: String = lines[current_start..].join("\n");
        if !block.trim().is_empty() {
            chunks.push(CodeChunk {
                file: file_path.to_string(),
                start_line: current_start + 1,
                end_line: lines.len(),
                content: block,
                kind: ChunkKind::Block,
            });
        }
    }

    Ok(chunks)
}
