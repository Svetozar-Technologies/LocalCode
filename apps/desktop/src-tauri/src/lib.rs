mod commands;

use commands::llm::create_llm_manager;
use commands::terminal::create_terminal_manager;

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    tauri::Builder::default()
        .plugin(tauri_plugin_dialog::init())
        .plugin(tauri_plugin_fs::init())
        .plugin(tauri_plugin_shell::init())
        .manage(create_terminal_manager())
        .manage(create_llm_manager())
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
            commands::llm::save_config,
            commands::llm::load_config,
            // Search
            commands::search::search_files,
            commands::search::search_content,
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
            // Agent
            commands::agent::agent_execute,
            // Model management
            commands::llm::list_model_catalog,
            commands::llm::download_model,
            commands::llm::list_downloaded_models,
            commands::llm::delete_model,
        ])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
