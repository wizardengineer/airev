//! Central application state for airev.
//!
//! This module owns all mutable UI state: the current mode, which panel has focus,
//! per-panel scroll offsets and viewport heights, panel width percentages, and the
//! unsaved-comment guard flag. No ratatui rendering logic lives here — `app.rs` is
//! pure state that is read by the render module and mutated by the keybinding dispatcher.

use ratatui::widgets::ListState;

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

    /// Vertical scroll offset for the diff `Paragraph` widget (centre panel).
    /// Passed directly as `Paragraph::scroll((diff_scroll, 0))`.
    pub diff_scroll: u16,
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
        }
    }
}

impl AppState {
    /// Scrolls the focused panel down by `lines` rows.
    ///
    /// For `FileList`: advances the `ListState` selection by `lines` items.
    /// For `Diff` / `Comments`: adds `lines` to the `u16` scroll offset (saturating).
    pub fn scroll_down(&mut self, lines: u16) {
        match self.focus {
            PanelFocus::FileList => {
                self.file_list_state.scroll_down_by(lines);
            }
            PanelFocus::Diff => {
                self.diff_scroll = self.diff_scroll.saturating_add(lines);
            }
            PanelFocus::Comments => {
                self.comments_scroll = self.comments_scroll.saturating_add(lines);
            }
        }
    }

    /// Scrolls the focused panel up by `lines` rows.
    ///
    /// For `FileList`: moves the `ListState` selection up by `lines` items.
    /// For `Diff` / `Comments`: subtracts `lines` from the scroll offset (saturating).
    pub fn scroll_up(&mut self, lines: u16) {
        match self.focus {
            PanelFocus::FileList => {
                self.file_list_state.scroll_up_by(lines);
            }
            PanelFocus::Diff => {
                self.diff_scroll = self.diff_scroll.saturating_sub(lines);
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
    /// For `FileList`: selects the last item. For `Diff` / `Comments`: sets offset to
    /// `u16::MAX`; ratatui silently clamps to the last visible line.
    pub fn scroll_bottom(&mut self) {
        match self.focus {
            PanelFocus::FileList => {
                self.file_list_state.select_last();
            }
            PanelFocus::Diff => {
                self.diff_scroll = u16::MAX;
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

    /// Navigates to the previous diff hunk (placeholder — Phase 3 wires real hunks).
    ///
    /// Currently scrolls the diff panel up by 5 lines. Phase 3 replaces this with
    /// an actual hunk-index walk using the parsed diff data.
    pub fn prev_hunk(&mut self) {
        self.diff_scroll = self.diff_scroll.saturating_sub(5);
    }

    /// Navigates to the next diff hunk (placeholder — Phase 3 wires real hunks).
    ///
    /// Currently scrolls the diff panel down by 5 lines. Phase 3 replaces this with
    /// an actual hunk-index walk using the parsed diff data.
    pub fn next_hunk(&mut self) {
        self.diff_scroll = self.diff_scroll.saturating_add(5);
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
