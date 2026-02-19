//! Owned data types for the git background thread.
//!
//! All types in this module are fully owned (no borrowed lifetimes) and
//! implement `Send` so they can be transferred from the background thread
//! that owns the `git2::Repository` to the main UI thread.
//!
//! The design intentionally avoids any `&'a` lifetime parameters so that
//! data can be stored in `AppState` without arena allocation.

/// A single line of diff output, fully owned and `Send`-safe.
///
/// Origin characters match `git2::DiffLine::origin()` conventions:
/// `'+'` added, `'-'` removed, `' '` context, `'H'` hunk header, `'F'` file header.
#[derive(Debug, Clone)]
pub struct OwnedDiffLine {
    /// Origin character: `'+'` added, `'-'` removed, `' '` context,
    /// `'H'` hunk header, `'F'` file header.
    pub origin: char,
    /// Full line content including trailing newline (owned — safe to send).
    pub content: String,
    /// Line number in the old (pre-patch) file, if applicable.
    pub old_lineno: Option<u32>,
    /// Line number in the new (post-patch) file, if applicable.
    pub new_lineno: Option<u32>,
}

/// One `@@` hunk block from a diff, fully owned and `Send`-safe.
///
/// Holds the hunk header string and all lines belonging to the hunk,
/// along with the starting line numbers for both old and new files.
#[derive(Debug, Clone)]
pub struct OwnedDiffHunk {
    /// The raw `@@ -old_start,old_lines +new_start,new_lines @@` header string.
    pub header: String,
    /// Starting line number in the old file.
    pub old_start: u32,
    /// Starting line number in the new file.
    pub new_start: u32,
    /// All lines belonging to this hunk, in order.
    pub lines: Vec<OwnedDiffLine>,
}

/// Per-file statistics for the file-list panel.
///
/// Aggregates the status character and line-count deltas for a single
/// changed file so the panel can render them without re-traversing diffs.
#[derive(Debug, Clone)]
pub struct FileSummary {
    /// Repository-relative path to the file.
    pub path: String,
    /// Status character: `'M'` modified, `'A'` added, `'D'` deleted, `'R'` renamed.
    pub status: char,
    /// Number of lines added in this file.
    pub added: usize,
    /// Number of lines removed from this file.
    pub removed: usize,
}

/// The four diff modes supported by airev.
///
/// Controls which git comparison the background thread performs when
/// loading diff data. The default is `Unstaged` (working directory vs index).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum DiffMode {
    /// Working directory vs index (`git diff`).
    #[default]
    Unstaged,
    /// Index vs HEAD (`git diff --cached`).
    Staged,
    /// Arbitrary commit range (`git diff A..B`).
    CommitRange,
    /// Branch comparison (`git diff main..HEAD`).
    BranchComparison,
}

/// Commands sent from the main thread to the git background worker thread.
///
/// Sent over a `crossbeam_channel::Sender<GitRequest>` owned by the main thread.
/// The worker thread receives these and performs the corresponding git operation.
#[derive(Debug)]
pub enum GitRequest {
    /// Load diff for a simple mode (Unstaged, Staged, or BranchComparison).
    LoadDiff(DiffMode),
    /// Load diff for an explicit commit range with `from` and `to` refs.
    LoadDiffRange {
        /// The starting ref (older commit or branch tip).
        from: String,
        /// The ending ref (newer commit or branch tip).
        to: String,
    },
}

/// Result payload sent from the git background thread back to the main thread.
///
/// Carried inside `AppEvent::GitResult(Box<GitResultPayload>)`. Using `Box`
/// keeps the enum variant small on the channel, since `GitResultPayload` can
/// be large (full highlighted line buffer).
///
/// `highlighted_lines` uses `'static` lifetime so lines can be stored directly
/// in `AppState` without arena allocation or re-rendering on each frame.
#[derive(Debug)]
#[allow(dead_code)]
pub struct GitResultPayload {
    /// The diff mode that was requested.
    pub mode: DiffMode,
    /// All diff hunks from the comparison, in file order.
    pub hunks: Vec<OwnedDiffHunk>,
    /// Per-file statistics for the file-list panel.
    pub files: Vec<FileSummary>,
    /// Pre-highlighted lines for the diff panel, computed in the background thread.
    ///
    /// `'static` lifetime is achieved by building `ratatui::text::Span` values
    /// with owned `String` content via `Cow::Owned` — no borrowed string slices.
    pub highlighted_lines: Vec<ratatui::text::Line<'static>>,
    /// Indices into `highlighted_lines` where hunk header lines appear.
    ///
    /// Used by `[` / `]` keybindings to jump between hunks.
    pub hunk_offsets: Vec<usize>,
    /// Starting line index in `highlighted_lines` for each file.
    ///
    /// `file_line_offsets[i]` is the line index where file `i`'s first hunk header
    /// appears in `highlighted_lines`. Used by `jump_to_selected_file()` to scroll
    /// the diff panel to the correct position.
    pub file_line_offsets: Vec<usize>,
}
