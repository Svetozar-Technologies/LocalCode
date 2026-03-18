pub mod fs;
pub mod git;
pub mod search;
pub mod terminal;
pub mod llm;
pub mod agent;
pub mod config;
pub mod indexing;
pub mod mcp;
pub mod plugin;
pub mod debug;
pub mod python;

use thiserror::Error;

#[derive(Error, Debug)]
pub enum CoreError {
    #[error("IO error: {0}")]
    Io(#[from] std::io::Error),

    #[error("Git error: {0}")]
    Git(#[from] git2::Error),

    #[error("JSON error: {0}")]
    Json(#[from] serde_json::Error),

    #[error("HTTP error: {0}")]
    Http(#[from] reqwest::Error),

    #[error("LLM error: {0}")]
    Llm(String),

    #[error("Agent error: {0}")]
    Agent(String),

    #[error("Config error: {0}")]
    Config(String),

    #[error("{0}")]
    Other(String),
}

pub type CoreResult<T> = Result<T, CoreError>;
