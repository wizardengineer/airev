//! Responsive 3-panel layout engine for airev.
//!
//! This module is pure layout arithmetic — no mutable application state lives here.
//! It is called inside `terminal.draw()` on every render so every frame gets a fresh
//! layout that automatically reflects the current terminal size.
//!
//! # Panel geometry
//!
//! At `>= 120` columns all three panels are visible with widths driven by
//! `AppState.left_pct / center_pct / right_pct` (defaults 20 / 55 / 25).
//! Below 120 columns both side panels collapse and the diff fills the full width.
//! Below 80 columns the same collapse applies (minimum viable single-panel display).
//!
//! `Spacing::Overlap(1)` combined with `Block::merge_borders(MergeStrategy::Fuzzy)`
//! makes adjacent panel borders share a single column and merge their corner/junction
//! Unicode box-drawing characters automatically.

use ratatui::{
    Frame,
    layout::{Constraint, Layout, Margin, Rect, Spacing},
    style::{Modifier, Style},
    symbols::merge::MergeStrategy,
    text::{Line, Span},
    widgets::{Block, BorderType, Paragraph},
};

use crate::app::{AppState, Mode};
use crate::theme::Theme;

/// Returns `[left, center, right, status_bar]` panel `Rect`s for the current frame.
///
/// Called inside `terminal.draw()` on every render. The returned slices are valid only
/// for the current draw closure — never store them across frames.
///
/// # Responsive behaviour
///
/// | Terminal width | Layout |
/// |----------------|--------|
/// | `< 120` cols   | Side panels collapsed; diff fills full width |
/// | `>= 120` cols  | 3-panel split using `state.left_pct / center_pct / right_pct` |
///
/// # Arguments
///
/// * `frame` — current render frame (provides `frame.area()` with live terminal size)
/// * `state` — read-only app state supplying panel width percentages
pub fn compute_layout(frame: &Frame, state: &AppState) -> [Rect; 4] {
    let term_width = frame.area().width;

    // Vertical split: main area (fills remaining height) + 1-row status bar.
    let [main_area, status_bar] =
        frame.area().layout(&Layout::vertical([Constraint::Fill(1), Constraint::Length(1)]));

    // Horizontal split: collapse side panels when terminal is narrow.
    let horizontal = if term_width >= 120 {
        Layout::horizontal([
            Constraint::Percentage(state.left_pct),
            Constraint::Percentage(state.center_pct),
            Constraint::Percentage(state.right_pct),
        ])
        .spacing(Spacing::Overlap(1))
    } else {
        // Both < 80 and 80..119 collapse side panels.
        Layout::horizontal([
            Constraint::Length(0),
            Constraint::Fill(1),
            Constraint::Length(0),
        ])
        .spacing(Spacing::Overlap(1))
    };

    let [left, center, right] = main_area.layout(&horizontal);

    [left, center, right, status_bar]
}

/// Returns the inner `Rect` of a panel after removing the 1-cell border on each side.
///
/// Used to cache viewport heights in `AppState` before panels are rendered, so that
/// half-page and full-page scroll distances are available at keypress time.
///
/// # Arguments
///
/// * `area` — the outer `Rect` of the panel (including borders)
pub fn inner_rect(area: Rect) -> Rect {
    area.inner(Margin { vertical: 1, horizontal: 1 })
}

/// Builds a bordered `Block` for a panel.
///
/// Applies `BorderType::Thick` when the panel is focused (distinct active border) and
/// `BorderType::Plain` otherwise. Uses `MergeStrategy::Fuzzy` for the border-merge
/// strategy because `Exact` produces incorrect junctions when mixing `Thick` and `Plain`
/// borders (see Phase 2 research pitfall 5).
///
/// # Arguments
///
/// * `title` — panel title shown in the top border
/// * `is_focused` — `true` when this panel has keyboard focus
/// * `theme` — active color theme (supplies `border_active` / `border_inactive`)
pub fn panel_block<'a>(title: &'a str, is_focused: bool, theme: &'a Theme) -> Block<'a> {
    let border_style = if is_focused {
        Style::default().fg(theme.border_active)
    } else {
        Style::default().fg(theme.border_inactive)
    };
    let border_type = if is_focused { BorderType::Thick } else { BorderType::Plain };

    Block::bordered()
        .title(title)
        .border_type(border_type)
        .border_style(border_style)
        .merge_borders(MergeStrategy::Fuzzy)
}

/// Renders the 1-row status bar at the bottom of the terminal.
///
/// Always shows a mode indicator (`NORMAL` or `INSERT`). Never renders blank.
/// `HelpOverlay` and `ConfirmQuit` both display `NORMAL` because the underlying
/// mode is `Normal` — the overlay is a transient visual layer, not a mode change.
///
/// # Arguments
///
/// * `frame` — current render frame
/// * `area` — the 1-row `Rect` returned by `compute_layout` (index 3)
/// * `state` — read-only app state supplying the current mode
/// * `theme` — active color theme (supplies status bar and mode indicator colors)
pub fn render_status_bar(frame: &mut Frame, area: Rect, state: &AppState, theme: &Theme) {
    let (mode_text, mode_fg) = match state.mode {
        Mode::Insert => (" INSERT ", theme.status_mode_insert),
        Mode::Normal | Mode::ConfirmQuit | Mode::HelpOverlay => {
            (" NORMAL ", theme.status_mode_normal)
        }
    };

    let mode_span = Span::styled(
        mode_text,
        Style::default().fg(mode_fg).add_modifier(Modifier::BOLD),
    );
    let status_line = Line::from(vec![mode_span]);

    frame.render_widget(
        Paragraph::new(status_line)
            .style(Style::default().bg(theme.status_bar_bg).fg(theme.status_bar_fg)),
        area,
    );
}
