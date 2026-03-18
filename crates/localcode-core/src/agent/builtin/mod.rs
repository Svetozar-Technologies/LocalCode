pub mod file_tools;
pub mod git_tools;
pub mod search_tools;
pub mod command_tools;
pub mod memory_tools;

use std::sync::Arc;
use crate::agent::tools::ToolRegistry;

/// Register all built-in tools with the registry
pub fn register_all(registry: &mut ToolRegistry) {
    // File tools
    registry.register(Arc::new(file_tools::ReadFileTool));
    registry.register(Arc::new(file_tools::WriteFileTool));
    registry.register(Arc::new(file_tools::EditFileTool));
    registry.register(Arc::new(file_tools::ListDirTool));
    registry.register(Arc::new(file_tools::CreateFileTool));
    registry.register(Arc::new(file_tools::DeleteFileTool));
    registry.register(Arc::new(file_tools::OpenInEditorTool));

    // Search tools
    registry.register(Arc::new(search_tools::SearchFilesTool));
    registry.register(Arc::new(search_tools::SearchContentTool));
    registry.register(Arc::new(search_tools::GlobFilesTool));

    // Git tools
    registry.register(Arc::new(git_tools::GitStatusTool));
    registry.register(Arc::new(git_tools::GitDiffTool));
    registry.register(Arc::new(git_tools::GitCommitTool));
    registry.register(Arc::new(git_tools::GitLogTool));

    // Command tools
    registry.register(Arc::new(command_tools::RunCommandTool));

    // Memory tools
    registry.register(Arc::new(memory_tools::CodebaseSearchTool));
    registry.register(Arc::new(memory_tools::UpdateMemoryTool));
}
