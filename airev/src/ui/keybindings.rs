//! Keybinding dispatcher for airev.
//!
//! Translates raw crossterm `KeyEvent`s into `AppState` mutations and returns a
//! `KeyAction` telling the event loop whether to continue or quit.  The dispatcher
//! branches first on `state.mode` so that HelpOverlay, ConfirmQuit, Insert, and Normal
//! all have isolated handler functions.

use crossterm::event::{KeyCode, KeyEvent, KeyModifiers, MouseButton, MouseEvent, MouseEventKind};
use ratatui::layout::Position;

use crate::app::{AppState, Mode, PanelFocus};
use crate::git::types::{DiffMode, GitRequest};

/// Control-flow signal returned from the key dispatcher.
///
/// The event loop checks this after every keypress: `Quit` tears down the terminal
/// and exits; `Continue` immediately requests another render.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum KeyAction {
    /// Continue the event loop normally — request another render.
    Continue,
    /// Exit cleanly (no unsaved state — the caller may skip the confirm dialog).
    Quit,
}

/// Dispatches a key event to the handler matching the current mode.
///
/// Mutates `state` in place and returns a `KeyAction` signalling whether to
/// continue or quit.  The event loop should call this once per received key and
/// then redraw regardless of the return value (except on `Quit`).
///
/// # Arguments
///
/// * `key`   — the raw crossterm key event (code + modifiers)
/// * `state` — mutable reference to all UI state
pub fn handle_key(key: KeyEvent, state: &mut AppState) -> KeyAction {
    match state.mode {
        Mode::HelpOverlay => handle_help(key, state),
        Mode::ConfirmQuit => handle_confirm_quit(key, state),
        Mode::Normal => handle_normal(key, state),
        Mode::Insert => handle_insert(key, state),
    }
}

// ---------------------------------------------------------------------------
// Normal mode
// ---------------------------------------------------------------------------

/// Handles a key event while in Normal mode.
///
/// Delegates scroll keys to `handle_scroll_key`, file-list-specific keys to
/// `handle_file_list_key`, and handles focus, panel resize, hunk navigation,
/// and mode transitions inline.
///
/// # Arguments
///
/// * `key`   — the raw crossterm key event
/// * `state` — mutable reference to all UI state
fn handle_normal(key: KeyEvent, state: &mut AppState) -> KeyAction {
    // Try scroll keys first (j/k/g/G/Ctrl-d/u/f/b).
    if let Some(action) = handle_scroll_key(key, state) {
        return action;
    }
    // Try file-list-specific keys (Enter, l, Tab) when focused on the file list.
    if let Some(action) = handle_file_list_key(key, state) {
        return action;
    }

    match key.code {
        // Panel focus
        KeyCode::Char('H') => {
            state.focus = state.focus.prev();
            KeyAction::Continue
        }
        KeyCode::Char('L') => {
            state.focus = state.focus.next();
            KeyAction::Continue
        }

        // File list navigation
        KeyCode::Char('{') => { state.prev_file(); KeyAction::Continue }
        KeyCode::Char('}') => { state.next_file(); KeyAction::Continue }

        // Hunk navigation — calls real AppState methods wired in Phase 3 Plan 02.
        KeyCode::Char('[') => { state.prev_hunk(); KeyAction::Continue }
        KeyCode::Char(']') => { state.next_hunk(); KeyAction::Continue }

        // Diff panel resize
        KeyCode::Char('<') => { state.shrink_diff_panel(); KeyAction::Continue }
        KeyCode::Char('>') => { state.grow_diff_panel(); KeyAction::Continue }

        // Help overlay
        KeyCode::Char('?') => {
            state.help_scroll = 0;
            state.mode = Mode::HelpOverlay;
            KeyAction::Continue
        }

        // Quit / confirm-quit
        KeyCode::Char('q') | KeyCode::Esc => {
            if state.has_unsaved_comments {
                state.mode = Mode::ConfirmQuit;
                KeyAction::Continue
            } else {
                KeyAction::Quit
            }
        }

        _ => KeyAction::Continue,
    }
}

/// Handles file-list-specific keys in Normal mode: Enter, l (jump), Tab (mode cycle).
///
/// Returns `Some(KeyAction)` when the key was consumed, `None` when the key
/// should fall through to the rest of the Normal handler.
///
/// # Arguments
///
/// * `key`   — the raw crossterm key event
/// * `state` — mutable reference to all UI state
fn handle_file_list_key(key: KeyEvent, state: &mut AppState) -> Option<KeyAction> {
    match key.code {
        // Enter and l both jump to the selected file when the file list is focused.
        KeyCode::Enter
        | KeyCode::Char('l') if state.focus == PanelFocus::FileList => {
            state.jump_to_selected_file();
            Some(KeyAction::Continue)
        }

        // Tab cycles the diff mode regardless of focused panel, then sends a new request.
        KeyCode::Tab => {
            let next_mode = match state.diff_mode {
                DiffMode::Unstaged => DiffMode::Staged,
                DiffMode::Staged => DiffMode::BranchComparison,
                DiffMode::BranchComparison => DiffMode::CommitRange,
                DiffMode::CommitRange => DiffMode::Unstaged,
            };
            state.diff_mode = next_mode;
            state.diff_loading = true;
            state.diff_scroll = 0;
            if let Some(ref tx) = state.git_tx {
                let _ = tx.send(GitRequest::LoadDiff(next_mode));
            }
            Some(KeyAction::Continue)
        }

        _ => None,
    }
}

/// Handles scroll-related keys in Normal mode: j / k / g / G and Ctrl combos.
///
/// Returns `Some(KeyAction)` when the key was consumed, `None` when the key
/// should fall through to the rest of the Normal handler.
///
/// # Arguments
///
/// * `key`   — the raw crossterm key event
/// * `state` — mutable reference to all UI state
fn handle_scroll_key(key: KeyEvent, state: &mut AppState) -> Option<KeyAction> {
    let ctrl = key.modifiers.contains(KeyModifiers::CONTROL);

    match key.code {
        KeyCode::Char('j') => {
            state.scroll_down(1);
            Some(KeyAction::Continue)
        }
        KeyCode::Char('k') => {
            state.scroll_up(1);
            Some(KeyAction::Continue)
        }
        KeyCode::Char('g') => {
            state.scroll_top();
            Some(KeyAction::Continue)
        }
        KeyCode::Char('G') => {
            state.scroll_bottom();
            Some(KeyAction::Continue)
        }
        KeyCode::Char('d') if ctrl => {
            state.half_page_down();
            Some(KeyAction::Continue)
        }
        KeyCode::Char('u') if ctrl => {
            state.half_page_up();
            Some(KeyAction::Continue)
        }
        KeyCode::Char('f') if ctrl => {
            state.full_page_down();
            Some(KeyAction::Continue)
        }
        KeyCode::Char('b') if ctrl => {
            state.full_page_up();
            Some(KeyAction::Continue)
        }
        _ => None,
    }
}

// ---------------------------------------------------------------------------
// HelpOverlay mode
// ---------------------------------------------------------------------------

/// Handles a key event while the help overlay is visible.
///
/// Any of `?`, `Esc`, or `q` dismisses the overlay and returns to Normal mode.
/// All other keys are silently ignored.
///
/// # Arguments
///
/// * `key`   — the raw crossterm key event
/// * `state` — mutable reference to all UI state
fn handle_help(key: KeyEvent, state: &mut AppState) -> KeyAction {
    match key.code {
        KeyCode::Char('j') => {
            state.help_scroll = state.help_scroll.saturating_add(1);
            KeyAction::Continue
        }
        KeyCode::Char('k') => {
            state.help_scroll = state.help_scroll.saturating_sub(1);
            KeyAction::Continue
        }
        KeyCode::Char('g') => {
            state.help_scroll = 0;
            KeyAction::Continue
        }
        KeyCode::Char('G') => {
            state.help_scroll = u16::MAX;
            KeyAction::Continue
        }
        KeyCode::Char('?') | KeyCode::Esc | KeyCode::Char('q') => {
            state.mode = Mode::Normal;
            KeyAction::Continue
        }
        _ => KeyAction::Continue,
    }
}

// ---------------------------------------------------------------------------
// ConfirmQuit mode
// ---------------------------------------------------------------------------

/// Handles a key event while the quit-confirmation dialog is active.
///
/// `y` / `Y` confirms the quit and returns `Quit`.  `n` / `N` / `Esc` cancels
/// and returns to Normal mode.  All other keys are ignored.
///
/// # Arguments
///
/// * `key`   — the raw crossterm key event
/// * `state` — mutable reference to all UI state
fn handle_confirm_quit(key: KeyEvent, state: &mut AppState) -> KeyAction {
    match key.code {
        KeyCode::Char('y') | KeyCode::Char('Y') => KeyAction::Quit,
        KeyCode::Char('n') | KeyCode::Char('N') | KeyCode::Esc => {
            state.mode = Mode::Normal;
            KeyAction::Continue
        }
        _ => KeyAction::Continue,
    }
}

// ---------------------------------------------------------------------------
// Insert mode
// ---------------------------------------------------------------------------

/// Handles a key event while in Insert mode (comment editing placeholder).
///
/// `Esc` returns to Normal mode.  All other keys are passed through without
/// mutation — comment text editing is wired in Phase 5.
///
/// # Arguments
///
/// * `key`   — the raw crossterm key event
/// * `state` — mutable reference to all UI state
fn handle_insert(key: KeyEvent, state: &mut AppState) -> KeyAction {
    match key.code {
        KeyCode::Esc => {
            state.mode = Mode::Normal;
            KeyAction::Continue
        }
        _ => KeyAction::Continue,
    }
}

// ---------------------------------------------------------------------------
// Mouse events
// ---------------------------------------------------------------------------

/// Handles a mouse event: click-to-focus and scroll-wheel.
///
/// Left click on a panel sets focus to that panel. Scroll wheel up/down
/// scrolls the focused panel by 3 lines (matching typical terminal scroll
/// speed). Mouse events in HelpOverlay mode scroll the help overlay.
///
/// # Arguments
///
/// * `mouse` — the crossterm mouse event
/// * `state` — mutable reference to all UI state
pub fn handle_mouse(mouse: MouseEvent, state: &mut AppState) -> KeyAction {
    match mouse.kind {
        MouseEventKind::Down(MouseButton::Left) => {
            handle_mouse_click(mouse.column, mouse.row, state)
        }
        MouseEventKind::ScrollUp => handle_mouse_scroll_up(state),
        MouseEventKind::ScrollDown => handle_mouse_scroll_down(state),
        _ => KeyAction::Continue,
    }
}

/// Sets panel focus based on the clicked screen position.
///
/// Checks each cached panel rect in `state.panel_rects`. Panels with zero width
/// are skipped so collapsed panels cannot receive focus via click.
///
/// # Arguments
///
/// * `col`   — column (x) of the mouse click
/// * `row`   — row (y) of the mouse click
/// * `state` — mutable reference to all UI state
fn handle_mouse_click(col: u16, row: u16, state: &mut AppState) -> KeyAction {
    let pos = Position { x: col, y: row };
    let [left, center, right] = state.panel_rects;

    if left.width > 0 && left.contains(pos) {
        state.focus = PanelFocus::FileList;
    } else if center.contains(pos) {
        state.focus = PanelFocus::Diff;
    } else if right.width > 0 && right.contains(pos) {
        state.focus = PanelFocus::Comments;
    }

    KeyAction::Continue
}

/// Scrolls up by 3 lines. Scrolls the help overlay when in HelpOverlay mode.
///
/// # Arguments
///
/// * `state` — mutable reference to all UI state
fn handle_mouse_scroll_up(state: &mut AppState) -> KeyAction {
    if state.mode == Mode::HelpOverlay {
        state.help_scroll = state.help_scroll.saturating_sub(3);
    } else {
        state.scroll_up(3);
    }
    KeyAction::Continue
}

/// Scrolls down by 3 lines. Scrolls the help overlay when in HelpOverlay mode.
///
/// # Arguments
///
/// * `state` — mutable reference to all UI state
fn handle_mouse_scroll_down(state: &mut AppState) -> KeyAction {
    if state.mode == Mode::HelpOverlay {
        state.help_scroll = state.help_scroll.saturating_add(3);
    } else {
        state.scroll_down(3);
    }
    KeyAction::Continue
}
