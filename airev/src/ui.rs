//! Placeholder UI for airev Phase 1.
//!
//! This module's sole job in Phase 1 is to prove the draw loop works with a visible
//! 3-panel layout. No real content is rendered — just borders and panel titles.
//! Subsequent phases will replace the placeholder widgets with live diff data.

use ratatui::Frame;
use ratatui::layout::{Constraint, Direction, Layout};
use ratatui::widgets::{Block, Borders};

/// Renders the placeholder 3-panel layout.
///
/// Panels (left to right):
///   - File list (25% width)
///   - Diff view (50% width)
///   - Comments (25% width)
///
/// All panels show borders only — no content until subsequent phases.
pub fn render(frame: &mut Frame) {
    let chunks = Layout::default()
        .direction(Direction::Horizontal)
        .constraints([
            Constraint::Percentage(25),
            Constraint::Percentage(50),
            Constraint::Percentage(25),
        ])
        .split(frame.area());

    frame.render_widget(
        Block::default().title("Files").borders(Borders::ALL),
        chunks[0],
    );
    frame.render_widget(
        Block::default().title("Diff").borders(Borders::ALL),
        chunks[1],
    );
    frame.render_widget(
        Block::default().title("Comments").borders(Borders::ALL),
        chunks[2],
    );
}
