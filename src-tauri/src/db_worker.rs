//! Single-threaded database worker.
//!
//! The SQLite connection lives on one dedicated thread. Commands send closures
//! to it and wait for the result, so the UI thread and the async runtime never
//! touch SQLite and can never freeze on disk I/O.

use std::sync::mpsc::{channel, Receiver, Sender};
use std::thread;

use noterust_db::{Db, DbError};

type Job = Box<dyn FnOnce(&mut Db) + Send>;

/// A handle to the database thread.
#[derive(Clone)]
pub struct DbHandle {
    sender: Sender<Job>,
}

impl DbHandle {
    /// Starts the worker thread that owns `db`.
    pub fn spawn(mut db: Db) -> Self {
        let (sender, receiver): (Sender<Job>, Receiver<Job>) = channel();
        thread::Builder::new()
            .name("noterust-db".to_string())
            .spawn(move || {
                while let Ok(job) = receiver.recv() {
                    job(&mut db);
                }
                tracing::debug!("database worker stopped");
            })
            .expect("failed to spawn database thread");
        Self { sender }
    }

    /// Runs `f` on the database thread and waits for its result.
    pub fn call<T, F>(&self, f: F) -> Result<T, String>
    where
        T: Send + 'static,
        F: FnOnce(&mut Db) -> Result<T, DbError> + Send + 'static,
    {
        let (reply_tx, reply_rx) = channel();
        self.sender
            .send(Box::new(move |db: &mut Db| {
                let _ = reply_tx.send(f(db));
            }))
            .map_err(|_| "تعذر الوصول إلى قاعدة البيانات".to_string())?;
        reply_rx
            .recv()
            .map_err(|_| "توقف خيط قاعدة البيانات".to_string())?
            .map_err(|err| err.to_string())
    }
}
