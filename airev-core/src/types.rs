/// A review session tied to a specific repository path.
#[derive(Debug, Clone)]
pub struct Session {
    pub id: i64,
    pub repo_path: String,
    pub created_at: i64,      // Unix timestamp
    pub last_opened_at: i64,  // Unix timestamp
}

/// A single comment attached to a hunk or line within a session.
#[derive(Debug, Clone)]
pub struct Comment {
    pub id: i64,
    pub session_id: i64,
    pub thread_id: i64,
    pub file_path: String,
    pub hunk_id: Option<String>,
    pub line_number: Option<i64>,
    pub comment_type: String,  // question/concern/til/suggestion/praise/nitpick
    pub severity: String,      // critical/major/minor/info
    pub body: String,
    pub created_at: i64,
    pub resolved_at: Option<i64>,
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
