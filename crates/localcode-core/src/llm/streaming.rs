use futures::StreamExt;

use super::provider::*;
use crate::CoreError;

/// Collect a ChatStream into a complete text string
pub async fn collect_stream_text(
    mut stream: ChatStream,
) -> Result<String, CoreError> {
    let mut text = String::new();

    while let Some(chunk) = stream.next().await {
        match chunk? {
            ChatChunk::Text(t) => text.push_str(&t),
            ChatChunk::Done => break,
            ChatChunk::Error(e) => return Err(CoreError::Llm(e)),
            _ => {}
        }
    }

    Ok(text)
}

/// Collect a ChatStream into a ChatMessage with potential tool calls
pub async fn collect_stream_message(
    mut stream: ChatStream,
) -> Result<ChatMessage, CoreError> {
    let mut text = String::new();
    let mut tool_calls: Vec<ToolCall> = Vec::new();
    let mut current_tool: Option<(String, String, String)> = None; // (id, name, args)

    while let Some(chunk) = stream.next().await {
        match chunk? {
            ChatChunk::Text(t) => text.push_str(&t),
            ChatChunk::ToolCallStart { id, name } => {
                current_tool = Some((id, name, String::new()));
            }
            ChatChunk::ToolCallDelta { arguments_delta, .. } => {
                if let Some((_, _, ref mut args)) = current_tool {
                    args.push_str(&arguments_delta);
                }
            }
            ChatChunk::ToolCallEnd { .. } => {
                if let Some((id, name, args)) = current_tool.take() {
                    tool_calls.push(ToolCall {
                        id,
                        call_type: "function".to_string(),
                        function: ToolCallFunction {
                            name,
                            arguments: args,
                        },
                    });
                }
            }
            ChatChunk::Done => break,
            ChatChunk::Error(e) => return Err(CoreError::Llm(e)),
        }
    }

    Ok(ChatMessage {
        role: "assistant".to_string(),
        content: text,
        tool_calls: if tool_calls.is_empty() {
            None
        } else {
            Some(tool_calls)
        },
        tool_call_id: None,
    })
}
