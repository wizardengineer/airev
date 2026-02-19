//! Background thread that owns git2::Repository for its lifetime.
//!
//! git2::Repository is !Send — it must be opened inside the thread, not passed in.
//! All communication is via channels: GitRequest in, AppEvent::GitResult out.

use std::sync::LazyLock;

use crossbeam_channel::Receiver;
use git2::{Delta, Diff, DiffOptions, Repository};
use ratatui::style::{Color, Modifier, Style};
use ratatui::text::{Line, Span};
use similar::{ChangeTag, TextDiff};
use syntect::easy::HighlightLines;
use syntect::highlighting::ThemeSet;
use syntect::parsing::SyntaxSet;
use tokio::sync::mpsc::UnboundedSender;

use crate::event::AppEvent;
use crate::git::types::{
    DiffMode, FileSummary, GitRequest, GitResultPayload, OwnedDiffHunk, OwnedDiffLine,
};

static PS: LazyLock<SyntaxSet> = LazyLock::new(SyntaxSet::load_defaults_newlines);
static TS: LazyLock<ThemeSet> = LazyLock::new(ThemeSet::load_defaults);

/// Entry point for the background thread that owns the git Repository.
///
/// Opens the Repository at `path` and loops over incoming `GitRequest` messages
/// until the channel is closed (sender dropped). Results are sent back via `event_tx`
/// as `AppEvent::GitResult`.
pub fn git_worker_loop(
    path: String,
    rx: Receiver<GitRequest>,
    event_tx: UnboundedSender<AppEvent>,
) {
    // Eagerly initialize LazyLock statics to avoid first-request latency.
    let _ = &*PS;
    let _ = &*TS;

    let repo = match Repository::open(&path) {
        Ok(r) => r,
        Err(_) => {
            return;
        }
    };

    for request in rx {
        let payload = handle_request(&repo, request);
        let _ = event_tx.send(AppEvent::GitResult(Box::new(payload)));
    }
}

/// Dispatches a GitRequest to the appropriate git2 operation and returns the payload.
///
/// On git2 errors, returns an empty payload for graceful degradation.
fn handle_request(repo: &Repository, request: GitRequest) -> GitResultPayload {
    let (mode, diff_result) = match request {
        GitRequest::LoadDiff(mode) => (mode, get_diff_for_mode(repo, mode)),
        GitRequest::LoadDiffRange { from, to } => {
            (DiffMode::CommitRange, get_diff_for_range(repo, &from, &to))
        }
    };

    match diff_result {
        Ok(diff) => process_diff(mode, &diff),
        Err(_) => GitResultPayload {
            mode,
            hunks: Vec::new(),
            files: Vec::new(),
            highlighted_lines: Vec::new(),
            hunk_offsets: Vec::new(),
            file_line_offsets: Vec::new(),
        },
    }
}

/// Obtains a git2::Diff for simple diff modes (Unstaged, Staged, BranchComparison).
///
/// Returns git2::Error on any failure (repo missing HEAD, no branch named "main", etc.).
fn get_diff_for_mode(repo: &Repository, mode: DiffMode) -> Result<Diff<'_>, git2::Error> {
    match mode {
        DiffMode::Unstaged => {
            let mut opts = DiffOptions::new();
            repo.diff_index_to_workdir(None, Some(&mut opts))
        }
        DiffMode::Staged => {
            let head_commit = repo.head()?.peel_to_commit()?;
            let head_tree = head_commit.tree()?;
            let mut opts = DiffOptions::new();
            repo.diff_tree_to_index(Some(&head_tree), None, Some(&mut opts))
        }
        DiffMode::BranchComparison => {
            let base_obj = repo.revparse_single("main")?;
            let base_commit = base_obj.peel_to_commit()?;
            let base_tree = base_commit.tree()?;
            let head_commit = repo.head()?.peel_to_commit()?;
            let head_tree = head_commit.tree()?;
            let mut opts = DiffOptions::new();
            repo.diff_tree_to_tree(Some(&base_tree), Some(&head_tree), Some(&mut opts))
        }
        DiffMode::CommitRange => {
            // CommitRange requires explicit refs; LoadDiff(CommitRange) is a no-op.
            Err(git2::Error::from_str("CommitRange requires LoadDiffRange"))
        }
    }
}

/// Resolves two ref strings to trees and diffs them.
///
/// Returns git2::Error if either ref cannot be resolved or tree-walking fails.
fn get_diff_for_range<'a>(
    repo: &'a Repository,
    from: &str,
    to: &str,
) -> Result<Diff<'a>, git2::Error> {
    let old_obj = repo.revparse_single(from)?;
    let old_commit = old_obj.peel_to_commit()?;
    let old_tree = old_commit.tree()?;

    let new_obj = repo.revparse_single(to)?;
    let new_commit = new_obj.peel_to_commit()?;
    let new_tree = new_commit.tree()?;

    let mut opts = DiffOptions::new();
    repo.diff_tree_to_tree(Some(&old_tree), Some(&new_tree), Some(&mut opts))
}

/// Extracts hunks + files from a Diff and builds highlighted lines.
///
/// Orchestrates extract_hunks, extract_files, and highlight_hunks into the final payload.
/// `file_hunk_starts` from `extract_hunks` is mapped through `hunk_offsets` to produce
/// `file_line_offsets` — the line index in `highlighted_lines` where each file begins.
fn process_diff(mode: DiffMode, diff: &Diff<'_>) -> GitResultPayload {
    let (hunks, file_hunk_starts) = extract_hunks(diff);
    let files = extract_files(diff);
    let ext = files.first().map(|f| file_ext(&f.path)).unwrap_or("txt");
    let (highlighted_lines, hunk_offsets) = highlight_hunks(&hunks, ext);

    let file_line_offsets: Vec<usize> = file_hunk_starts
        .iter()
        .map(|&hunk_idx| hunk_offsets.get(hunk_idx).copied().unwrap_or(0))
        .collect();

    GitResultPayload { mode, hunks, files, highlighted_lines, hunk_offsets, file_line_offsets }
}

/// Walks diff hunks and lines, converting to owned types for cross-thread transfer.
///
/// Each git2::DiffHunk and git2::DiffLine is converted to owned types
/// (String, u32, char) inside the foreach callbacks before returning.
/// Uses RefCell to share mutable access between closures on the same thread.
///
/// Returns `(hunks, file_hunk_starts)` where `file_hunk_starts[i]` is the index
/// into the returned hunk vec where file `i`'s first hunk begins. This is used by
/// `process_diff` to map file indices to line offsets via `hunk_offsets`.
fn extract_hunks(diff: &Diff<'_>) -> (Vec<OwnedDiffHunk>, Vec<usize>) {
    use std::cell::RefCell;

    // RefCell allows the two closures to share mutable access to the hunk list
    // without requiring raw pointers or unsafe code. Both closures run on the
    // same thread sequentially (git2's contract), so RefCell's single-writer
    // invariant is always satisfied at runtime.
    let hunks: RefCell<Vec<OwnedDiffHunk>> = RefCell::new(Vec::new());
    let file_hunk_starts: RefCell<Vec<usize>> = RefCell::new(Vec::new());

    let _ = diff.foreach(
        &mut |_delta, _progress| {
            file_hunk_starts.borrow_mut().push(hunks.borrow().len());
            true
        },
        None,
        Some(&mut |_delta, hunk| {
            let header = String::from_utf8_lossy(hunk.header()).into_owned();
            let old_start = hunk.old_start();
            let new_start = hunk.new_start();
            hunks.borrow_mut().push(OwnedDiffHunk {
                header,
                old_start,
                new_start,
                lines: Vec::new(),
            });
            true
        }),
        Some(&mut |_delta, _hunk, line| {
            let origin = line.origin();
            let content = String::from_utf8_lossy(line.content()).into_owned();
            let old_lineno = line.old_lineno();
            let new_lineno = line.new_lineno();
            if let Some(h) = hunks.borrow_mut().last_mut() {
                h.lines.push(OwnedDiffLine { origin, content, old_lineno, new_lineno });
            }
            true
        }),
    );

    (hunks.into_inner(), file_hunk_starts.into_inner())
}

/// Collects per-file status info and real added/removed line counts from diff deltas.
///
/// Uses `diff.foreach()` with all four callbacks so that line-origin characters
/// (`'+'` / `'-'`) are counted per file in a single pass. The `file_cb` callback
/// fires once per delta (file) in order, so `files.last_mut()` in the line callback
/// always refers to the correct current file.
fn extract_files(diff: &Diff<'_>) -> Vec<FileSummary> {
    use std::cell::RefCell;

    let files: RefCell<Vec<FileSummary>> = RefCell::new(Vec::new());

    let _ = diff.foreach(
        &mut |delta, _progress| {
            let path = delta
                .new_file()
                .path()
                .unwrap_or(std::path::Path::new("unknown"))
                .to_string_lossy()
                .into_owned();
            let status = match delta.status() {
                Delta::Added => 'A',
                Delta::Deleted => 'D',
                Delta::Renamed => 'R',
                _ => 'M',
            };
            files.borrow_mut().push(FileSummary { path, status, added: 0, removed: 0 });
            true
        },
        None,
        None,
        Some(&mut |_delta, _hunk, line| {
            let mut files = files.borrow_mut();
            if let Some(f) = files.last_mut() {
                match line.origin() {
                    '+' => f.added += 1,
                    '-' => f.removed += 1,
                    _ => {}
                }
            }
            true
        }),
    );

    files.into_inner()
}

/// Converts a syntect (Style, &str) pair to an owned ratatui Span.
///
/// Rebuilds color and modifier fields from syntect types into ratatui types to
/// avoid the type mismatch between ratatui::style::Style and ratatui::prelude::Style
/// that arises from syntect-tui using a different ratatui crate split.
fn syntect_to_span(style: syntect::highlighting::Style, content: &str) -> Span<'static> {
    use syntect::highlighting::Color as SC;
    let to_color = |c: SC| -> Option<Color> {
        if c.a > 0 { Some(Color::Rgb(c.r, c.g, c.b)) } else { None }
    };
    let mut ratatui_style = Style::default();
    if let Some(fg) = to_color(style.foreground) {
        ratatui_style = ratatui_style.fg(fg);
    }
    if let Some(bg) = to_color(style.background) {
        ratatui_style = ratatui_style.bg(bg);
    }
    if style.font_style.contains(syntect::highlighting::FontStyle::BOLD) {
        ratatui_style = ratatui_style.add_modifier(Modifier::BOLD);
    }
    if style.font_style.contains(syntect::highlighting::FontStyle::ITALIC) {
        ratatui_style = ratatui_style.add_modifier(Modifier::ITALIC);
    }
    if style.font_style.contains(syntect::highlighting::FontStyle::UNDERLINE) {
        ratatui_style = ratatui_style.add_modifier(Modifier::UNDERLINED);
    }
    Span::styled(content.to_owned(), ratatui_style)
}

/// Builds syntect-highlighted spans for a single line of code.
///
/// Returns owned `Vec<Span<'static>>`. Falls back to a plain unstyled span on error.
fn build_syntect_spans(
    code: &str,
    h: &mut HighlightLines,
    ps: &SyntaxSet,
) -> Vec<Span<'static>> {
    let ranges = h.highlight_line(code, ps).unwrap_or_default();
    let spans: Vec<Span<'static>> =
        ranges.into_iter().map(|(style, text)| syntect_to_span(style, text)).collect();
    if spans.is_empty() {
        vec![Span::raw(code.to_owned())]
    } else {
        spans
    }
}

/// Computes word-level diff spans for a removed/added line pair.
///
/// Returns two parallel Vecs of spans: old_line spans and new_line spans.
/// Changed words are rendered bold; unchanged words use the base diff color.
fn word_diff_spans(
    old_line: &str,
    new_line: &str,
) -> (Vec<Span<'static>>, Vec<Span<'static>>) {
    let diff = TextDiff::from_words(old_line, new_line);
    let mut old_spans: Vec<Span<'static>> = Vec::new();
    let mut new_spans: Vec<Span<'static>> = Vec::new();

    for op in diff.ops() {
        for change in diff.iter_inline_changes(op) {
            for (emphasized, value) in change.iter_strings_lossy() {
                let text = value.into_owned();
                match change.tag() {
                    ChangeTag::Delete => {
                        let style = if emphasized {
                            Style::default().fg(Color::Red).add_modifier(Modifier::BOLD)
                        } else {
                            Style::default().fg(Color::Red)
                        };
                        old_spans.push(Span::styled(text, style));
                    }
                    ChangeTag::Insert => {
                        let style = if emphasized {
                            Style::default().fg(Color::Green).add_modifier(Modifier::BOLD)
                        } else {
                            Style::default().fg(Color::Green)
                        };
                        new_spans.push(Span::styled(text, style));
                    }
                    ChangeTag::Equal => {
                        let span =
                            Span::styled(text.clone(), Style::default().fg(Color::DarkGray));
                        old_spans.push(span.clone());
                        new_spans.push(span);
                    }
                }
            }
        }
    }
    (old_spans, new_spans)
}

/// Converts a slice of OwnedDiffHunk into highlighted ratatui Lines.
///
/// Applies syntect syntax highlighting and word-level diff emphasis for
/// consecutive -/+ line pairs. Returns the lines and the hunk-header offsets.
fn highlight_hunks(hunks: &[OwnedDiffHunk], ext: &str) -> (Vec<Line<'static>>, Vec<usize>) {
    let theme = TS.themes.get("base16-ocean.dark").or_else(|| TS.themes.values().next());
    let syntax = PS.find_syntax_by_extension(ext).unwrap_or_else(|| PS.find_syntax_plain_text());

    let mut highlighted_lines: Vec<Line<'static>> = Vec::new();
    let mut hunk_offsets: Vec<usize> = Vec::new();

    for hunk in hunks {
        // Record hunk header position and emit a styled header line.
        hunk_offsets.push(highlighted_lines.len());
        let header_span = Span::styled(hunk.header.trim_end().to_owned(), Style::default().fg(Color::Cyan));
        highlighted_lines.push(Line::from(vec![header_span]));

        // Fresh highlighter per hunk for simplicity (safe, predictable state).
        let mut h = match theme {
            Some(t) => HighlightLines::new(syntax, t),
            None => {
                emit_plain_hunk_lines(&hunk.lines, &mut highlighted_lines);
                continue;
            }
        };

        let mut pending_removed: Option<(String, Vec<Span<'static>>)> = None;

        for dl in &hunk.lines {
            let origin = dl.origin;
            let content = &dl.content;
            let code =
                if content.starts_with(['+', '-', ' ']) { &content[1..] } else { content };
            let code = code.trim_end_matches('\n');
            let base_spans = build_syntect_spans(code, &mut h, &PS);

            match origin {
                '-' => {
                    if let Some((_, spans)) = pending_removed.take() {
                        highlighted_lines.push(Line::from(spans));
                    }
                    let mut s = vec![Span::styled("- ", Style::default().fg(Color::Red))];
                    s.extend(base_spans);
                    pending_removed = Some((code.to_owned(), s));
                }
                '+' => {
                    if let Some((old_code, _)) = pending_removed.take() {
                        let (old_word, new_word) = word_diff_spans(&old_code, code);
                        let mut old_s = vec![Span::styled("- ", Style::default().fg(Color::Red))];
                        old_s.extend(old_word);
                        highlighted_lines.push(Line::from(old_s));
                        let mut new_s =
                            vec![Span::styled("+ ", Style::default().fg(Color::Green))];
                        new_s.extend(new_word);
                        highlighted_lines.push(Line::from(new_s));
                    } else {
                        let mut s =
                            vec![Span::styled("+ ", Style::default().fg(Color::Green))];
                        s.extend(base_spans);
                        highlighted_lines.push(Line::from(s));
                    }
                }
                _ => {
                    if let Some((_, spans)) = pending_removed.take() {
                        highlighted_lines.push(Line::from(spans));
                    }
                    let prefix = format!("{origin} ");
                    let mut s = vec![Span::styled(prefix, Style::default().fg(Color::DarkGray))];
                    s.extend(base_spans);
                    highlighted_lines.push(Line::from(s));
                }
            }
        }
        // Flush any trailing unpaired removed line.
        if let Some((_, spans)) = pending_removed.take() {
            highlighted_lines.push(Line::from(spans));
        }
    }

    (highlighted_lines, hunk_offsets)
}

/// Emits plain (non-syntect) lines for a hunk when no theme is available.
///
/// Fallback path used when ThemeSet contains no themes (unusual but possible).
fn emit_plain_hunk_lines(lines: &[OwnedDiffLine], out: &mut Vec<Line<'static>>) {
    for dl in lines {
        let color = match dl.origin {
            '+' => Color::Green,
            '-' => Color::Red,
            _ => Color::DarkGray,
        };
        let text = format!("{} {}", dl.origin, dl.content.trim_end_matches('\n'));
        out.push(Line::from(vec![Span::styled(text, Style::default().fg(color))]));
    }
}

/// Extracts the file extension from a repository-relative path.
///
/// Returns "txt" if the path has no extension.
fn file_ext(path: &str) -> &str {
    path.rsplit('.').next().unwrap_or("txt")
}
