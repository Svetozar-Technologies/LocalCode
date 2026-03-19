use futures::StreamExt;
use std::io::{self, Write};
use crossterm::style::Stylize;

use localcode_core::llm::provider::{ChatChunk, ChatStream};
use localcode_core::CoreError;

pub async fn stream_to_stdout(mut stream: ChatStream) -> Result<String, CoreError> {
    let mut full_text = String::new();

    while let Some(chunk) = stream.next().await {
        match chunk? {
            ChatChunk::Text(text) => {
                print!("{}", text);
                io::stdout().flush().unwrap();
                full_text.push_str(&text);
            }
            ChatChunk::Done => break,
            ChatChunk::Error(e) => {
                eprintln!("\n{}", format!("Error: {}", e).red());
                return Err(CoreError::Llm(e));
            }
            ChatChunk::ToolCallStart { name, .. } => {
                print!("\n  {} {} ", "[".dark_grey(), name.cyan());
                io::stdout().flush().unwrap();
            }
            ChatChunk::ToolCallDelta { .. } => {
                // Don't print raw JSON args during streaming
            }
            ChatChunk::ToolCallEnd { .. } => {
                println!("{}", "]".dark_grey());
            }
        }
    }

    println!();
    Ok(full_text)
}
