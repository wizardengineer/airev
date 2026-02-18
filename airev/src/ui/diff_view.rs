//! Diff panel renderer for airev.
//!
//! Renders the centre diff panel using a List widget with manual virtual scrolling.
//! Only lines[diff_scroll..diff_scroll+viewport_height] are passed to the List per frame,
//! making rendering O(viewport) not O(total_lines). This enables 5000+ line diffs at 60fps.

use ratatui::{
    Frame,
    text::Line,
    widgets::{List, ListItem},
};

use crate::app::{AppState, PanelFocus};
use crate::theme::Theme;
use crate::ui::layout::{inner_rect, panel_block};

/// Renders the diff centre panel using virtual List scrolling.
///
/// Only the visible window of `state.diff_lines` is materialized into ListItems per frame.
/// If `state.diff_lines` is empty, shows a "No diff loaded" placeholder.
///
/// # Arguments
///
/// * `frame` — current render frame
/// * `area` — the `Rect` for the centre panel (includes borders)
/// * `focus` — current panel focus (determines border style)
/// * `state` — read-only app state supplying `diff_lines`, `diff_scroll`, and `diff_loading`
/// * `theme` — active color theme
pub fn render_diff(
    frame: &mut Frame,
    area: ratatui::layout::Rect,
    focus: PanelFocus,
    state: &AppState,
    theme: &Theme,
) {
    let is_focused = focus == PanelFocus::Diff;
    let block = panel_block("Diff", is_focused, theme);
    let inner = inner_rect(area);
    let viewport_height = inner.height as usize;

    frame.render_widget(block, area);

    if state.diff_lines.is_empty() {
        // Show placeholder when no diff is loaded yet.
        let msg = if state.diff_loading {
            "Computing diff..."
        } else {
            "No diff loaded. Start in a git repository."
        };
        let items = vec![ListItem::new(Line::raw(msg))];
        let list = List::new(items);
        frame.render_widget(list, inner);
        return;
    }

    let total = state.diff_lines.len();
    let visible_start = state.diff_scroll.min(total.saturating_sub(1));
    let visible_end = (visible_start + viewport_height).min(total);

    let items: Vec<ListItem> = state.diff_lines[visible_start..visible_end]
        .iter()
        .map(|l| ListItem::new(l.clone()))
        .collect();

    let list = List::new(items);
    frame.render_widget(list, inner);
}
