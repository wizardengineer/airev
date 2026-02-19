//! Terminal lifecycle management for airev.
//!
//! **Why stderr, not stdout?**
//! The MCP server binary (`airev-mcp`) communicates with the host editor via JSON-RPC
//! on stdin/stdout. To avoid mixing TUI escape sequences with the JSON-RPC byte stream,
//! the TUI renders entirely to stderr. This lets both processes share the same terminal
//! session: the editor's extension reads stdout from `airev-mcp` while the human user
//! sees the TUI on stderr. It also means shell pipelines (`airev | …`) remain clean.

use crossterm::event::{DisableMouseCapture, EnableMouseCapture};
use crossterm::execute;
use crossterm::terminal::{
    disable_raw_mode, enable_raw_mode, EnterAlternateScreen, LeaveAlternateScreen,
};
use ratatui::backend::CrosstermBackend;
use ratatui::Terminal;
use signal_hook::consts::SIGTERM;
use signal_hook::flag::register;
use std::io::{stderr, BufWriter, Stderr};
use std::panic;
use std::sync::{atomic::AtomicBool, Arc};

/// The terminal type used by airev — CrosstermBackend over a buffered stderr writer.
///
/// Using `BufWriter<Stderr>` batches escape sequences into fewer write(2) syscalls,
/// reducing flicker on high-frequency draws (30 FPS render interval).
pub type Tui = Terminal<CrosstermBackend<BufWriter<Stderr>>>;

/// Initialise the terminal for TUI rendering.
///
/// Creates a `CrosstermBackend` backed by a `BufWriter<Stderr>`, enables raw mode,
/// and enters the alternate screen. Call [`restore_tui`] at every exit path.
///
/// # Errors
///
/// Returns `Err` if `enable_raw_mode`, `execute!`, or `Terminal::new` fails.
pub fn init_tui() -> std::io::Result<Tui> {
    let mut out = BufWriter::new(stderr());
    enable_raw_mode()?;
    execute!(out, EnterAlternateScreen, EnableMouseCapture)?;
    Terminal::new(CrosstermBackend::new(out))
}

/// Restore the terminal to its pre-TUI state.
///
/// Disables raw mode and leaves the alternate screen. This function is idempotent
/// and must be called at every exit path — including the panic hook — because
/// ratatui 0.30 does NOT auto-restore the terminal on `Drop` (see GitHub #2087).
///
/// # Errors
///
/// Returns `Err` if `disable_raw_mode` or `execute!` fails. Callers in the panic
/// hook should use `let _ = restore_tui();` and ignore the error (best-effort only).
pub fn restore_tui() -> std::io::Result<()> {
    disable_raw_mode()?;
    execute!(stderr(), LeaveAlternateScreen, DisableMouseCapture)?;
    Ok(())
}

/// Install a panic hook that restores the terminal before printing the panic message.
///
/// Must be called **before** [`init_tui`]. Chains onto any previously installed hook
/// so that the default (or test framework's) panic printer still runs after the
/// terminal is restored. Without this hook, a panic leaves the terminal in raw mode
/// with the alternate screen active, making the panic message invisible and the
/// shell unusable until the user types `reset`.
pub fn install_panic_hook() {
    let original_hook = panic::take_hook();
    panic::set_hook(Box::new(move |panic_info| {
        // Restore terminal first so the panic message is readable.
        // Errors from restore_tui() are intentionally ignored here —
        // we're already in a panic, best-effort cleanup only.
        let _ = restore_tui();
        original_hook(panic_info);
    }));
}

/// Register a SIGTERM handler that sets an `AtomicBool` flag.
///
/// Returns an `Arc<AtomicBool>` that transitions from `false` to `true` when
/// the process receives SIGTERM. Poll this flag in the main event loop — the
/// event task sends `AppEvent::Quit` when the flag is set (see `event.rs`).
///
/// # Panics
///
/// Panics if the OS refuses to register the signal handler (extremely rare —
/// treated as a fatal initialisation error rather than a recoverable condition).
pub fn register_sigterm() -> Arc<AtomicBool> {
    let term = Arc::new(AtomicBool::new(false));
    // Safety: signal_hook::flag::register is safe for AtomicBool targets —
    // the handler only calls atomic_store, which is async-signal-safe.
    register(SIGTERM, Arc::clone(&term)).expect("Failed to register SIGTERM handler");
    term
}
