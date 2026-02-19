//! Help overlay renderer for airev.
//!
//! Provides `render_help_overlay()` which draws a centred modal box over the existing
//! panel layout using ratatui's `Clear` widget to erase the background first.  The
//! overlay is rendered inside the same `terminal.draw()` closure as all other panels —
//! calling `frame.render_widget(Clear, area)` before the bordered `Paragraph` achieves
//! the modal effect without a second draw call.

use ratatui::{
    Frame,
    layout::Constraint,
    text::{Line, Text},
    widgets::{Block, Clear, Paragraph, Wrap},
};

use crate::theme::Theme;

/// Renders the help overlay as a centred modal on top of the 3-panel layout.
///
/// Erases the overlay area with `Clear`, then draws a bordered `Block` titled
/// `" Help  — j/k scroll, ? or Esc to dismiss "` and a `Paragraph` containing all
/// keybinding descriptions. The paragraph scrolls vertically by `help_scroll` rows,
/// enabling navigation of long help text on short terminals.
///
/// If the terminal is narrower than 60 columns the overlay is skipped to avoid
/// a zero-height `Rect` panic (Pitfall 6 from Phase 2 research).
///
/// # Arguments
///
/// * `frame` — current render frame provided by `terminal.draw()`
/// * `theme` — active color theme (supplies `border_active` for the modal border)
/// * `help_scroll` — vertical scroll offset; j/k in HelpOverlay mode mutate this field
pub fn render_help_overlay(frame: &mut Frame, theme: &Theme, help_scroll: u16) {
    // Guard: skip on very narrow terminals to prevent zero-height Rect (Pitfall 6).
    if frame.area().width < 60 {
        return;
    }

    let overlay_area = frame
        .area()
        .centered(Constraint::Percentage(80), Constraint::Percentage(80));

    // Erase the background behind the modal before drawing content.
    frame.render_widget(Clear, overlay_area);

    let block = Block::bordered()
        .title(" Help  — j/k scroll, ? or Esc to dismiss ")
        .border_style(ratatui::style::Style::default().fg(theme.border_active));

    let help_text = build_help_text();

    frame.render_widget(
        Paragraph::new(help_text)
            .block(block)
            .wrap(Wrap { trim: false })
            .scroll((help_scroll, 0)),
        overlay_area,
    );
}

/// Builds the help text as a multi-line `Text` value.
///
/// Returns all keybinding descriptions grouped by section.  No color styling is
/// applied to the body text — theme coloring for help content is reserved for
/// Phase 5+ polish.
fn build_help_text() -> Text<'static> {
    Text::from(vec![
        Line::from("Navigation"),
        Line::from("  j / k         Scroll down / up one line"),
        Line::from("  g / G         Jump to top / bottom"),
        Line::from("  Ctrl-d / u    Scroll half page down / up"),
        Line::from("  Ctrl-f / b    Scroll full page down / up"),
        Line::from("  H / L         Move panel focus left / right"),
        Line::from(""),
        Line::from("File List"),
        Line::from("  { / }         Previous / next file"),
        Line::from("  Enter / l     Jump to selected file in diff view"),
        Line::from(""),
        Line::from("Diff View"),
        Line::from("  [ / ]         Previous / next hunk"),
        Line::from("  < / >         Shrink / grow diff panel by 5%"),
        Line::from(""),
        Line::from("Diff Mode  (Tab cycles through all modes)"),
        Line::from("  Unstaged  ->  Staged  ->  Branch vs main  ->  Commit Range"),
        Line::from(""),
        Line::from("General"),
        Line::from("  j / k         Scroll this help overlay"),
        Line::from("  ?             Open / close this help overlay"),
        Line::from("  q / Esc       Quit (confirms if unsaved comments exist)"),
    ])
}
