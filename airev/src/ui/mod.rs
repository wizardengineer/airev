//! UI rendering module for airev.
//!
//! This is the module root for `ui/`. It re-exports `render()` as the single entry
//! point called by the event loop's `terminal.draw()` closure.
//!
//! All layout arithmetic lives in `layout.rs`. Per-panel render helpers will be
//! introduced in later plans (Phase 2 plan 02 adds keybindings; Phase 2 plan 03
//! wires real diff content).

mod layout;
pub mod keybindings;

use ratatui::{
    Frame,
    style::Style,
    text::{Line, Text},
    widgets::{List, ListItem, Paragraph},
};

use crate::app::{AppState, PanelFocus};
use crate::theme::Theme;
use layout::{compute_layout, inner_rect, panel_block, render_status_bar};

/// Renders one complete frame: 3-panel layout, placeholder content, and status bar.
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
        render_file_list(frame, left, focus, state, theme);
    }

    // Centre panel: diff view (always visible)
    render_diff(frame, center, focus, state, theme);

    // Right panel: comments (skip rendering if collapsed)
    if right.width > 0 {
        render_comments(frame, right, focus, state, theme);
    }

    // Status bar: always visible, 1 row, shows current mode.
    render_status_bar(frame, status_bar, state, theme);

    // TODO: render_help_overlay(frame, theme)
    // Added in Phase 2 plan 02 (ui/help.rs). Conditional on state.mode == Mode::HelpOverlay.
}

/// Renders the file-list panel with placeholder items.
///
/// Uses `render_stateful_widget` so the `ListState` selection highlight is applied.
/// Placeholder items show fake file paths — Phase 3 replaces these with real git data.
///
/// # Arguments
///
/// * `frame` — current render frame
/// * `area` — the `Rect` for the left panel (includes borders)
/// * `focus` — current panel focus (determines border style)
/// * `state` — mutable app state (provides `file_list_state`)
/// * `theme` — active color theme
fn render_file_list(
    frame: &mut Frame,
    area: ratatui::layout::Rect,
    focus: PanelFocus,
    state: &mut AppState,
    theme: &Theme,
) {
    let is_focused = focus == PanelFocus::FileList;
    let block = panel_block("Files", is_focused, theme);

    let items: Vec<ListItem> = vec![
        ListItem::new("modified/src/main.rs"),
        ListItem::new("modified/src/lib.rs"),
        ListItem::new("added/src/handler.rs"),
        ListItem::new("deleted/src/legacy.rs"),
        ListItem::new("modified/Cargo.toml"),
    ];

    let list = List::new(items)
        .block(block)
        .highlight_style(Style::default().fg(theme.border_active));

    frame.render_stateful_widget(list, area, &mut state.file_list_state);
}

/// Renders the diff panel with placeholder content.
///
/// Uses `Paragraph::scroll((diff_scroll, 0))` driven by the manual `u16` offset stored
/// in `AppState`. Phase 3 replaces the placeholder text with real parsed diff data.
///
/// # Arguments
///
/// * `frame` — current render frame
/// * `area` — the `Rect` for the centre panel (includes borders)
/// * `focus` — current panel focus (determines border style)
/// * `state` — app state supplying `diff_scroll`
/// * `theme` — active color theme
fn render_diff(
    frame: &mut Frame,
    area: ratatui::layout::Rect,
    focus: PanelFocus,
    state: &AppState,
    theme: &Theme,
) {
    let is_focused = focus == PanelFocus::Diff;
    let block = panel_block("Diff", is_focused, theme);
    let inner = inner_rect(area);

    frame.render_widget(block, area);

    let placeholder = build_diff_placeholder(theme);
    let paragraph = Paragraph::new(placeholder).scroll((state.diff_scroll, 0));
    frame.render_widget(paragraph, inner);
}

/// Renders the comments panel with placeholder content.
///
/// Uses `Paragraph::scroll((comments_scroll, 0))` driven by the manual `u16` offset.
/// Phase 3 replaces placeholder text with real comment thread data from SQLite.
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
    let is_focused = focus == PanelFocus::Comments;
    let block = panel_block("Comments", is_focused, theme);
    let inner = inner_rect(area);

    frame.render_widget(block, area);

    let placeholder = build_comments_placeholder(theme);
    let paragraph = Paragraph::new(placeholder).scroll((state.comments_scroll, 0));
    frame.render_widget(paragraph, inner);
}

/// Builds a placeholder `Text` representing a fake diff for layout testing.
///
/// Returns 15 lines mixing hunk headers, added, removed, and context lines styled
/// with theme colors. All content is static — Phase 3 replaces this with parsed git data.
fn build_diff_placeholder(theme: &Theme) -> Text<'static> {
    use ratatui::style::Stylize as _;
    Text::from(vec![
        Line::from("@@ -1,6 +1,8 @@").fg(theme.diff_hunk_header),
        Line::from(" use std::io;").fg(theme.diff_context),
        Line::from("-fn old_main() {").fg(theme.diff_removed),
        Line::from("+fn main() -> io::Result<()> {").fg(theme.diff_added),
        Line::from("+    // New entry point").fg(theme.diff_added),
        Line::from("     let x = 1;").fg(theme.diff_context),
        Line::from("-    println!(\"old\");").fg(theme.diff_removed),
        Line::from("+    println!(\"new\");").fg(theme.diff_added),
        Line::from(" }").fg(theme.diff_context),
        Line::from(""),
        Line::from("@@ -15,4 +17,6 @@").fg(theme.diff_hunk_header),
        Line::from(" fn helper() {").fg(theme.diff_context),
        Line::from("-    unimplemented!()").fg(theme.diff_removed),
        Line::from("+    // Phase 3 will wire real content here").fg(theme.diff_added),
        Line::from(" }").fg(theme.diff_context),
    ])
}

/// Builds placeholder `Text` for the comments panel.
///
/// Returns static comment thread entries for layout and scroll testing.
/// Phase 3 replaces these with real comment data loaded from the SQLite WAL database.
fn build_comments_placeholder(theme: &Theme) -> Text<'static> {
    use ratatui::style::Stylize as _;
    Text::from(vec![
        Line::from("  [CRITICAL] line 3").fg(theme.badge_critical),
        Line::from("  Consider error handling here."),
        Line::from(""),
        Line::from("  [MINOR] line 7").fg(theme.badge_minor),
        Line::from("  Rename for clarity."),
        Line::from(""),
        Line::from("  [INFO] line 12").fg(theme.badge_info),
        Line::from("  This pattern is idiomatic."),
        Line::from(""),
        Line::from("  (Phase 3 loads real comments)").fg(theme.diff_context),
    ])
}
