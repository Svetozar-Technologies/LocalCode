use crate::rendering;

const TEMPLATE: &str = r#"# Project Instructions

## Project Info
- Description: [Your project description]
- Language: [Primary language]
- Framework: [Framework/library]

## Build & Test
- Build: [build command]
- Test: [test command]
- Lint: [lint command]

## Conventions
- [Add your coding conventions here]
- [Branch naming, commit style, etc.]

## Important Notes
- [Key architecture decisions]
- [Things the AI should know about this project]
"#;

pub fn run_init() {
    let path = std::env::current_dir()
        .unwrap_or_default()
        .join("LOCALCODE.md");

    if path.exists() {
        rendering::print_info("LOCALCODE.md already exists");
        return;
    }

    match std::fs::write(&path, TEMPLATE) {
        Ok(_) => rendering::print_success("Created LOCALCODE.md — customize it for your project"),
        Err(e) => rendering::print_error(&format!("Failed to create LOCALCODE.md: {}", e)),
    }
}
