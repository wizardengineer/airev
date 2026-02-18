//! AsyncGit facade — spawns and communicates with the git background thread.
//!
//! The background thread owns `git2::Repository` for its lifetime. All requests
//! are sent via a `crossbeam_channel` sender; results arrive as `AppEvent::GitResult`.

pub mod types;
pub mod worker;

use crossbeam_channel::{unbounded, Sender};
use tokio::sync::mpsc::UnboundedSender;

use crate::event::AppEvent;
use crate::git::types::GitRequest;

/// Facade for the git background thread.
///
/// Owns the send-half of the request channel. Dropping this struct signals
/// the worker thread to exit (channel closes, worker loop terminates).
pub struct AsyncGit {
    /// Send work requests to the background thread via this sender.
    pub request_tx: Sender<GitRequest>,
}

impl AsyncGit {
    /// Spawns the background thread and returns the AsyncGit handle.
    ///
    /// The `event_tx` is cloned and captured by the thread; results arrive as
    /// `AppEvent::GitResult` on the main event channel.
    pub fn new(event_tx: UnboundedSender<AppEvent>, repo_path: String) -> Self {
        let (request_tx, request_rx) = unbounded::<GitRequest>();
        std::thread::spawn(move || {
            worker::git_worker_loop(repo_path, request_rx, event_tx);
        });
        Self { request_tx }
    }

    /// Sends a diff load request to the worker thread.
    ///
    /// Non-blocking. Returns false if the worker thread has exited (channel closed).
    pub fn load_diff(&self, request: GitRequest) -> bool {
        self.request_tx.send(request).is_ok()
    }
}
