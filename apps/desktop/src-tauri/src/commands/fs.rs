use std::path::PathBuf;
use std::sync::{Arc, Mutex};

use localcode_core::fs;
use localcode_core::fs::watcher::{ChangeKind, FileWatcher};
use tauri::{AppHandle, Emitter};

pub type WatcherManager = Arc<Mutex<Option<FileWatcher>>>;

pub fn create_watcher_manager() -> WatcherManager {
    Arc::new(Mutex::new(None))
}

#[tauri::command]
pub fn read_dir(path: String) -> Result<Vec<fs::FileEntry>, String> {
    fs::read_dir(&path).map_err(|e| e.to_string())
}

#[tauri::command]
pub fn read_file(path: String) -> Result<String, String> {
    fs::read_file(&path).map_err(|e| e.to_string())
}

#[tauri::command]
pub fn write_file(path: String, content: String) -> Result<(), String> {
    fs::write_file(&path, &content).map_err(|e| e.to_string())
}

#[tauri::command]
pub fn create_file(path: String) -> Result<(), String> {
    fs::create_file(&path).map_err(|e| e.to_string())
}

#[tauri::command]
pub fn create_dir(path: String) -> Result<(), String> {
    fs::create_dir(&path).map_err(|e| e.to_string())
}

#[tauri::command]
pub fn delete_entry(path: String) -> Result<(), String> {
    fs::delete_entry(&path).map_err(|e| e.to_string())
}

#[tauri::command]
pub fn rename_entry(old_path: String, new_path: String) -> Result<(), String> {
    fs::rename_entry(&old_path, &new_path).map_err(|e| e.to_string())
}

#[tauri::command]
pub fn watch_directory(
    path: String,
    app: AppHandle,
    state: tauri::State<'_, WatcherManager>,
) -> Result<(), String> {
    let mut manager = state.lock().map_err(|e| e.to_string())?;

    let app_handle = app.clone();
    let watcher = FileWatcher::new(move |file_path: PathBuf, kind: ChangeKind| {
        let kind_str = match kind {
            ChangeKind::Create => "create",
            ChangeKind::Modify => "modify",
            ChangeKind::Remove => "remove",
            ChangeKind::Rename => "rename",
            ChangeKind::Other => "other",
        };
        let _ = app_handle.emit(
            "file-changed",
            serde_json::json!({
                "path": file_path.to_string_lossy(),
                "kind": kind_str,
            }),
        );
    })
    .map_err(|e| e.to_string())?;

    *manager = Some(watcher);
    let watcher = manager.as_mut().unwrap();
    watcher
        .watch(std::path::Path::new(&path))
        .map_err(|e| e.to_string())?;

    Ok(())
}

#[tauri::command]
pub fn unwatch_directory(
    path: String,
    state: tauri::State<'_, WatcherManager>,
) -> Result<(), String> {
    let mut manager = state.lock().map_err(|e| e.to_string())?;
    if let Some(ref mut watcher) = *manager {
        watcher
            .unwatch(std::path::Path::new(&path))
            .map_err(|e| e.to_string())?;
    }
    Ok(())
}
