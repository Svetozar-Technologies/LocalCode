pub enum SlashCommand {
    Help,
    Clear,
    Model(Option<String>),
    Memory,
    Commit,
    Config,
    Exit,
    Sessions(Option<String>),       // /sessions or /sessions search <query>
    SessionInfo(String),            // /session info <id>
    SessionDelete(String),          // /session delete <id>
    Recall(String),                 // /recall <query>
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
        "/sessions" => SlashCommand::Sessions(arg),
        "/session" => {
            // Parse subcommands: /session info <id>, /session delete <id>
            if let Some(ref rest) = arg {
                let sub_parts: Vec<&str> = rest.splitn(2, ' ').collect();
                match sub_parts[0] {
                    "info" => {
                        if let Some(id) = sub_parts.get(1) {
                            SlashCommand::SessionInfo(id.trim().to_string())
                        } else {
                            SlashCommand::Unknown("/session info requires an ID".to_string())
                        }
                    }
                    "delete" => {
                        if let Some(id) = sub_parts.get(1) {
                            SlashCommand::SessionDelete(id.trim().to_string())
                        } else {
                            SlashCommand::Unknown("/session delete requires an ID".to_string())
                        }
                    }
                    _ => SlashCommand::Unknown(format!("/session {}", rest)),
                }
            } else {
                SlashCommand::Sessions(None) // /session alone = list
            }
        }
        "/recall" => {
            if let Some(query) = arg {
                SlashCommand::Recall(query)
            } else {
                SlashCommand::Unknown("/recall requires a search query".to_string())
            }
        }
        _ => SlashCommand::Unknown(cmd.to_string()),
    })
}

#[allow(dead_code)]
pub fn print_help() {
    crate::rendering::print_help();
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_sessions_list() {
        let cmd = parse_slash("/sessions").unwrap();
        assert!(matches!(cmd, SlashCommand::Sessions(None)));
    }

    #[test]
    fn test_parse_sessions_search() {
        let cmd = parse_slash("/sessions search snake game").unwrap();
        match cmd {
            SlashCommand::Sessions(Some(q)) => assert_eq!(q, "search snake game"),
            _ => panic!("Expected Sessions with search query"),
        }
    }

    #[test]
    fn test_parse_session_info() {
        let cmd = parse_slash("/session info abc123").unwrap();
        match cmd {
            SlashCommand::SessionInfo(id) => assert_eq!(id, "abc123"),
            _ => panic!("Expected SessionInfo"),
        }
    }

    #[test]
    fn test_parse_session_info_no_id() {
        let cmd = parse_slash("/session info").unwrap();
        assert!(matches!(cmd, SlashCommand::Unknown(_)));
    }

    #[test]
    fn test_parse_session_delete() {
        let cmd = parse_slash("/session delete xyz789").unwrap();
        match cmd {
            SlashCommand::SessionDelete(id) => assert_eq!(id, "xyz789"),
            _ => panic!("Expected SessionDelete"),
        }
    }

    #[test]
    fn test_parse_session_delete_no_id() {
        let cmd = parse_slash("/session delete").unwrap();
        assert!(matches!(cmd, SlashCommand::Unknown(_)));
    }

    #[test]
    fn test_parse_session_alone_lists() {
        let cmd = parse_slash("/session").unwrap();
        assert!(matches!(cmd, SlashCommand::Sessions(None)));
    }

    #[test]
    fn test_parse_recall() {
        let cmd = parse_slash("/recall todo json").unwrap();
        match cmd {
            SlashCommand::Recall(q) => assert_eq!(q, "todo json"),
            _ => panic!("Expected Recall"),
        }
    }

    #[test]
    fn test_parse_recall_no_query() {
        let cmd = parse_slash("/recall").unwrap();
        assert!(matches!(cmd, SlashCommand::Unknown(_)));
    }

    #[test]
    fn test_parse_not_slash() {
        assert!(parse_slash("hello").is_none());
    }

    #[test]
    fn test_parse_existing_commands_unchanged() {
        assert!(matches!(parse_slash("/help").unwrap(), SlashCommand::Help));
        assert!(matches!(parse_slash("/h").unwrap(), SlashCommand::Help));
        assert!(matches!(parse_slash("/clear").unwrap(), SlashCommand::Clear));
        assert!(matches!(parse_slash("/exit").unwrap(), SlashCommand::Exit));
        assert!(matches!(parse_slash("/q").unwrap(), SlashCommand::Exit));
        assert!(matches!(parse_slash("/config").unwrap(), SlashCommand::Config));
        assert!(matches!(parse_slash("/memory").unwrap(), SlashCommand::Memory));
        assert!(matches!(parse_slash("/commit").unwrap(), SlashCommand::Commit));
    }

    #[test]
    fn test_parse_session_unknown_subcommand() {
        let cmd = parse_slash("/session foobar").unwrap();
        assert!(matches!(cmd, SlashCommand::Unknown(_)));
    }
}
