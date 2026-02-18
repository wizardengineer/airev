//! UI rendering module for airev.
//!
//! This is the module root for `ui/`. It re-exports `render()` as the single entry
//! point called by the event loop's `terminal.draw()` closure.
//!
//! All layout arithmetic lives in `layout.rs`. Diff panel rendering lives in
//! `diff_view.rs` and file-list rendering lives in `file_tree.rs`. The comments
//! panel remains a Paragraph placeholder until Phase 5.

mod layout;
pub mod diff_view;
pub mod file_tree;
pub mod help;
pub mod keybindings;

use ratatui::{
    Frame,
    text::Line,
    widgets::Paragraph,
};

use crate::app::{AppState, Mode, PanelFocus};
use crate::theme::Theme;
use layout::{compute_layout, inner_rect, panel_block, render_status_bar};

/// Renders one complete frame: 3-panel layout, real diff/file-list content, and status bar.
///
/// Called exactly once per `AppEvent::Render` inside `terminal.draw()`. This is the
/// only location where `terminal.draw()` is called in the application — never call
/// it from anywhere else.
///
/// After computing the layout, viewport heights are written back into `state` so that
/// scroll operations triggered by the *next* keypress can compute half-page and
/// full-page distances correctly. The one-frame lag is imperceptible in practice.
///
/// # Arguments
///
/// * `frame` — current render frame provided by `terminal.draw()`
/// * `state` — mutable reference to app state (viewport heights are cached here)
/// * `theme` — active color theme
pub fn render(frame: &mut Frame, state: &mut AppState, theme: &Theme) {
    let [left, center, right, status_bar] = compute_layout(frame, state);

    // Cache viewport heights BEFORE rendering panels so they are available for the
    // next keypress cycle. Uses inner_rect() to strip the 1-cell border on each side.
    state.file_list_viewport_height = inner_rect(left).height;
    state.diff_viewport_height = inner_rect(center).height;
    state.comments_viewport_height = inner_rect(right).height;

    let focus = state.focus;

    // Left panel: file list (skip rendering if collapsed)
    if left.width > 0 {
        file_tree::render_file_list(frame, left, focus, state, theme);
    }

    // Centre panel: diff view (always visible)
    diff_view::render_diff(frame, center, focus, state, theme);

    // Right panel: comments placeholder (skip rendering if collapsed; Phase 5 replaces)
    if right.width > 0 {
        render_comments(frame, right, focus, state, theme);
    }

    // Status bar: always visible, 1 row, shows current mode.
    render_status_bar(frame, status_bar, state, theme);

    // Help overlay: rendered after all panels so it sits on top.
    // Clear is called inside render_help_overlay() to erase the background.
    if state.mode == Mode::HelpOverlay {
        help::render_help_overlay(frame, theme);
    }
}

/// Renders the comments panel with placeholder content.
///
/// Uses `Paragraph::scroll((comments_scroll, 0))` driven by the manual `u16` offset.
/// Phase 5 replaces this placeholder with real comment thread data from SQLite.
///
/// # Arguments
///
/// * `frame` — current render frame
/// * `area` — the `Rect` for the right panel (includes borders)
/// * `focus` — current panel focus (determines border style)
/// * `state` — app state supplying `comments_scroll`
/// * `theme` — active color theme
fn render_comments(
    frame: &mut Frame,
    area: ratatui::layout::Rect,
    focus: PanelFocus,
    state: &AppState,
    theme: &Theme,
) {
    use ratatui::style::Stylize as _;
    let is_focused = focus == PanelFocus::Comments;
    let block = panel_block("Comments", is_focused, theme);
    let inner = inner_rect(area);

    frame.render_widget(block, area);

    let placeholder = ratatui::text::Text::from(vec![
        Line::from("  (Phase 5 loads real comments)").fg(theme.diff_context),
    ]);
    let paragraph = Paragraph::new(placeholder).scroll((state.comments_scroll, 0));
    frame.render_widget(paragraph, inner);
}
