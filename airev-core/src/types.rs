/// A review session tied to a specific repository, diff mode, and arguments.
///
/// Sessions are keyed by UUID v4 text. Each unique combination of `repo_path`,
/// `diff_mode`, and `diff_args` produces a separate session on first launch;
/// subsequent launches resume the most-recent matching session.
#[derive(Debug, Clone)]
pub struct Session {
    pub id: String,           // UUID v4 text
    pub repo_path: String,
    pub diff_mode: String,
    pub diff_args: String,
    pub created_at: i64,      // Unix timestamp seconds
    pub updated_at: i64,      // Unix timestamp seconds
}

/// A single comment attached to a hunk or line within a session.
///
/// Comments may optionally belong to a `thread_id` (multi-round review, Phase 7).
/// `comment_type` is one of: question, concern, til, suggestion, praise, nitpick.
/// `severity` is one of: critical, major, minor, info.
#[derive(Debug, Clone)]
pub struct Comment {
    pub id: String,           // UUID v4 text
    pub session_id: String,
    pub file_path: String,
    pub line_number: Option<i64>,
    pub hunk_offset: Option<i64>,
    pub comment_type: String,
    pub severity: String,
    pub body: String,
    pub created_at: i64,
    pub resolved_at: Option<i64>,
    pub thread_id: Option<String>,
}

/// Per-file reviewed state within a session.
///
/// Toggled by the user via the `r` keybinding in the file list panel.
/// `reviewed_at` is set when `reviewed` transitions to `true`, cleared on untoggle.
#[derive(Debug, Clone)]
pub struct FileReviewState {
    pub session_id: String,
    pub file_path: String,
    pub reviewed: bool,
    pub reviewed_at: Option<i64>,
}

/// A diff hunk with metadata for display and persistence.
#[derive(Debug, Clone)]
pub struct Hunk {
    pub id: String,           // content-addressed hash of file+range
    pub file_path: String,
    pub old_start: u32,
    pub old_lines: u32,
    pub new_start: u32,
    pub new_lines: u32,
    pub header: String,
    pub body: String,         // raw unified diff text for this hunk
}

/// A single line within a diff hunk with change type.
#[derive(Debug, Clone)]
pub struct DiffLine {
    pub kind: DiffLineKind,
    pub content: String,
    pub old_lineno: Option<u32>,
    pub new_lineno: Option<u32>,
}

/// The type of change for a diff line.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum DiffLineKind {
    Added,
    Removed,
    Context,
}
