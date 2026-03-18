use serde::{Deserialize, Serialize};
use std::path::Path;

use crate::CoreResult;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CodeSymbol {
    pub name: String,
    pub kind: SymbolKind,
    pub file: String,
    pub start_line: usize,
    pub end_line: usize,
    pub signature: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum SymbolKind {
    Function,
    Class,
    Method,
    Struct,
    Enum,
    Interface,
    Module,
    Variable,
    Constant,
    Type,
}

/// Simple regex-based symbol extraction (tree-sitter can be added later)
pub fn extract_symbols(file_path: &str) -> CoreResult<Vec<CodeSymbol>> {
    let content = std::fs::read_to_string(file_path)?;
    let ext = Path::new(file_path)
        .extension()
        .and_then(|e| e.to_str())
        .unwrap_or("");

    let mut symbols = Vec::new();

    for (i, line) in content.lines().enumerate() {
        let trimmed = line.trim();

        match ext {
            "rs" => {
                if let Some(sym) = parse_rust_line(trimmed, file_path, i + 1) {
                    symbols.push(sym);
                }
            }
            "py" => {
                if let Some(sym) = parse_python_line(trimmed, file_path, i + 1) {
                    symbols.push(sym);
                }
            }
            "ts" | "tsx" | "js" | "jsx" => {
                if let Some(sym) = parse_js_line(trimmed, file_path, i + 1) {
                    symbols.push(sym);
                }
            }
            "go" => {
                if let Some(sym) = parse_go_line(trimmed, file_path, i + 1) {
                    symbols.push(sym);
                }
            }
            _ => {}
        }
    }

    Ok(symbols)
}

fn parse_rust_line(line: &str, file: &str, line_num: usize) -> Option<CodeSymbol> {
    if line.starts_with("pub fn ") || line.starts_with("fn ") || line.starts_with("pub async fn ") || line.starts_with("async fn ") {
        let name = extract_name_after(line, "fn ")?;
        Some(CodeSymbol {
            name,
            kind: SymbolKind::Function,
            file: file.to_string(),
            start_line: line_num,
            end_line: line_num,
            signature: line.to_string(),
        })
    } else if line.starts_with("pub struct ") || line.starts_with("struct ") {
        let name = extract_name_after(line, "struct ")?;
        Some(CodeSymbol {
            name,
            kind: SymbolKind::Struct,
            file: file.to_string(),
            start_line: line_num,
            end_line: line_num,
            signature: line.to_string(),
        })
    } else if line.starts_with("pub enum ") || line.starts_with("enum ") {
        let name = extract_name_after(line, "enum ")?;
        Some(CodeSymbol {
            name,
            kind: SymbolKind::Enum,
            file: file.to_string(),
            start_line: line_num,
            end_line: line_num,
            signature: line.to_string(),
        })
    } else if line.starts_with("pub trait ") || line.starts_with("trait ") {
        let name = extract_name_after(line, "trait ")?;
        Some(CodeSymbol {
            name,
            kind: SymbolKind::Interface,
            file: file.to_string(),
            start_line: line_num,
            end_line: line_num,
            signature: line.to_string(),
        })
    } else {
        None
    }
}

fn parse_python_line(line: &str, file: &str, line_num: usize) -> Option<CodeSymbol> {
    if line.starts_with("def ") || line.starts_with("async def ") {
        let name = extract_name_after(line, "def ")?;
        Some(CodeSymbol {
            name,
            kind: SymbolKind::Function,
            file: file.to_string(),
            start_line: line_num,
            end_line: line_num,
            signature: line.to_string(),
        })
    } else if line.starts_with("class ") {
        let name = extract_name_after(line, "class ")?;
        Some(CodeSymbol {
            name,
            kind: SymbolKind::Class,
            file: file.to_string(),
            start_line: line_num,
            end_line: line_num,
            signature: line.to_string(),
        })
    } else {
        None
    }
}

fn parse_js_line(line: &str, file: &str, line_num: usize) -> Option<CodeSymbol> {
    if line.starts_with("function ") || line.starts_with("export function ")
        || line.starts_with("export default function ")
        || line.starts_with("async function ")
        || line.starts_with("export async function ")
    {
        let name = extract_name_after(line, "function ")?;
        Some(CodeSymbol {
            name,
            kind: SymbolKind::Function,
            file: file.to_string(),
            start_line: line_num,
            end_line: line_num,
            signature: line.to_string(),
        })
    } else if line.starts_with("class ") || line.starts_with("export class ") {
        let name = extract_name_after(line, "class ")?;
        Some(CodeSymbol {
            name,
            kind: SymbolKind::Class,
            file: file.to_string(),
            start_line: line_num,
            end_line: line_num,
            signature: line.to_string(),
        })
    } else if line.starts_with("export interface ") || line.starts_with("interface ") {
        let name = extract_name_after(line, "interface ")?;
        Some(CodeSymbol {
            name,
            kind: SymbolKind::Interface,
            file: file.to_string(),
            start_line: line_num,
            end_line: line_num,
            signature: line.to_string(),
        })
    } else {
        None
    }
}

fn parse_go_line(line: &str, file: &str, line_num: usize) -> Option<CodeSymbol> {
    if line.starts_with("func ") {
        let name = extract_name_after(line, "func ")?;
        Some(CodeSymbol {
            name,
            kind: SymbolKind::Function,
            file: file.to_string(),
            start_line: line_num,
            end_line: line_num,
            signature: line.to_string(),
        })
    } else if line.starts_with("type ") && line.contains("struct") {
        let name = extract_name_after(line, "type ")?;
        Some(CodeSymbol {
            name,
            kind: SymbolKind::Struct,
            file: file.to_string(),
            start_line: line_num,
            end_line: line_num,
            signature: line.to_string(),
        })
    } else {
        None
    }
}

fn extract_name_after(line: &str, keyword: &str) -> Option<String> {
    let after = line.split(keyword).nth(1)?;
    let name: String = after
        .chars()
        .take_while(|c| c.is_alphanumeric() || *c == '_')
        .collect();
    if name.is_empty() {
        None
    } else {
        Some(name)
    }
}
