//! External-file watch registry.

use std::path::Path;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;

use crate::commands::{spawn_watch_thread, AppState};

/// Starts (or restarts) watching the file behind a note.
pub fn watch_note(state: &AppState, note_id: i64, path: &Path) {
    stop_watching(state, note_id);
    let stop = Arc::new(AtomicBool::new(false));
    if let Ok(mut map) = state.watchers.lock() {
        map.insert(note_id, stop.clone());
    }
    spawn_watch_thread(state, note_id, path.to_path_buf(), stop);
}

/// Stops watching a note, if it is watched.
pub fn stop_watching(state: &AppState, note_id: i64) {
    if let Ok(mut map) = state.watchers.lock() {
        if let Some(flag) = map.remove(&note_id) {
            flag.store(true, Ordering::Relaxed);
        }
    }
}
