//! Central application state for airev.
//!
//! This module owns all mutable UI state: the current mode, which panel has focus,
//! per-panel scroll offsets and viewport heights, panel width percentages, and the
//! unsaved-comment guard flag. No ratatui rendering logic lives here — `app.rs` is
//! pure state that is read by the render module and mutated by the keybinding dispatcher.

use ratatui::widgets::ListState;

use crate::git::types::{DiffMode, FileSummary};

/// Editor mode controlling which keybinding set is active.
///
/// The default mode is `Normal`. Transitions are driven by the keybinding dispatcher.
#[derive(Debug, Default, Clone, Copy, PartialEq, Eq)]
pub enum Mode {
    /// Normal vim-style navigation mode (default).
    #[default]
    Normal,
    /// Text insertion mode for comment editing.
    Insert,
    /// Full-screen help overlay is shown above all panels.
    HelpOverlay,
    /// Quit-confirmation dialog shown when unsaved comments exist.
    ConfirmQuit,
}

/// Which panel currently has keyboard focus.
///
/// The default focus is `FileList`. Navigation cycles through FileList → Diff →
/// Comments → FileList via `next()` and in reverse via `prev()`.
#[derive(Debug, Default, Clone, Copy, PartialEq, Eq)]
pub enum PanelFocus {
    /// Left panel showing the list of changed files.
    #[default]
    FileList,
    /// Centre panel showing the unified diff for the selected file.
    Diff,
    /// Right panel showing threaded review comments.
    Comments,
}

impl PanelFocus {
    /// Returns the panel that precedes `self` in the cycle (wraps around).
    ///
    /// Cycle order: `FileList` → `Comments` → `Diff` → `FileList` (reversed).
    pub fn prev(self) -> Self {
        match self {
            PanelFocus::FileList => PanelFocus::Comments,
            PanelFocus::Diff => PanelFocus::FileList,
            PanelFocus::Comments => PanelFocus::Diff,
        }
    }

    /// Returns the panel that follows `self` in the cycle (wraps around).
    ///
    /// Cycle order: `FileList` → `Diff` → `Comments` → `FileList`.
    pub fn next(self) -> Self {
        match self {
            PanelFocus::FileList => PanelFocus::Diff,
            PanelFocus::Diff => PanelFocus::Comments,
            PanelFocus::Comments => PanelFocus::FileList,
        }
    }
}

/// All mutable UI state passed through every render cycle.
///
/// Scroll state, focus, mode, and panel geometry are bundled here so the render
/// function receives a single immutable reference and the keybinding dispatcher
/// receives a single mutable reference. No logic resides in the render path.
pub struct AppState {
    /// Current editor mode governing which keybindings are active.
    pub mode: Mode,
    /// Which panel currently receives keyboard scroll/navigation events.
    pub focus: PanelFocus,

    /// Stateful list widget backing the file-list panel (left).
    pub file_list_state: ListState,

    /// Vertical scroll offset for the diff panel (centre panel).
    /// usize supports >65535 line diffs; clamped by the renderer to visible range.
    pub diff_scroll: usize,
    /// Vertical scroll offset for the comments `Paragraph` widget (right panel).
    pub comments_scroll: u16,

    /// Inner height of the diff panel after borders, cached after each render.
    /// Used by half-page and full-page scroll calculations.
    pub diff_viewport_height: u16,
    /// Inner height of the comments panel after borders, cached after each render.
    pub comments_viewport_height: u16,
    /// Inner height of the file-list panel after borders, cached after each render.
    pub file_list_viewport_height: u16,

    /// Width percentage allocated to the left (file-list) panel. Default: 20.
    pub left_pct: u16,
    /// Width percentage allocated to the centre (diff) panel. Default: 55.
    pub center_pct: u16,
    /// Width percentage allocated to the right (comments) panel. Default: 25.
    pub right_pct: u16,

    /// Set to `true` when the user has typed a comment that has not been saved.
    /// Guards the quit path — if `true`, a confirmation dialog is shown first.
    pub has_unsaved_comments: bool,

    // Phase 3: Git Layer fields

    /// Pre-highlighted diff lines from the git background thread (List widget source).
    pub diff_lines: Vec<ratatui::text::Line<'static>>,
    /// File summaries from the most recent git diff (for the file-list panel).
    pub file_summaries: Vec<FileSummary>,
    /// Currently active diff mode (Unstaged by default).
    pub diff_mode: DiffMode,
    /// True while the background thread is computing a diff (shows spinner in status bar).
    pub diff_loading: bool,
    /// Line indices of @@ hunk header lines within diff_lines (for [/] hunk navigation).
    pub hunk_offsets: Vec<usize>,
    /// Index of the currently selected file in file_summaries (for file-list → diff jump).
    pub selected_file_index: usize,
    /// Current hunk offset cursor index (index into hunk_offsets, not line number).
    pub hunk_cursor: usize,
}

impl Default for AppState {
    /// Constructs `AppState` with sensible defaults.
    ///
    /// Panel percentages are 20 / 55 / 25 (left / centre / right). All scroll
    /// offsets start at zero. No file is initially selected in the file list.
    fn default() -> Self {
        Self {
            mode: Mode::default(),
            focus: PanelFocus::default(),
            file_list_state: ListState::default(),
            diff_scroll: 0,
            comments_scroll: 0,
            diff_viewport_height: 0,
            comments_viewport_height: 0,
            file_list_viewport_height: 0,
            left_pct: 20,
            center_pct: 55,
            right_pct: 25,
            has_unsaved_comments: false,
            diff_lines: Vec::new(),
            file_summaries: Vec::new(),
            diff_mode: DiffMode::default(),
            diff_loading: false,
            hunk_offsets: Vec::new(),
            selected_file_index: 0,
            hunk_cursor: 0,
        }
    }
}

impl AppState {
    /// Scrolls the focused panel down by `lines` rows.
    ///
    /// For `FileList`: advances the `ListState` selection by `lines` items.
    /// For `Diff`: adds `lines` to the usize scroll offset (saturating).
    /// For `Comments`: adds `lines` to the u16 scroll offset (saturating).
    pub fn scroll_down(&mut self, lines: u16) {
        match self.focus {
            PanelFocus::FileList => {
                self.file_list_state.scroll_down_by(lines);
            }
            PanelFocus::Diff => {
                self.diff_scroll = self.diff_scroll.saturating_add(lines as usize);
            }
            PanelFocus::Comments => {
                self.comments_scroll = self.comments_scroll.saturating_add(lines);
            }
        }
    }

    /// Scrolls the focused panel up by `lines` rows.
    ///
    /// For `FileList`: moves the `ListState` selection up by `lines` items.
    /// For `Diff`: subtracts `lines` from the usize scroll offset (saturating).
    /// For `Comments`: subtracts `lines` from the u16 scroll offset (saturating).
    pub fn scroll_up(&mut self, lines: u16) {
        match self.focus {
            PanelFocus::FileList => {
                self.file_list_state.scroll_up_by(lines);
            }
            PanelFocus::Diff => {
                self.diff_scroll = self.diff_scroll.saturating_sub(lines as usize);
            }
            PanelFocus::Comments => {
                self.comments_scroll = self.comments_scroll.saturating_sub(lines);
            }
        }
    }

    /// Scrolls the focused panel to the very top.
    ///
    /// For `FileList`: selects the first item. For `Diff` / `Comments`: resets offset to 0.
    pub fn scroll_top(&mut self) {
        match self.focus {
            PanelFocus::FileList => {
                self.file_list_state.select_first();
            }
            PanelFocus::Diff => {
                self.diff_scroll = 0;
            }
            PanelFocus::Comments => {
                self.comments_scroll = 0;
            }
        }
    }

    /// Scrolls the focused panel to the very bottom.
    ///
    /// For `FileList`: selects the last item. For `Diff`: sets to last line index
    /// (clamped by renderer). For `Comments`: sets offset to u16::MAX (ratatui clamps).
    pub fn scroll_bottom(&mut self) {
        match self.focus {
            PanelFocus::FileList => {
                self.file_list_state.select_last();
            }
            PanelFocus::Diff => {
                self.diff_scroll = self.diff_lines.len().saturating_sub(1);
            }
            PanelFocus::Comments => {
                self.comments_scroll = u16::MAX;
            }
        }
    }

    /// Scrolls the focused panel down by half its visible height.
    ///
    /// Uses the viewport height cached from the previous render. If the cached height
    /// is zero (first frame), scrolls by 1 to avoid a no-op.
    pub fn half_page_down(&mut self) {
        let half = match self.focus {
            PanelFocus::FileList => self.file_list_viewport_height / 2,
            PanelFocus::Diff => self.diff_viewport_height / 2,
            PanelFocus::Comments => self.comments_viewport_height / 2,
        };
        self.scroll_down(half.max(1));
    }

    /// Scrolls the focused panel up by half its visible height.
    ///
    /// Uses the viewport height cached from the previous render.
    pub fn half_page_up(&mut self) {
        let half = match self.focus {
            PanelFocus::FileList => self.file_list_viewport_height / 2,
            PanelFocus::Diff => self.diff_viewport_height / 2,
            PanelFocus::Comments => self.comments_viewport_height / 2,
        };
        self.scroll_up(half.max(1));
    }

    /// Scrolls the focused panel down by its full visible height (one page).
    ///
    /// Uses the viewport height cached from the previous render.
    pub fn full_page_down(&mut self) {
        let full = match self.focus {
            PanelFocus::FileList => self.file_list_viewport_height,
            PanelFocus::Diff => self.diff_viewport_height,
            PanelFocus::Comments => self.comments_viewport_height,
        };
        self.scroll_down(full.max(1));
    }

    /// Scrolls the focused panel up by its full visible height (one page).
    ///
    /// Uses the viewport height cached from the previous render.
    pub fn full_page_up(&mut self) {
        let full = match self.focus {
            PanelFocus::FileList => self.file_list_viewport_height,
            PanelFocus::Diff => self.diff_viewport_height,
            PanelFocus::Comments => self.comments_viewport_height,
        };
        self.scroll_up(full.max(1));
    }

    /// Moves the file-list selection to the previous file (up one row).
    ///
    /// Equivalent to pressing `k` while focused on the file list, regardless
    /// of which panel actually has focus.
    pub fn prev_file(&mut self) {
        self.file_list_state.scroll_up_by(1);
    }

    /// Moves the file-list selection to the next file (down one row).
    ///
    /// Equivalent to pressing `j` while focused on the file list, regardless
    /// of which panel actually has focus.
    pub fn next_file(&mut self) {
        self.file_list_state.scroll_down_by(1);
    }

    /// Jumps diff_scroll to the previous hunk header ([ keybinding).
    ///
    /// Decrements hunk_cursor. If already at the first hunk, stays there.
    pub fn prev_hunk(&mut self) {
        if self.hunk_offsets.is_empty() {
            return;
        }
        self.hunk_cursor = self.hunk_cursor.saturating_sub(1);
        self.diff_scroll = self.hunk_offsets[self.hunk_cursor];
    }

    /// Jumps diff_scroll to the next hunk header (] keybinding).
    ///
    /// Advances hunk_cursor. If already at the last hunk, stays there.
    pub fn next_hunk(&mut self) {
        if self.hunk_offsets.is_empty() {
            return;
        }
        self.hunk_cursor = (self.hunk_cursor + 1).min(self.hunk_offsets.len() - 1);
        self.diff_scroll = self.hunk_offsets[self.hunk_cursor];
    }

    /// Applies the received GitResultPayload to AppState.
    ///
    /// Called from the AppEvent::GitResult arm in main.rs. Replaces diff content,
    /// clears the loading flag, and resets scroll to top on mode change.
    pub fn apply_git_result(&mut self, payload: crate::git::types::GitResultPayload) {
        let mode_changed = self.diff_mode != payload.mode;
        self.diff_mode = payload.mode;
        self.file_summaries = payload.files;
        self.diff_lines = payload.highlighted_lines;
        self.hunk_offsets = payload.hunk_offsets;
        self.diff_loading = false;
        if mode_changed {
            self.diff_scroll = 0;
            self.hunk_cursor = 0;
        }
    }

    /// Jumps diff view to the selected file (Enter or l on file list).
    ///
    /// Updates selected_file_index and resets diff_scroll to 0.
    pub fn jump_to_selected_file(&mut self) {
        if let Some(idx) = self.file_list_state.selected() {
            self.selected_file_index = idx;
            self.diff_scroll = 0;
            self.hunk_cursor = 0;
            self.focus = PanelFocus::Diff;
        }
    }

    /// Shrinks the diff (centre) panel by transferring 5% to each side panel.
    ///
    /// The centre panel will not shrink below 20%. The 5% is split evenly: 2% to
    /// the left panel and 3% to the right panel (or nearest even split).
    pub fn shrink_diff_panel(&mut self) {
        const MIN_CENTER: u16 = 20;
        const STEP: u16 = 5;
        if self.center_pct <= MIN_CENTER {
            return;
        }
        let transfer = STEP.min(self.center_pct - MIN_CENTER);
        self.center_pct -= transfer;
        // Distribute transfer: half to left, half to right (round down to left).
        let left_gain = transfer / 2;
        let right_gain = transfer - left_gain;
        self.left_pct = self.left_pct.saturating_add(left_gain);
        self.right_pct = self.right_pct.saturating_add(right_gain);
    }

    /// Grows the diff (centre) panel by pulling 5% from each side panel equally.
    ///
    /// The centre panel will not grow above 80%. Side panels each give 2–3% until
    /// either hits its minimum or the centre reaches its maximum.
    pub fn grow_diff_panel(&mut self) {
        const MAX_CENTER: u16 = 80;
        const MIN_SIDE: u16 = 5;
        const STEP: u16 = 5;
        if self.center_pct >= MAX_CENTER {
            return;
        }
        let room = MAX_CENTER - self.center_pct;
        let transfer = STEP.min(room);
        let left_give = (transfer / 2).min(self.left_pct.saturating_sub(MIN_SIDE));
        let right_give = (transfer - transfer / 2).min(self.right_pct.saturating_sub(MIN_SIDE));
        let actual = left_give + right_give;
        self.left_pct -= left_give;
        self.right_pct -= right_give;
        self.center_pct += actual;
    }
}
