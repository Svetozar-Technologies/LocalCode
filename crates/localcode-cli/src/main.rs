mod repl;
mod commands;
mod rendering;
mod streaming;
mod conversation;
mod slash;
#[allow(dead_code)]
mod hooks;

use clap::{Parser, Subcommand};

#[derive(Parser)]
#[command(name = "localcode", version, about = "LocalCode - AI-powered coding assistant")]
struct Cli {
    #[command(subcommand)]
    command: Option<Commands>,
}

#[derive(Subcommand)]
enum Commands {
    /// Send a single chat message (for interactive REPL, run `localcode` with no arguments)
    Chat {
        /// Question or prompt
        message: String,
        /// Provider to use (local, openai, anthropic)
        #[arg(short, long)]
        provider: Option<String>,
        /// Model to use
        #[arg(short, long)]
        model: Option<String>,
    },
    /// Run agent to execute a task autonomously
    Agent {
        /// Task description
        task: String,
        /// Provider to use
        #[arg(short, long)]
        provider: Option<String>,
    },
    /// Initialize LOCALCODE.md in current directory
    Init,
    /// Manage configuration
    Config {
        /// Set a config key
        #[arg(short, long)]
        set: Option<String>,
        /// Get a config key
        #[arg(short, long)]
        get: Option<String>,
        /// Show all config
        #[arg(long)]
        show: bool,
    },
    /// Review current git changes with AI
    Review {
        /// Provider to use
        #[arg(short, long)]
        provider: Option<String>,
    },
    /// Fix an error with AI assistance
    Fix {
        /// Error message or description
        error: String,
        /// Provider to use
        #[arg(short, long)]
        provider: Option<String>,
    },
}

#[tokio::main]
async fn main() {
    let cli = Cli::parse();

    match cli.command {
        None => {
            // Default: launch interactive REPL
            if let Err(e) = repl::run_repl().await {
                eprintln!("Error: {}", e);
                std::process::exit(1);
            }
        }
        Some(Commands::Chat { message, provider, model }) => {
            if let Err(e) = commands::chat::run_chat(&message, provider.as_deref(), model.as_deref()).await {
                eprintln!("Error: {}", e);
                std::process::exit(1);
            }
        }
        Some(Commands::Agent { task, provider }) => {
            if let Err(e) = commands::agent::run_agent(&task, provider.as_deref()).await {
                eprintln!("Error: {}", e);
                std::process::exit(1);
            }
        }
        Some(Commands::Init) => {
            commands::init::run_init();
        }
        Some(Commands::Config { set, get, show }) => {
            commands::config::run_config(set.as_deref(), get.as_deref(), show);
        }
        Some(Commands::Review { provider }) => {
            if let Err(e) = commands::review::run_review(provider.as_deref()).await {
                eprintln!("Error: {}", e);
                std::process::exit(1);
            }
        }
        Some(Commands::Fix { error, provider }) => {
            if let Err(e) = commands::fix::run_fix(&error, provider.as_deref()).await {
                eprintln!("Error: {}", e);
                std::process::exit(1);
            }
        }
    }
}
