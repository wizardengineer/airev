//! Event bus for airev.
//!
//! All user input, timer ticks, and background-task results are normalised into
//! a single `AppEvent` enum and sent over a tokio unbounded MPSC channel. The
//! main loop receives from this channel and dispatches accordingly.
//!
//! Two independent intervals drive the render and logic cycles:
//! - **Render interval** (33 ms ≈ 30 FPS) — triggers a `terminal.draw()` call.
//! - **Tick interval** (250 ms = 4 Hz) — triggers application-state updates.
//!
//! Keeping them independent allows tuning render frequency (e.g., drop to 20 FPS
//! on battery) without affecting logic frequency, and vice-versa.

use crossterm::event::{Event, EventStream, KeyEvent, KeyEventKind, MouseEvent};
use futures::{FutureExt, StreamExt};
use std::time::Duration;
use tokio::sync::mpsc;
use tokio::time::interval;

/// All events the application can receive from any source.
///
/// Marked `#[non_exhaustive]` so that new variants added in later phases
/// (e.g., LSP diagnostics, AI streaming tokens) do not break exhaustive match
/// arms in existing handlers.
#[derive(Debug)]
#[non_exhaustive]
pub enum AppEvent {
    /// A key press from the terminal (`KeyEventKind::Press` only).
    ///
    /// Release and repeat events are filtered in [`spawn_event_task`] to avoid
    /// double-firing on Windows, which synthesises both press and release for
    /// every keystroke.
    Key(KeyEvent),
    /// A mouse event from the terminal (click, scroll, move).
    Mouse(MouseEvent),
    /// Terminal was resized to (columns, rows).
    Resize(u16, u16),
    /// Logic tick for state updates (4 Hz / 250 ms).
    Tick,
    /// Render tick — triggers a `terminal.draw()` call (≈30 FPS / 33 ms).
    Render,
    /// A watched file changed on disk.
    FileChanged,
    /// Result from the git background thread.
    GitResult(Box<crate::git::types::GitResultPayload>),
    /// Result from the database background task.
    DbResult,
    /// Quit signal (from `q` key or SIGTERM).
    Quit,
}

/// Holds the sender and receiver ends of the unified event channel.
///
/// The sender (`tx`) is cloned and distributed to background tasks;
/// the receiver (`rx`) is owned by the main event loop.
pub struct EventHandler {
    /// Send half — clone this for each background task that produces events.
    pub tx: mpsc::UnboundedSender<AppEvent>,
    /// Receive half — owned by the main loop; call `.recv().await` to block
    /// until the next event.
    pub rx: mpsc::UnboundedReceiver<AppEvent>,
}

impl EventHandler {
    /// Creates a new `EventHandler` with a fresh unbounded channel.
    ///
    /// Unbounded is appropriate here because the producer side (terminal events
    /// + timers) generates events at a bounded hardware rate, and the consumer
    /// (main loop) always keeps up. If backpressure ever becomes a concern in a
    /// future phase, swap to a bounded channel at that point.
    pub fn new() -> Self {
        let (tx, rx) = mpsc::unbounded_channel();
        Self { tx, rx }
    }
}

impl Default for EventHandler {
    fn default() -> Self {
        Self::new()
    }
}

/// Spawns the background tokio task that drives the unified event channel.
///
/// The task runs until the `tx` sender is dropped (i.e., the `EventHandler` is
/// dropped). Two fully independent `tokio::time::interval` timers drive the
/// render and logic cycles; crossterm input is polled via `EventStream`.
///
/// # Key implementation choices
///
/// - `reader.next().fuse()` — required so that if the crossterm stream
///   terminates unexpectedly, `tokio::select!` does not keep polling a
///   completed future (which would cause a panic).
/// - `KeyEventKind::Press` filter — Windows fires both `Press` and `Release`
///   for every keystroke. Without the filter, every key press appears twice.
/// - Send errors are silently ignored (`let _ = tx.send(…)`) — if the
///   receiver has been dropped, the task simply exits on the next loop
///   iteration when it tries to send.
pub fn spawn_event_task(tx: mpsc::UnboundedSender<AppEvent>) {
    tokio::spawn(async move {
        let mut tick_interval = interval(Duration::from_millis(250));
        let mut render_interval = interval(Duration::from_millis(33));
        let mut reader = EventStream::new();

        loop {
            let tick_tick = tick_interval.tick();
            let render_tick = render_interval.tick();
            let crossterm_event = reader.next().fuse();

            tokio::select! {
                _ = tick_tick => {
                    let _ = tx.send(AppEvent::Tick);
                }
                _ = render_tick => {
                    let _ = tx.send(AppEvent::Render);
                }
                maybe_event = crossterm_event => {
                    match maybe_event {
                        Some(Ok(Event::Key(key))) => {
                            if key.kind == KeyEventKind::Press {
                                let _ = tx.send(AppEvent::Key(key));
                            }
                        }
                        Some(Ok(Event::Resize(w, h))) => {
                            let _ = tx.send(AppEvent::Resize(w, h));
                        }
                        Some(Ok(Event::Mouse(mouse))) => {
                            let _ = tx.send(AppEvent::Mouse(mouse));
                        }
                        _ => {}
                    }
                }
            }
        }
    });
}
