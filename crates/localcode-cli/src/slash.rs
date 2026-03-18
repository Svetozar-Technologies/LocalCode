pub enum SlashCommand {
    Help,
    Clear,
    Model(Option<String>),
    Memory,
    Commit,
    Config,
    Exit,
    Unknown(String),
}

pub fn parse_slash(input: &str) -> Option<SlashCommand> {
    let trimmed = input.trim();
    if !trimmed.starts_with('/') {
        return None;
    }

    let parts: Vec<&str> = trimmed.splitn(2, ' ').collect();
    let cmd = parts[0];
    let arg = parts.get(1).map(|s| s.to_string());

    Some(match cmd {
        "/help" | "/h" => SlashCommand::Help,
        "/clear" | "/c" => SlashCommand::Clear,
        "/model" | "/m" => SlashCommand::Model(arg),
        "/memory" => SlashCommand::Memory,
        "/commit" => SlashCommand::Commit,
        "/config" => SlashCommand::Config,
        "/exit" | "/quit" | "/q" => SlashCommand::Exit,
        _ => SlashCommand::Unknown(cmd.to_string()),
    })
}

pub fn print_help() {
    println!("  Slash Commands:");
    println!("    /help          Show this help");
    println!("    /clear         Clear conversation");
    println!("    /model [name]  Switch model/provider");
    println!("    /memory        Show memory for this project");
    println!("    /commit        AI-generated commit message");
    println!("    /config        Show configuration");
    println!("    /exit          Exit REPL");
    println!();
}
