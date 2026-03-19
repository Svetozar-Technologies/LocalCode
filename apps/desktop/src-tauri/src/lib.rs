mod commands;

use commands::chat::create_chat_manager;
use commands::debug::create_debug_manager;
use commands::fs::create_watcher_manager;
use commands::llm::{create_llm_manager, LLMManager};
use commands::lsp::create_lsp_manager;
use commands::terminal::create_terminal_manager;

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    let llm_manager = create_llm_manager();
    let llm_for_cleanup = llm_manager.clone();

    tauri::Builder::default()
        .plugin(tauri_plugin_dialog::init())
        .plugin(tauri_plugin_fs::init())
        .plugin(tauri_plugin_shell::init())
        .manage(create_terminal_manager())
        .manage(llm_manager)
        .manage(create_debug_manager())
        .manage(create_watcher_manager())
        .manage(create_lsp_manager())
        .manage(create_chat_manager())
        .on_window_event(move |_window, event| {
            if let tauri::WindowEvent::Destroyed = event {
                // Kill llama-server when app window closes
                cleanup_llm(&llm_for_cleanup);
            }
        })
        .setup(|app| {
            if cfg!(debug_assertions) {
                app.handle().plugin(
                    tauri_plugin_log::Builder::default()
                        .level(log::LevelFilter::Info)
                        .build(),
                )?;
            }
            Ok(())
        })
        .invoke_handler(tauri::generate_handler![
            // File system
            commands::fs::read_dir,
            commands::fs::read_file,
            commands::fs::write_file,
            commands::fs::create_file,
            commands::fs::create_dir,
            commands::fs::delete_entry,
            commands::fs::rename_entry,
            commands::fs::watch_directory,
            commands::fs::unwatch_directory,
            // Terminal
            commands::terminal::spawn_terminal,
            commands::terminal::write_terminal,
            commands::terminal::resize_terminal,
            commands::terminal::kill_terminal,
            // LLM
            commands::llm::start_llm_server,
            commands::llm::stop_llm_server,
            commands::llm::llm_chat,
            commands::llm::llm_complete,
            commands::llm::llm_complete_stream,
            commands::llm::save_config,
            commands::llm::load_config,
            // Search
            commands::search::search_files,
            commands::search::search_content,
            commands::search::search_codebase,
            // Git
            commands::git::git_status,
            commands::git::git_branch,
            commands::git::git_log,
            commands::git::git_diff,
            commands::git::git_add,
            commands::git::git_add_all,
            commands::git::git_commit,
            commands::git::git_push,
            commands::git::git_pull,
            commands::git::git_list_branches,
            commands::git::git_unstage,
            commands::git::git_init,
            commands::git::git_blame,
            commands::git::git_file_log,
            // Debug
            commands::debug::debug_start,
            commands::debug::debug_stop,
            commands::debug::debug_continue,
            commands::debug::debug_step_over,
            commands::debug::debug_step_into,
            commands::debug::debug_step_out,
            commands::debug::debug_pause,
            // Agent
            commands::agent::agent_execute,
            commands::agent::composer_generate,
            // Model management
            commands::llm::list_model_catalog,
            commands::llm::download_model,
            commands::llm::list_downloaded_models,
            commands::llm::delete_model,
            // LSP
            commands::lsp::lsp_start,
            commands::lsp::lsp_hover,
            commands::lsp::lsp_definition,
            commands::lsp::lsp_references,
            commands::lsp::lsp_completions,
            // Chat persistence
            commands::chat::chat_create_session,
            commands::chat::chat_list_sessions,
            commands::chat::chat_get_messages,
            commands::chat::chat_add_message,
            commands::chat::chat_update_message,
            commands::chat::chat_delete_session,
            commands::chat::chat_update_session_title,
            commands::chat::chat_search,
        ])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}

fn cleanup_llm(manager: &LLMManager) {
    if let Ok(llm) = manager.lock() {
        let _ = llm.local.stop_server();
    }
    // Also kill any orphan llama-server processes
    let _ = std::process::Command::new("pkill")
        .arg("-f")
        .arg("llama-server")
        .output();
}
