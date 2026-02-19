//! File list panel renderer for airev.
//!
//! Renders the left file-list panel from AppState.file_summaries. Each entry shows
//! a status badge (M/A/D/R), filename, and +N/-N change counts. When file_summaries
//! is empty, shows a "No files" placeholder matching the diff loading state.

use ratatui::{
    Frame,
    style::{Color, Style},
    text::{Line, Span},
    widgets::{List, ListItem},
};

use crate::app::{AppState, PanelFocus};
use crate::git::types::FileSummary;
use crate::theme::Theme;
use crate::ui::layout::panel_block;

/// Renders the file-list left panel from `AppState.file_summaries`.
///
/// Uses `render_stateful_widget` so the ListState selection highlight is applied.
/// File count is shown in the panel title block (e.g., "Files (12)").
///
/// # Arguments
///
/// * `frame` — current render frame
/// * `area` — the `Rect` for the left panel (includes borders)
/// * `focus` — current panel focus (determines border style)
/// * `state` — mutable app state providing `file_summaries`, `diff_loading`, and `file_list_state`
/// * `theme` — active color theme
pub fn render_file_list(
    frame: &mut Frame,
    area: ratatui::layout::Rect,
    focus: PanelFocus,
    state: &mut AppState,
    theme: &Theme,
) {
    let is_focused = focus == PanelFocus::FileList;
    let file_count = state.file_summaries.len();
    let title = if file_count > 0 {
        format!("Files ({})", file_count)
    } else {
        "Files".to_owned()
    };
    let block = panel_block(&title, is_focused, theme);

    let items: Vec<ListItem> = if state.file_summaries.is_empty() {
        let msg = if state.diff_loading { "Loading..." } else { "No files" };
        vec![ListItem::new(Line::raw(msg))]
    } else {
        state.file_summaries.iter().map(|f| {
            let reviewed = state.file_review_states.get(&f.path).copied().unwrap_or(false);
            file_summary_item(f, reviewed, theme)
        }).collect()
    };

    let list = List::new(items)
        .block(block)
        .highlight_style(Style::default().fg(theme.border_active));

    frame.render_stateful_widget(list, area, &mut state.file_list_state);
}

/// Converts a FileSummary into a styled ListItem.
///
/// Format: `[x] [M] src/main.rs  +42 -7` when reviewed, `[ ] [M] src/...` when not.
/// Badge colors: M=Yellow, A=Green, D=Red, R=Cyan.
/// Review mark colors: reviewed=Green, unreviewed=DarkGray.
fn file_summary_item(f: &FileSummary, reviewed: bool, _theme: &Theme) -> ListItem<'static> {
    let review_mark = if reviewed {
        Span::styled("[x] ", Style::default().fg(Color::Green))
    } else {
        Span::styled("[ ] ", Style::default().fg(Color::DarkGray))
    };
    let badge_color = match f.status {
        'A' => Color::Green,
        'D' => Color::Red,
        'R' => Color::Cyan,
        _ => Color::Yellow, // 'M' and anything else
    };
    let badge = Span::styled(
        format!("[{}] ", f.status),
        Style::default().fg(badge_color),
    );
    // Truncate long paths to avoid horizontal overflow.
    let max_path_len = 28usize;
    let path_display = if f.path.len() > max_path_len {
        format!("...{}", &f.path[f.path.len() - (max_path_len - 3)..])
    } else {
        f.path.clone()
    };
    let path_span = Span::raw(path_display);
    let counts = if f.added > 0 || f.removed > 0 {
        Span::styled(
            format!("  +{} -{}", f.added, f.removed),
            Style::default().fg(Color::DarkGray),
        )
    } else {
        Span::raw("")
    };
    ListItem::new(Line::from(vec![review_mark, badge, path_span, counts]))
}
