use std::path::{Path, PathBuf};
use std::sync::{Arc, Mutex};

use notify::{Event, EventKind, RecommendedWatcher, RecursiveMode, Watcher};

use crate::CoreResult;

/// Describes a file change kind
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub enum ChangeKind {
    Create,
    Modify,
    Remove,
    Rename,
    Other,
}

impl From<EventKind> for ChangeKind {
    fn from(kind: EventKind) -> Self {
        match kind {
            EventKind::Create(_) => ChangeKind::Create,
            EventKind::Modify(_) => ChangeKind::Modify,
            EventKind::Remove(_) => ChangeKind::Remove,
            _ => ChangeKind::Other,
        }
    }
}

/// File watcher that monitors directories for changes
pub struct FileWatcher {
    watcher: RecommendedWatcher,
    watched_paths: Vec<PathBuf>,
}

impl FileWatcher {
    pub fn new<F>(callback: F) -> CoreResult<Self>
    where
        F: Fn(PathBuf, ChangeKind) + Send + 'static,
    {
        let callback = Arc::new(Mutex::new(callback));
        let watcher = notify::recommended_watcher(move |res: Result<Event, notify::Error>| {
            if let Ok(event) = res {
                let kind = ChangeKind::from(event.kind);
                let cb = callback.lock().unwrap();
                for path in event.paths {
                    cb(path, kind.clone());
                }
            }
        })
        .map_err(|e| crate::CoreError::Other(format!("Failed to create file watcher: {}", e)))?;

        Ok(Self {
            watcher,
            watched_paths: Vec::new(),
        })
    }

    pub fn watch(&mut self, path: &Path) -> CoreResult<()> {
        self.watcher
            .watch(path, RecursiveMode::Recursive)
            .map_err(|e| crate::CoreError::Other(format!("Failed to watch path: {}", e)))?;
        self.watched_paths.push(path.to_path_buf());
        Ok(())
    }

    pub fn unwatch(&mut self, path: &Path) -> CoreResult<()> {
        self.watcher
            .unwatch(path)
            .map_err(|e| crate::CoreError::Other(format!("Failed to unwatch path: {}", e)))?;
        self.watched_paths.retain(|p| p != path);
        Ok(())
    }

    pub fn watched_paths(&self) -> &[PathBuf] {
        &self.watched_paths
    }
}
