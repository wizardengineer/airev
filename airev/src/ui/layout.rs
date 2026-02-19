//! Responsive 3-panel layout engine for airev.
//!
//! This module is pure layout arithmetic — no mutable application state lives here.
//! It is called inside `terminal.draw()` on every render so every frame gets a fresh
//! layout that automatically reflects the current terminal size.
//!
//! # Panel geometry
//!
//! Three breakpoints govern the horizontal layout:
//!
//! | Terminal width | Panels visible | Notes |
//! |----------------|----------------|-------|
//! | `>= 120` cols  | File list + Diff + Comments | Full 3-panel; widths from `left_pct/center_pct/right_pct` |
//! | `80..=119` cols | File list + Diff | Comments panel hidden; file list gets 25%, diff fills the rest |
//! | `< 80` cols    | Diff only | Both side panels collapsed to width 0 |
//!
//! `Spacing::Overlap(1)` combined with `Block::merge_borders(MergeStrategy::Fuzzy)`
//! makes adjacent panel borders share a single column and merge their corner/junction
//! Unicode box-drawing characters automatically. Overlap is NOT used on the 80-119
//! layout because `Length(0)` panels with overlap cause u16 underflow in ratatui's
//! layout engine.

use ratatui::{
    Frame,
    layout::{Constraint, Layout, Margin, Rect, Spacing},
    style::{Color, Modifier, Style},
    symbols::merge::MergeStrategy,
    text::{Line, Span},
    widgets::{Block, BorderType, Paragraph},
};

use crate::app::{AppState, Mode};
use crate::git::types::DiffMode;
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
/// | `>= 120` cols  | 3-panel split using `state.left_pct / center_pct / right_pct` |
/// | `80..=119` cols | File list (25%) + Diff (fill); comments panel hidden |
/// | `< 80` cols    | Diff only; both side panels collapsed |
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

    // Horizontal split: 3-breakpoint responsive layout.
    let horizontal = if term_width >= 120 {
        // Full 3-panel layout with border overlap for clean junctions.
        Layout::horizontal([
            Constraint::Percentage(state.left_pct),
            Constraint::Percentage(state.center_pct),
            Constraint::Percentage(state.right_pct),
        ])
        .spacing(Spacing::Overlap(1))
    } else if term_width >= 80 {
        // Medium width: show file list (25%) + diff, hide comments.
        // Do NOT use Spacing::Overlap here — Length(0) right panel + Overlap(1) causes
        // u16 underflow in ratatui's layout engine (0 - 1 wraps to u16::MAX).
        Layout::horizontal([
            Constraint::Percentage(25),
            Constraint::Fill(1),
            Constraint::Length(0),
        ])
    } else {
        // Narrow: diff-only layout. No overlap (same underflow risk applies).
        Layout::horizontal([
            Constraint::Length(0),
            Constraint::Fill(1),
            Constraint::Length(0),
        ])
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
/// Shows a mode indicator (`NORMAL` or `INSERT`), the active diff mode label
/// (`UNSTAGED`, `STAGED`, `BRANCH`, or `RANGE`), a file count (e.g. `12 files`)
/// when files are loaded, and a `Computing diff...` loading indicator when
/// `state.diff_loading` is true.
///
/// `HelpOverlay` and `ConfirmQuit` both display `NORMAL` because the underlying
/// mode is `Normal` — the overlay is a transient visual layer, not a mode change.
///
/// # Arguments
///
/// * `frame` — current render frame
/// * `area` — the 1-row `Rect` returned by `compute_layout` (index 3)
/// * `state` — read-only app state supplying the current mode, diff mode, loading flag,
///   and file summaries
/// * `theme` — active color theme (supplies status bar and mode indicator colors)
pub fn render_status_bar(frame: &mut Frame, area: Rect, state: &AppState, theme: &Theme) {
    let (mode_text, mode_fg) = match state.mode {
        Mode::Insert => (" INSERT ", theme.status_mode_insert),
        Mode::Normal | Mode::ConfirmQuit | Mode::HelpOverlay => {
            (" NORMAL ", theme.status_mode_normal)
        }
    };

    let diff_mode_label = match state.diff_mode {
        DiffMode::Unstaged => "UNSTAGED",
        DiffMode::Staged => "STAGED",
        DiffMode::BranchComparison => "BRANCH",
        DiffMode::CommitRange => "RANGE",
    };

    let mut spans = vec![
        Span::styled(mode_text, Style::default().fg(mode_fg).add_modifier(Modifier::BOLD)),
        Span::raw("  |  "),
        Span::styled(diff_mode_label, Style::default().fg(Color::DarkGray)),
    ];

    if !state.file_summaries.is_empty() {
        spans.push(Span::raw("  |  "));
        spans.push(Span::styled(
            format!("{} files", state.file_summaries.len()),
            Style::default().fg(Color::DarkGray),
        ));
    }

    if state.diff_loading {
        spans.push(Span::raw("  |  "));
        spans.push(Span::styled("Computing diff...", Style::default().fg(Color::Yellow)));
    }

    frame.render_widget(
        Paragraph::new(Line::from(spans))
            .style(Style::default().bg(theme.status_bar_bg).fg(theme.status_bar_fg)),
        area,
    );
}
