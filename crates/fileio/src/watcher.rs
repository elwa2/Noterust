//! Filesystem watching for notes opened directly from disk.
//!
//! The UI is notified when the file changes outside the app so it can show a
//! "this file changed on disk" banner instead of silently overwriting it.

use std::path::{Path, PathBuf};
use std::sync::mpsc::{channel, Receiver};
use std::time::{Duration, Instant};

use notify::{Config, Event, RecommendedWatcher, RecursiveMode, Watcher};

/// A watcher that reports change events for one file.
pub struct FileWatcher {
    _watcher: RecommendedWatcher,
    receiver: Receiver<PathBuf>,
}

impl FileWatcher {
    /// Watches `path` for modifications. The event channel never blocks the OS
    /// watcher thread and de-duplicates bursts within 400 ms.
    pub fn new(path: &Path) -> Result<Self, crate::FileIoError> {
        let (sender, receiver) = channel::<PathBuf>();
        let target = path.to_path_buf();
        let mut last: Option<Instant> = None;

        let mut watcher = RecommendedWatcher::new(
            move |event: notify::Result<Event>| {
                let Ok(event) = event else {
                    return;
                };
                if !event.kind.is_modify() && !event.kind.is_create() {
                    return;
                }
                if !event.paths.iter().any(|candidate| candidate == &target) {
                    return;
                }
                let now = Instant::now();
                if last.is_some_and(|previous| {
                    now.duration_since(previous) < Duration::from_millis(400)
                }) {
                    return;
                }
                last = Some(now);
                let _ = sender.send(target.clone());
            },
            Config::default().with_poll_interval(Duration::from_millis(500)),
        )
        .map_err(|err| crate::FileIoError::Watch(err.to_string()))?;

        watcher
            .watch(path, RecursiveMode::NonRecursive)
            .map_err(|err| crate::FileIoError::Watch(err.to_string()))?;

        Ok(Self {
            _watcher: watcher,
            receiver,
        })
    }

    /// Blocks until the file changes or `timeout` elapses.
    pub fn next_change(&self, timeout: Duration) -> Option<PathBuf> {
        self.receiver.recv_timeout(timeout).ok()
    }
}

/// Convenience helper used by the command layer.
pub fn watch_path(path: &Path) -> Result<FileWatcher, crate::FileIoError> {
    FileWatcher::new(path)
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;
    use std::time::Duration as StdDuration;

    #[test]
    fn reports_external_modification() {
        let dir = std::env::temp_dir().join(format!("noterust-watch-{}", std::process::id()));
        fs::create_dir_all(&dir).expect("dir");
        let path = dir.join("watched.txt");
        fs::write(&path, "a").expect("seed");

        let watcher = FileWatcher::new(&path).expect("watch");
        // Give the OS watcher a moment to attach.
        std::thread::sleep(StdDuration::from_millis(300));
        fs::write(&path, "b").expect("modify");

        let changed = watcher.next_change(StdDuration::from_secs(5));
        assert!(changed.is_some(), "expected a change event");
    }
}
