use crossterm::style::{Color, Stylize};
use std::io::{self, Write};

// ── Brand Colors ──────────────────────────────────────────────
const BRAND: Color = Color::Rgb { r: 99, g: 102, b: 241 };    // Indigo
const ACCENT: Color = Color::Rgb { r: 34, g: 211, b: 238 };   // Cyan
const SUCCESS: Color = Color::Rgb { r: 52, g: 211, b: 153 };  // Emerald
#[allow(dead_code)]
const WARN: Color = Color::Rgb { r: 251, g: 191, b: 36 };     // Amber
const ERR: Color = Color::Rgb { r: 248, g: 113, b: 113 };     // Red
const DIM: Color = Color::Rgb { r: 100, g: 116, b: 139 };     // Slate
const TEXT: Color = Color::Rgb { r: 226, g: 232, b: 240 };    // Light slate

// ── Header ────────────────────────────────────────────────────
pub fn print_header() {
    let version = env!("CARGO_PKG_VERSION");
    let cwd = std::env::current_dir()
        .map(|p| p.display().to_string())
        .unwrap_or_else(|_| "unknown".to_string());
    let _project_name = std::path::Path::new(&cwd)
        .file_name()
        .and_then(|n| n.to_str())
        .unwrap_or("unknown");

    println!();
    println!(
        "  {} {} {}",
        "◆".with(BRAND),
        "LocalCode".bold().with(TEXT),
        format!("v{}", version).with(DIM)
    );
    println!(
        "  {}  {}",
        "│".with(DIM),
        format!("{}", cwd).with(DIM)
    );
    println!(
        "  {}  Type {} for commands, {} to quit",
        "│".with(DIM),
        "/help".with(ACCENT),
        "/exit".with(ACCENT)
    );
    println!();
}

// ── Prompts ───────────────────────────────────────────────────
pub fn print_prompt() {
    print!("{} ", "›".with(TEXT));
    io::stdout().flush().unwrap();
}

pub fn print_agent_prompt() {
    print!("{} ", "❯".bold().with(BRAND));
    io::stdout().flush().unwrap();
}

#[allow(dead_code)]
pub fn print_assistant_prefix() {
    println!();
}

// ── Tool Calls ────────────────────────────────────────────────
pub fn print_tool_call(name: &str, args: &serde_json::Value) {
    // Extract the most relevant arg
    let detail = if let Some(obj) = args.as_object() {
        obj.get("path").or(obj.get("command")).or(obj.get("query"))
            .or(obj.get("pattern")).or(obj.get("message")).or(obj.get("fact"))
            .and_then(|v| v.as_str())
            .map(|s| truncate_str(s, 55))
            .unwrap_or_default()
    } else {
        String::new()
    };

    // Human-readable action log
    let (icon, action) = match name {
        "read_file" =>     ("◇", format!("Reading {}", if detail.is_empty() { "file".into() } else { detail.clone() })),
        "write_file" =>    ("◈", format!("Writing {}", if detail.is_empty() { "file".into() } else { detail.clone() })),
        "create_file" =>   ("◈", format!("Creating {}", if detail.is_empty() { "file".into() } else { detail.clone() })),
        "edit_file" =>     ("✎", format!("Editing {}", if detail.is_empty() { "file".into() } else { detail.clone() })),
        "delete_file" =>   ("✕", format!("Deleting {}", if detail.is_empty() { "file".into() } else { detail.clone() })),
        "list_dir" =>      ("◎", format!("Listing {}", if detail.is_empty() { "directory".into() } else { detail.clone() })),
        "search_files" =>  ("◎", format!("Finding files: {}", detail)),
        "search_content" =>("⊙", format!("Searching: {}", detail)),
        "codebase_search"=>("⊙", format!("Searching codebase: {}", detail)),
        "grep_search" =>   ("⊙", format!("Grep: {}", detail)),
        "find_files" =>    ("◎", format!("Finding: {}", detail)),
        "glob" =>          ("◎", format!("Glob: {}", detail)),
        "run_command" =>   ("▶", format!("Running: {}", if detail.is_empty() { "command".into() } else { detail.clone() })),
        "git_status" =>    ("⎇", "Checking git status".into()),
        "git_diff" =>      ("⎇", "Getting diff".into()),
        "git_commit" =>    ("⎇", format!("Committing: {}", detail)),
        "git_log" =>       ("⎇", "Viewing commit history".into()),
        "http_request" =>  ("⇄", format!("Fetching: {}", detail)),
        "sed_replace" =>   ("✎", format!("Replacing in {}", detail)),
        "count_lines" =>   ("▪", format!("Counting lines: {}", detail)),
        "update_memory" => ("◉", format!("Saving: {}", truncate_str(&detail, 40))),
        "dispatch_subagent"=>("⊕", format!("Dispatching: {}", detail)),
        _ =>               ("●", format!("{} {}", name, detail)),
    };

    println!(
        "    {} {}",
        icon.with(ACCENT),
        action.with(DIM),
    );
}

pub fn print_tool_result(_name: &str, result: &str) {
    let is_error = result.starts_with("[ERROR") || result.starts_with("Error:");
    let max_len = if is_error { 500 } else { 300 };
    let truncated = truncate_str(result, max_len);

    if is_error {
        println!(
            "      {} {}",
            "✗".with(ERR),
            truncated.with(ERR),
        );
    } else {
        println!(
            "      {} {}",
            "↳".with(SUCCESS),
            truncated.with(DIM),
        );
    }
}

// ── Status Messages ───────────────────────────────────────────
pub fn print_error(msg: &str) {
    eprintln!(
        "  {} {}",
        "✗".with(ERR),
        msg.with(ERR),
    );
}

pub fn print_success(msg: &str) {
    println!(
        "  {} {}",
        "✓".with(SUCCESS),
        msg,
    );
}

pub fn print_info(msg: &str) {
    println!(
        "  {} {}",
        "→".with(ACCENT),
        msg.with(TEXT),
    );
}

#[allow(dead_code)]
pub fn print_warning(msg: &str) {
    println!(
        "  {} {}",
        "!".with(WARN),
        msg,
    );
}

// ── Section Headers ───────────────────────────────────────────
#[allow(dead_code)]
pub fn print_thinking() {
    println!(
        "  {} {}",
        "◆".with(BRAND),
        "Thinking...".with(DIM),
    );
}

#[allow(dead_code)]
pub fn print_done() {
    println!();
}

// ── Markdown Rendering ───────────────────────────────────────
pub fn print_markdown(text: &str) {
    let mut in_code_block = false;
    for line in text.lines() {
        if line.starts_with("```") {
            in_code_block = !in_code_block;
            if in_code_block {
                println!("  {}", "─".repeat(50).with(DIM));
            } else {
                println!("  {}", "─".repeat(50).with(DIM));
            }
            continue;
        }

        if in_code_block {
            println!("  {}", line.with(ACCENT));
            continue;
        }

        if line.starts_with("### ") {
            println!("  {} {}", "▸".with(BRAND), line[4..].bold().with(TEXT));
        } else if line.starts_with("## ") {
            println!();
            println!("  {} {}", "▪".with(BRAND), line[3..].bold().with(TEXT));
        } else if line.starts_with("# ") {
            println!();
            println!("  {} {}", "◆".with(BRAND), line[2..].bold().with(TEXT));
        } else if line.starts_with("- ") || line.starts_with("* ") {
            println!("  {} {}", "·".with(DIM), &line[2..]);
        } else if line.starts_with("  - ") || line.starts_with("  * ") {
            println!("    {} {}", "·".with(DIM), &line[4..]);
        } else if let Some(rest) = line.strip_prefix("> ") {
            println!("  {} {}", "│".with(BRAND), rest.with(DIM));
        } else if line.starts_with("**") && line.ends_with("**") {
            let inner = &line[2..line.len()-2];
            println!("  {}", inner.bold().with(TEXT));
        } else if line.trim().is_empty() {
            println!();
        } else {
            println!("  {}", line);
        }
    }
}

// ── Help Screen ──────────────────────────────────────────────
pub fn print_help() {
    println!();
    println!(
        "  {} {}",
        "◆".with(BRAND),
        "LocalCode Commands".bold().with(TEXT),
    );
    println!();

    let commands = [
        ("/help, /h", "Show this help"),
        ("/clear, /c", "Clear conversation"),
        ("/agent, /a", "Toggle agent mode"),
        ("/model [name]", "Show or switch model"),
        ("/memory", "Show project memory"),
        ("/sessions", "List recent sessions"),
        ("/sessions search ..", "Search sessions by keyword"),
        ("/session info <id>", "Show session details"),
        ("/session delete <id>", "Delete a session"),
        ("/recall <query>", "Load matching session into context"),
        ("/commit", "AI-generated commit"),
        ("/config", "Show configuration"),
        ("/exit, /q", "Exit"),
    ];

    for (cmd, desc) in &commands {
        println!(
            "    {}  {}",
            format!("{:<16}", cmd).with(ACCENT),
            desc.with(DIM),
        );
    }
    println!();
}

// ── Agent Mode Banner ─────────────────────────────────────────
pub fn print_agent_mode_status(enabled: bool) {
    if enabled {
        println!(
            "  {} {}",
            "◆".with(BRAND),
            "Agent mode enabled — tools are active".with(TEXT),
        );
    } else {
        println!(
            "  {} {}",
            "○".with(DIM),
            "Chat mode — no tools".with(DIM),
        );
    }
}

// ── Session Display ──────────────────────────────────────────
pub fn print_sessions_list(sessions: &[(String, u64, String, String)]) {
    // Each tuple: (id, timestamp, project_name, title)
    if sessions.is_empty() {
        println!(
            "  {} {}",
            "→".with(ACCENT),
            "No sessions found".with(DIM),
        );
        return;
    }

    println!();
    println!(
        "  {} {}",
        "◆".with(BRAND),
        "Sessions".bold().with(TEXT),
    );
    println!();

    for (id, timestamp, project, title) in sessions {
        let date = format_timestamp(*timestamp);
        let short_id = if id.len() > 8 { &id[..8] } else { id.as_str() };
        println!(
            "    {} {} {} {}",
            short_id.with(DIM),
            date.with(ACCENT),
            project.as_str().with(TEXT),
            title.as_str().with(DIM),
        );
    }
    println!();
}

pub fn print_session_detail(
    id: &str,
    project: &str,
    started: u64,
    ended: Option<u64>,
    files: &[String],
    tasks: &[String],
    summary: &str,
    tags: &[String],
) {
    println!();
    println!(
        "  {} {}",
        "◆".with(BRAND),
        "Session Detail".bold().with(TEXT),
    );
    println!();
    println!("    {} {}", "ID:".with(DIM), id.with(TEXT));
    println!("    {} {}", "Project:".with(DIM), project.with(TEXT));
    println!("    {} {}", "Started:".with(DIM), format_timestamp(started).with(TEXT));
    if let Some(end) = ended {
        println!("    {} {}", "Ended:".with(DIM), format_timestamp(end).with(TEXT));
    }
    if !tags.is_empty() {
        println!("    {} {}", "Tags:".with(DIM), tags.join(", ").with(ACCENT));
    }
    if !files.is_empty() {
        println!("    {} {}", "Files:".with(DIM), files.join(", ").with(TEXT));
    }
    if !tasks.is_empty() {
        println!("    {}", "Tasks:".with(DIM));
        for t in tasks {
            println!("      {} {}", "·".with(DIM), t.as_str().with(TEXT));
        }
    }
    if !summary.is_empty() {
        println!("    {}", "Summary:".with(DIM));
        println!("      {}", summary.with(TEXT));
    }
    println!();
}

pub fn print_search_results(results: &[(f32, String, String, String)]) {
    // Each tuple: (score, id, project_name, title)
    if results.is_empty() {
        println!(
            "  {} {}",
            "→".with(ACCENT),
            "No matching sessions found".with(DIM),
        );
        return;
    }

    println!();
    println!(
        "  {} {}",
        "◆".with(BRAND),
        "Search Results".bold().with(TEXT),
    );
    println!();

    for (score, id, project, title) in results {
        let short_id = if id.len() > 8 { &id[..8] } else { id.as_str() };
        let pct = (score * 100.0) as u32;
        println!(
            "    {}% {} {} {}",
            format!("{:>3}", pct).with(SUCCESS),
            short_id.with(DIM),
            project.as_str().with(TEXT),
            title.as_str().with(DIM),
        );
    }
    println!();
}

fn format_timestamp(ts: u64) -> String {
    // Simple date formatting without chrono
    let secs_per_day: u64 = 86400;
    let days_since_epoch = ts / secs_per_day;
    let secs_in_day = ts % secs_per_day;
    let hours = secs_in_day / 3600;
    let minutes = (secs_in_day % 3600) / 60;

    // Approximate date calculation
    let mut y: i64 = 1970;
    let mut remaining_days = days_since_epoch as i64;

    loop {
        let days_in_year: i64 = if y % 4 == 0 && (y % 100 != 0 || y % 400 == 0) { 366 } else { 365 };
        if remaining_days < days_in_year {
            break;
        }
        remaining_days -= days_in_year;
        y += 1;
    }

    let months_days: Vec<i64> = if y % 4 == 0 && (y % 100 != 0 || y % 400 == 0) {
        vec![31,29,31,30,31,30,31,31,30,31,30,31]
    } else {
        vec![31,28,31,30,31,30,31,31,30,31,30,31]
    };

    let mut m = 0;
    for (i, &days) in months_days.iter().enumerate() {
        if remaining_days < days {
            m = i + 1;
            break;
        }
        remaining_days -= days;
    }
    if m == 0 { m = 12; }
    let d = remaining_days + 1;

    format!("{:04}-{:02}-{:02} {:02}:{:02}", y, m, d, hours, minutes)
}

// ── Utilities ─────────────────────────────────────────────────
fn truncate_str(s: &str, max: usize) -> String {
    // Handle multi-line: take first line only
    let first_line = s.lines().next().unwrap_or(s);
    if first_line.len() > max {
        format!("{}…", &first_line[..max - 1])
    } else {
        first_line.to_string()
    }
}
