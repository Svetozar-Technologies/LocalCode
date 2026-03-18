use crossterm::style::{self, Color, Stylize};
use std::io::{self, Write};

pub fn print_header() {
    let version = env!("CARGO_PKG_VERSION");
    println!();
    println!("  {}", "╭─────────────────────────────────╮".dark_grey());
    println!("  {}  LocalCode v{}               {}",
        "│".dark_grey(), version, "│".dark_grey());
    print_project_info();
    println!("  {}", "╰─────────────────────────────────╯".dark_grey());
    println!();
}

fn print_project_info() {
    let cwd = std::env::current_dir()
        .map(|p| p.display().to_string())
        .unwrap_or_else(|_| "unknown".to_string());

    let project_name = std::path::Path::new(&cwd)
        .file_name()
        .and_then(|n| n.to_str())
        .unwrap_or("unknown");

    println!("  {}  Project: {:<22} {}",
        "│".dark_grey(), project_name, "│".dark_grey());
}

pub fn print_prompt() {
    print!("{} ", ">".green().bold());
    io::stdout().flush().unwrap();
}

pub fn print_assistant_prefix() {
    println!();
}

pub fn print_tool_call(name: &str, args: &serde_json::Value) {
    let args_str = if let Some(obj) = args.as_object() {
        obj.iter()
            .map(|(k, v)| {
                let val = match v {
                    serde_json::Value::String(s) => {
                        if s.len() > 60 {
                            format!("{}...", &s[..57])
                        } else {
                            s.clone()
                        }
                    }
                    _ => v.to_string(),
                };
                format!("{}={}", k, val)
            })
            .collect::<Vec<_>>()
            .join(", ")
    } else {
        String::new()
    };

    println!("  {} {} {}", "[".dark_grey(), name.cyan(), "]".dark_grey());
}

pub fn print_tool_result(name: &str, result: &str) {
    let truncated = if result.len() > 200 {
        format!("{}...", &result[..197])
    } else {
        result.to_string()
    };
    println!("  {} {}", "✓".green(), truncated.dark_grey());
}

pub fn print_error(msg: &str) {
    eprintln!("  {} {}", "✗".red(), msg);
}

pub fn print_success(msg: &str) {
    println!("  {} {}", "✓".green(), msg);
}

pub fn print_info(msg: &str) {
    println!("  {} {}", "ℹ".blue(), msg);
}

pub fn print_markdown(text: &str) {
    // Simple markdown rendering — print as-is with basic formatting
    for line in text.lines() {
        if line.starts_with("```") {
            println!("{}", line.dark_grey());
        } else if line.starts_with("# ") {
            println!("{}", line[2..].bold());
        } else if line.starts_with("## ") {
            println!("{}", line[3..].bold());
        } else if line.starts_with("- ") || line.starts_with("* ") {
            println!("  {} {}", "•".dark_grey(), &line[2..]);
        } else {
            println!("{}", line);
        }
    }
}
