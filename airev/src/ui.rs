//! Placeholder UI for airev Phase 1.
//!
//! This module's sole job in Phase 1 is to prove the draw loop works with a visible
//! 3-panel layout. No real content is rendered — just borders and panel titles,
//! styled with colors from the active `Theme`.
//!
//! The `theme` parameter is passed through from `main.rs` at every draw call.
//! Phase 2 will switch the focused panel to `theme.border_active`; all panels
//! use `border_inactive` here because there is no focus model yet.
//!
//! Subsequent phases will replace the placeholder widgets with live diff data.

use crate::theme::Theme;
use ratatui::Frame;
use ratatui::layout::{Constraint, Direction, Layout};
use ratatui::style::Style;
use ratatui::widgets::{Block, Borders};

/// Renders the placeholder 3-panel layout using theme border colors.
///
/// Panels (left to right):
///   - File list (25% width)
///   - Diff view (50% width)
///   - Comments (25% width)
///
/// All panels show borders only — no content until subsequent phases.
/// Border colors come from `theme.border_inactive` (no focus model in Phase 1).
pub fn render(frame: &mut Frame, theme: &Theme) {
    let chunks = Layout::default()
        .direction(Direction::Horizontal)
        .constraints([
            Constraint::Percentage(25),
            Constraint::Percentage(50),
            Constraint::Percentage(25),
        ])
        .split(frame.area());

    // Phase 2 will switch the focused panel to theme.border_active.
    frame.render_widget(
        Block::default()
            .title("Files")
            .borders(Borders::ALL)
            .border_style(Style::default().fg(theme.border_inactive)),
        chunks[0],
    );
    frame.render_widget(
        Block::default()
            .title("Diff")
            .borders(Borders::ALL)
            .border_style(Style::default().fg(theme.border_inactive)),
        chunks[1],
    );
    frame.render_widget(
        Block::default()
            .title("Comments")
            .borders(Borders::ALL)
            .border_style(Style::default().fg(theme.border_inactive)),
        chunks[2],
    );
}
