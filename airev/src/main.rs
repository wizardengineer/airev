//! airev — AI-assisted code review TUI.
//!
//! Entry point for the `airev` binary. Wires together the terminal lifecycle
//! (`tui`), unified event bus (`event`), placeholder UI (`ui`), theme system
//! (`theme`), and the shared WAL-mode SQLite database (`airev-core`).
//!
//! # Startup sequence (order matters — see RESEARCH.md Pitfall 6)
//!
//! 1. Load theme from XDG config — read-only, safe before terminal init.
//! 2. `install_panic_hook()` — installed first so it is the innermost hook.
//!    Restores the terminal before the panic message prints.
//! 3. `register_sigterm()` — returns `Arc<AtomicBool>` polled in the event loop.
//! 4. `init_tui()` — enters alternate screen and enables raw mode.
//! 5. Create event channel and `spawn_event_task()`.
//! 6. `create_dir_all(".airev")` + `open_db()` — DB opened before first frame
//!    so there is no "loading" state to manage.
//!
//! # Safety
//!
//! `restore_tui()` is called after the event loop exits (normal quit, 'q' key,
//! SIGTERM, or `None` channel close). The `?` operator is only used before
//! `init_tui()` or inside the Render arm — draw errors propagate out of the loop
//! and reach `restore_tui()` after `break`. The panic hook covers unexpected panics.

mod app;
mod event;
mod theme;
mod tui;
mod ui;

use std::sync::atomic::Ordering;

/// Returns the path to the airev config file.
///
/// Prefers `$XDG_CONFIG_HOME/airev/config.toml`; falls back to
/// `~/.config/airev/config.toml` when the env var is absent.
fn config_path() -> std::path::PathBuf {
    let base = std::env::var("XDG_CONFIG_HOME")
        .ok()
        .map(std::path::PathBuf::from)
        .or_else(|| {
            std::env::var("HOME")
                .ok()
                .map(|h| std::path::PathBuf::from(h).join(".config"))
        })
        .unwrap_or_else(|| std::path::PathBuf::from(".config"));
    base.join("airev").join("config.toml")
}

/// Loads the theme name from `~/.config/airev/config.toml`.
///
/// Returns `"dark"` if the file does not exist, cannot be parsed, or has no
/// `theme` key. Never panics — config errors are soft failures printed to stderr.
fn load_theme_name() -> String {
    let path = config_path();
    let raw = match std::fs::read_to_string(&path) {
        Ok(s) => s,
        Err(_) => return "catppuccin-mocha".to_owned(),
    };
    let table: toml::Table = match toml::from_str(&raw) {
        Ok(t) => t,
        Err(e) => {
            eprintln!("airev: config parse error in {:?}: {}", path, e);
            return "catppuccin-mocha".to_owned();
        }
    };
    table
        .get("theme")
        .and_then(|v| v.as_str())
        .unwrap_or("catppuccin-mocha")
        .to_owned()
}

#[tokio::main]
async fn main() -> std::io::Result<()> {
    // Step 0: load theme from config — read-only, safe before terminal init.
    let theme = theme::Theme::from_name(&load_theme_name());
    let mut state = app::AppState::default();

    // Step 1: panic hook installed first — innermost hook restores terminal.
    tui::install_panic_hook();

    // Step 2: SIGTERM flag — polled in the 50ms heartbeat arm below.
    let term_flag = tui::register_sigterm();

    // Step 3: enter alternate screen and raw mode.
    let mut terminal = tui::init_tui()?;

    // Step 4: create event channel and spawn the background event task.
    let handler = event::EventHandler::new();
    event::spawn_event_task(handler.tx.clone());
    let mut rx = handler.rx;

    // Step 5: open the WAL-mode SQLite database before drawing the first frame.
    // Create the directory if it does not already exist.
    std::fs::create_dir_all(".airev")?;
    let _db = airev_core::db::open_db(".airev/reviews.db")
        .await
        .map_err(|e| std::io::Error::new(std::io::ErrorKind::Other, e))?;

    // Event loop — exits only via `break`, never via `?`.
    // This guarantees `restore_tui()` is always reached after the loop.
    'event_loop: loop {
        tokio::select! {
            // Heartbeat: guarantees SIGTERM is checked at least every 50ms,
            // even when no crossterm/tick/render events arrive.
            // Without this arm, a quiescent terminal blocks forever in rx.recv()
            // and the SIGTERM flag is never polled.
            _ = tokio::time::sleep(std::time::Duration::from_millis(50)) => {
                if term_flag.load(Ordering::Relaxed) {
                    break 'event_loop;
                }
            }
            maybe_event = rx.recv() => {
                match maybe_event {
                    Some(event::AppEvent::Render) => {
                        // Exactly one draw() call per Render event — never elsewhere.
                        terminal.draw(|frame| ui::render(frame, &mut state, &theme))?;
                    }
                    Some(event::AppEvent::Key(key)) => {
                        use crossterm::event::KeyCode;
                        if matches!(key.code, KeyCode::Char('q') | KeyCode::Char('Q')) {
                            break 'event_loop;
                        }
                    }
                    Some(event::AppEvent::Resize(_, _)) => {
                        // Resize is handled automatically by ratatui on the next Render:
                        // frame.area() returns the new terminal size. No manual relayout
                        // is needed here in Phase 1. Future phases may store the new
                        // dimensions in app state for widget calculations.
                    }
                    Some(event::AppEvent::Quit) | None => break 'event_loop,
                    _ => {}
                }
                // Check SIGTERM after every event too, not just on the heartbeat,
                // so quit latency is at most one event cycle rather than 50ms.
                if term_flag.load(Ordering::Relaxed) {
                    break 'event_loop;
                }
            }
        }
    }

    // Restore the terminal at the single exit point of the loop.
    // Called unconditionally — covers normal quit, 'q' key, SIGTERM, and
    // channel close. The panic hook handles the panic path separately.
    tui::restore_tui()?;
    Ok(())
}
