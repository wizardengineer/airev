# Phase 3: Git Layer - Research

**Researched:** 2026-02-18
**Domain:** Rust git integration, async TUI background threads, syntax highlighting, virtual scrolling
**Confidence:** HIGH

## Summary

Phase 3 wires the diff view to real git content. The core challenge is that `git2::Repository`
does not implement `Send`, which means it cannot cross thread boundaries. The canonical solution —
used in production by `gitui`/`asyncgit` — is to keep the `Repository` pinned to a single
`std::thread::spawn` thread for its entire lifetime and communicate through channels. The
background thread receives `GitRequest` commands via a `crossbeam-channel` bounded or unbounded
receiver, performs git operations, serializes results into owned `Vec<DiffHunk>` structs (no
borrowed lifetimes), and sends them back by cloning the tokio `mpsc::UnboundedSender<AppEvent>`
that was captured when the thread was spawned. This pattern is fully verified by the compiler
without `unsafe`.

Syntax highlighting is handled by `syntect 5.3` with `default-features = false, features =
["default-fancy"]` (pure-Rust regex, no Oniguruma C dependency). `syntect-tui 3.0.6` bridges
syntect's `Style`/`FontStyle` types to ratatui's `Span`. Highlighting must happen in the
background thread — not the render closure — to avoid frame drops. The result is stored as
`Vec<ratatui::text::Line<'static>>` in `AppState`.

Virtual scrolling is implemented manually by storing all lines in `AppState` and slicing
`&lines[visible_start..visible_end]` before passing to a `List` widget each frame. This avoids
loading anything into the render closure beyond the visible window, enabling 5000+ line diffs to
scroll at 60fps. The `List` widget (not `Paragraph`) is used for the diff view because individual
items can carry full `Line<'static>` with mixed `Span` styles, and `ListState` tracks the offset
natively.

**Primary recommendation:** Pin `Repository` to one `std::thread::spawn` thread; use
`crossbeam_channel::unbounded` for the request side and the existing
`tokio::sync::mpsc::UnboundedSender<AppEvent>` for results; extract all git data into owned
structs before returning; highlight in that same thread with syntect; slice for virtual scrolling
in the render path.

---

## Standard Stack

### Core

| Library | Version | Purpose | Why Standard |
|---------|---------|---------|--------------|
| `git2` | 0.20.4 | libgit2 bindings — diff, index, worktree | Locked decision; only mature git2 binding in Rust ecosystem |
| `crossbeam-channel` | 0.5.15 | Request channel into the git background thread | `Send`+`Sync` channels; `Sender` is cloneable; both bounded and unbounded |
| `syntect` | 5.3.0 | Syntax highlighting using Sublime Text grammars | Locked decision; `SyntaxSet` is `Send+Sync`, safe to share |
| `syntect-tui` | 3.0.6 | Converts syntect `Style` → ratatui `Span`/`Style` | Locked decision; only bridge crate for this pair |
| `similar` | 2.7.0 | Word-level (inline) diff within changed lines | Locked decision; `inline` feature adds `InlineChange` / `iter_inline_changes` |

### Supporting

| Library | Version | Purpose | When to Use |
|---------|---------|---------|-------------|
| `tokio::sync::mpsc` (already a dep) | 1.49 | Send `AppEvent::GitResult` back to the async event loop | Re-use the existing channel rather than adding a new one |
| `std::thread::spawn` (std) | stable | Owns the `Repository` for its lifetime | Replaces tokio tasks for `!Send` types |

### Alternatives Considered

| Instead of | Could Use | Tradeoff |
|------------|-----------|----------|
| `crossbeam-channel` for requests | `std::sync::mpsc` | crossbeam is faster, supports `select!`, widely used in gitui; std mpsc works but offers no advantage |
| `similar` for word diff | `prettydiff` | prettydiff exposes `diff_words` directly but is less maintained; similar is well-maintained and has `InlineChange` with emphasis tuples |
| manual slice for virtual scroll | `tui-scrollview` | Extra dependency; manual slice is 5 lines of code and already specified in requirements |
| `syntect` default-fancy | `tree-sitter` | tree-sitter is more accurate but requires C build dep and is far more complex to integrate; syntect is sufficient for this use case |

**Installation (Cargo.toml additions):**

```toml
[workspace.dependencies]
git2           = "0.20"
crossbeam-channel = "0.5"
syntect        = { version = "5.3", default-features = false, features = ["default-fancy"] }
syntect-tui    = "3.0"
similar        = { version = "2.7", features = ["inline"] }
```

---

## Architecture Patterns

### Recommended Project Structure

```
airev/src/
├── git/
│   ├── mod.rs          # AsyncGit facade: spawn thread, expose request sender
│   ├── types.rs        # Owned DiffHunk, DiffLine, FileSummary structs
│   └── worker.rs       # Background thread event loop: recv request, run git2, send AppEvent
├── ui/
│   ├── diff_view.rs    # Render the diff panel: slice Vec<Line> for virtual scroll
│   └── file_tree.rs    # Render the file list panel with real git file summaries
├── app.rs              # Extended AppState: diff_lines, file_list, diff_mode, loading flag
└── event.rs            # AppEvent::GitResult(GitResult) variant
```

### Pattern 1: Background Thread Owning Repository

**What:** `std::thread::spawn` captures `Repository` by move. Thread loops on a
`crossbeam_channel::Receiver<GitRequest>`. Results sent back via a cloned
`tokio::sync::mpsc::UnboundedSender<AppEvent>`.

**When to use:** Any time `git2::Repository` is involved — it is `!Send`, so this is the only
safe pattern.

**How the channel bridge works:** The tokio `UnboundedSender` is `Send + Clone`. The git thread
captures a clone of `handler.tx` before spawning. After computing a diff, the thread calls
`tx.send(AppEvent::GitResult(result))` — this is a non-blocking call that deposits onto the tokio
MPSC channel, and the main async loop picks it up on the next `rx.recv().await`.

```rust
// Source: verified pattern from gitui/asyncgit + tokio bridging docs
// git/mod.rs

use crossbeam_channel::{unbounded, Sender};
use tokio::sync::mpsc::UnboundedSender;
use crate::event::AppEvent;
use crate::git::worker::git_worker_loop;
use crate::git::types::GitRequest;

pub struct AsyncGit {
    pub request_tx: Sender<GitRequest>,
}

impl AsyncGit {
    pub fn new(event_tx: UnboundedSender<AppEvent>, repo_path: &str) -> Self {
        let (request_tx, request_rx) = unbounded::<GitRequest>();
        let path = repo_path.to_owned();
        std::thread::spawn(move || {
            git_worker_loop(path, request_rx, event_tx);
        });
        Self { request_tx }
    }
}
```

```rust
// git/worker.rs
use crossbeam_channel::Receiver;
use git2::Repository;
use tokio::sync::mpsc::UnboundedSender;
use crate::event::AppEvent;
use crate::git::types::{GitRequest, GitResult};

pub fn git_worker_loop(
    path: String,
    rx: Receiver<GitRequest>,
    event_tx: UnboundedSender<AppEvent>,
) {
    let repo = Repository::open(&path).expect("failed to open repo");
    for request in rx {   // blocks until next request or channel closes
        let result = handle_request(&repo, request);
        let _ = event_tx.send(AppEvent::GitResult(result));
    }
}
```

### Pattern 2: Owned Diff Data Structs

**What:** `git2::DiffHunk` and `git2::DiffLine` borrow from `git2::Diff` and cannot leave the
thread. Convert to owned structs inside the `foreach` callback, before the callback returns.

**Key insight:** Call `String::from_utf8_lossy(line.content())` inside the `foreach` closure to
extract owned `String` data. Collect hunks into `Vec<OwnedHunk>` which is `Send`.

```rust
// git/types.rs
#[derive(Debug, Clone)]
pub struct OwnedDiffLine {
    pub origin: char,           // '+', '-', ' ', 'H', 'F'
    pub content: String,        // owned — safe to send across threads
    pub old_lineno: Option<u32>,
    pub new_lineno: Option<u32>,
}

#[derive(Debug, Clone)]
pub struct OwnedDiffHunk {
    pub header: String,
    pub old_start: u32,
    pub new_start: u32,
    pub lines: Vec<OwnedDiffLine>,
}

#[derive(Debug, Clone)]
pub struct FileSummary {
    pub path: String,
    pub status: char,     // 'M', 'A', 'D', 'R'
    pub added: usize,
    pub removed: usize,
}
```

```rust
// Inside the worker — collecting owned data from git2 callbacks
let mut hunks: Vec<OwnedDiffHunk> = Vec::new();
diff.foreach(
    &mut |_, _| true,
    None,
    Some(&mut |_, hunk| {
        let header = String::from_utf8_lossy(hunk.header()).into_owned();
        hunks.push(OwnedDiffHunk {
            header,
            old_start: hunk.old_start(),
            new_start: hunk.new_start(),
            lines: Vec::new(),
        });
        true
    }),
    Some(&mut |_, _, line| {
        if let Some(hunk) = hunks.last_mut() {
            hunk.lines.push(OwnedDiffLine {
                origin: line.origin(),
                content: String::from_utf8_lossy(line.content()).into_owned(),
                old_lineno: line.old_lineno(),
                new_lineno: line.new_lineno(),
            });
        }
        true
    }),
).unwrap();
```

### Pattern 3: Four Diff Modes via git2 APIs

**What:** Map each UI diff mode to its git2 `Repository` method.

| Mode | git2 Method | Equivalent git CLI |
|------|------------|-------------------|
| Unstaged workdir | `repo.diff_index_to_workdir(None, None)` | `git diff` |
| Staged index | `repo.diff_tree_to_index(Some(&head_tree), None, None)` | `git diff --cached` |
| Commit range | `repo.diff_tree_to_tree(Some(&old_tree), Some(&new_tree), None)` | `git diff A B` |
| Branch comparison | `repo.diff_tree_to_tree(Some(&base_tree), Some(&head_tree), None)` | `git diff main..HEAD` |

To resolve a branch name to a tree:
```rust
let obj = repo.revparse_single("main").unwrap();
let commit = obj.peel_to_commit().unwrap();
let tree = commit.tree().unwrap();
```

### Pattern 4: Syntax Highlighting in Background Thread

**What:** `SyntaxSet` and `ThemeSet` are both `Send+Sync` (since syntect 3.0). Load them once
as `LazyLock` statics (or build inside the worker thread). Call `HighlightLines::highlight_line`
per line; convert with `syntect_tui::into_span`. Return `Vec<ratatui::text::Line<'static>>`.

```rust
// Source: syntect docs + syntect-tui README
use syntect::easy::HighlightLines;
use syntect::highlighting::ThemeSet;
use syntect::parsing::SyntaxSet;
use syntect::util::LinesWithEndings;
use syntect_tui::into_span;
use ratatui::text::{Line, Span};
use std::sync::LazyLock;

static PS: LazyLock<SyntaxSet> = LazyLock::new(SyntaxSet::load_defaults_newlines);
static TS: LazyLock<ThemeSet> = LazyLock::new(ThemeSet::load_defaults);

fn highlight_diff_line(
    line_content: &str,
    extension: &str,
) -> Line<'static> {
    let syntax = PS.find_syntax_by_extension(extension)
        .unwrap_or_else(|| PS.find_syntax_plain_text());
    let theme = &TS.themes["base16-ocean.dark"];
    let mut h = HighlightLines::new(syntax, theme);
    let spans: Vec<Span<'static>> = h
        .highlight_line(line_content, &PS)
        .unwrap_or_default()
        .into_iter()
        .filter_map(|seg| into_span(seg).ok())
        .collect();
    Line::from(spans)
}
```

**Important:** `HighlightLines` holds incremental state per file — create a new instance per
file. Do NOT reuse across file boundaries.

### Pattern 5: Word-Level Diff via `similar`

**What:** After identifying a removed line and its corresponding added line, run `TextDiff` at
word level and use `iter_inline_changes` to get emphasized/de-emphasized token spans.

```rust
// Source: similar docs - requires features = ["inline"]
use similar::{ChangeTag, TextDiff};

fn word_diff_spans(old_line: &str, new_line: &str) -> (Vec<(bool, String)>, Vec<(bool, String)>) {
    let diff = TextDiff::from_words(old_line, new_line);
    // For each change group that is a Replace, iter_inline_changes yields
    // InlineChange with .iter_strings_lossy() -> (emphasized: bool, value: &str)
    let mut old_spans = Vec::new();
    let mut new_spans = Vec::new();
    for group in diff.grouped_ops(0) {
        for op in &group {
            for change in diff.iter_inline_changes(op) {
                match change.tag() {
                    ChangeTag::Delete => {
                        for (emph, val) in change.iter_strings_lossy() {
                            old_spans.push((emph, val.to_owned()));
                        }
                    }
                    ChangeTag::Insert => {
                        for (emph, val) in change.iter_strings_lossy() {
                            new_spans.push((emph, val.to_owned()));
                        }
                    }
                    ChangeTag::Equal => {}
                }
            }
        }
    }
    (old_spans, new_spans)
}
```

### Pattern 6: Manual Virtual Scrolling

**What:** Store all highlighted lines in `AppState.diff_lines: Vec<Line<'static>>`. Track
`diff_scroll: usize` as a line offset. On each render frame, slice:

```rust
// In render_diff():
let visible = &state.diff_lines[
    state.diff_scroll.min(state.diff_lines.len().saturating_sub(1))
    ..
    (state.diff_scroll + viewport_height as usize).min(state.diff_lines.len())
];
let items: Vec<ListItem> = visible.iter()
    .map(|l| ListItem::new(l.clone()))
    .collect();
let list = List::new(items).block(block);
frame.render_widget(list, inner);
```

**Why `List` not `Paragraph`:** `Paragraph` requires a single `Text` value and applies uniform
scroll; `List` renders individual `ListItem` values each containing a full `Line<'static>` with
mixed `Span` styles. The `List` widget naturally handles mixed-style lines without wrapping
concerns.

**AppState changes needed:**
```rust
// New fields in AppState
pub diff_lines: Vec<ratatui::text::Line<'static>>,  // all highlighted lines
pub diff_scroll: usize,                              // line offset (replaces u16)
pub file_summaries: Vec<FileSummary>,                // from git diff output
pub diff_mode: DiffMode,                             // enum: Unstaged/Staged/Range/Branch
pub diff_loading: bool,                              // true while background thread works
pub hunk_offsets: Vec<usize>,                        // line indices of @@ headers for [/] nav
```

**Note:** `diff_scroll` must change from `u16` to `usize` because 5000+ line diffs exceed `u16::MAX` (65535). The `scroll_down`/`scroll_up` methods in `AppState` must be updated accordingly.

### Anti-Patterns to Avoid

- **Never share `git2::Repository` across threads:** the compiler enforces this; attempting it
  without `unsafe` produces a compile error. Never wrap in `Arc<Mutex<Repository>>` — `Mutex`
  would make it `Sync` but not `Send`, which still doesn't help.
- **Never call `HighlightLines::highlight_line` in the render closure:** render closures run at
  60fps; highlighting is CPU-intensive and would cause frame drops.
- **Never pass `git2::DiffHunk` or `git2::DiffLine` out of a `foreach` callback:** these borrow
  from `Diff` and have lifetimes tied to it. Extract all data inside the callback.
- **Never use `String::from_utf8` without a fallback:** git content may be valid UTF-8 but use
  `from_utf8_lossy` defensively; binary diffs will produce replacement characters which is
  acceptable.
- **Never reuse `HighlightLines` across file boundaries:** the highlighter maintains incremental
  parser state. Create a new instance per file.

---

## Don't Hand-Roll

| Problem | Don't Build | Use Instead | Why |
|---------|-------------|-------------|-----|
| Cross-thread message passing | Custom mutex/arc ring buffer | `crossbeam-channel` 0.5 | Handles backpressure, wakeup, channel lifecycle automatically |
| Syntax highlighting | Custom regex/ANSI coloring | `syntect` 5.3 + `syntect-tui` 3.0 | 1000+ language syntaxes, theme support, battle-tested parser |
| Word-level diff | Character-by-character comparison | `similar` 2.7 with `inline` feature | LCS-based algorithm with InlineChange API; handles edge cases |
| Git diff parsing | Custom git output parser | `git2` 0.20 bindings | Wraps libgit2, handles binary files, encodings, submodules |

**Key insight:** The git2 + crossbeam-channel + tokio MPSC bridge is the exact pattern used in
`gitui`'s `asyncgit` crate — a production TUI with the same constraints. Follow it exactly.

---

## Common Pitfalls

### Pitfall 1: Trying to Move Repository Across Threads

**What goes wrong:** `std::thread::spawn(move || { use_repo(repo) })` fails to compile because
`git2::Repository` is `!Send`.

**Why it happens:** libgit2 uses thread-local state in some code paths; the Rust bindings
conservatively mark it `!Send` rather than implementing `unsafe Send`.

**How to avoid:** Open the `Repository` inside `std::thread::spawn` using a path string:
```rust
std::thread::spawn(move || {
    let repo = Repository::open(&path).unwrap();  // open INSIDE the thread
    // ... use repo forever
});
```

**Warning signs:** Compiler error `E0277: the trait bound git2::Repository: Send is not satisfied`.

### Pitfall 2: Borrowing git2 Data Across the foreach Boundary

**What goes wrong:** Trying to return `DiffHunk<'_>` or `DiffLine<'_>` from a closure, or
storing them in a `Vec` that outlives the `foreach` call.

**Why it happens:** These types have lifetimes tied to the `Diff` struct which is on the stack.

**How to avoid:** Extract all data to owned types (`String`, `u32`, etc.) inside every callback
before it returns `true`. Use the `OwnedDiffHunk`/`OwnedDiffLine` pattern shown above.

**Warning signs:** Lifetime error `lifetime may not live long enough` in foreach callbacks.

### Pitfall 3: Using u16 for Scroll Offset with Large Diffs

**What goes wrong:** `diff_scroll: u16` saturates at 65535. An LLVM-scale diff of 5000+ lines
will be fine but a very large monorepo commit could exceed this.

**Why it happens:** The existing `AppState` uses `u16` for `Paragraph::scroll()` offset. The
diff view switches to a `List` widget, so the offset is now a `usize` slice index.

**How to avoid:** Change `diff_scroll` to `usize` in `AppState`. The existing `Paragraph`-based
comments panel keeps `u16`.

**Warning signs:** Scroll stops at line 65535 on large diffs.

### Pitfall 4: Calling highlight_line with Raw Diff Content Including Prefix Characters

**What goes wrong:** A diff line content like `"+fn main() -> Result<()> {"` includes the `+`
prefix. Passing this to `HighlightLines::highlight_line` makes syntect try to highlight the `+`
as Rust code, producing incorrect coloring.

**Why it happens:** git2's `DiffLine::content()` returns the raw line content including the
leading `+`/`-`/` ` prefix character.

**How to avoid:** Strip the first byte (the origin character) from the content before passing to
syntect:
```rust
let code_content = if content.starts_with(['+', '-', ' ']) {
    &content[1..]
} else {
    &content
};
```
Apply the diff color (add/remove/context) as the base `Style` on the whole `Line`, then overlay
syntect spans on top — or keep diff coloring and syntect coloring as separate layers using the
approach of applying `Style::fg(diff_color)` to the `ListItem` rather than modifying individual
spans.

**Warning signs:** Syntax highlighting looks garbled; `+` and `-` characters appear highlighted
as operators.

### Pitfall 5: syntect LazyLock Initialization in the Wrong Thread

**What goes wrong:** `SyntaxSet::load_defaults_newlines()` takes ~10-100ms on first call. If
called from the render closure (main thread), it causes a frame drop on startup.

**Why it happens:** Lazy statics initialize on first access from any thread.

**How to avoid:** Trigger the `LazyLock` by accessing `*PS` and `*TS` inside the git worker
thread's initialization code, before processing any requests. Alternatively, pass
`Arc<SyntaxSet>` and `Arc<Theme>` into the thread at spawn time.

**Warning signs:** First frame renders slowly (~100ms instead of <1ms).

### Pitfall 6: AppEvent::GitResult Variant Carrying No Data

**What goes wrong:** The existing `AppEvent::GitResult` variant (from Phase 1) carries no
payload. Extending it to carry data requires adding a `Box<GitResult>` or similar, which changes
the enum variant signature.

**Why it happens:** `AppEvent` was declared `#[non_exhaustive]` with `GitResult` as a unit
variant placeholder.

**How to avoid:** Change the variant to `GitResult(Box<GitResultPayload>)` where the payload
holds `Vec<OwnedDiffHunk>`, `Vec<FileSummary>`, etc. Because `AppEvent` is `#[non_exhaustive]`,
existing `_ => {}` match arms in `main.rs` do not need updating — only add an explicit arm for
the new `GitResult(payload)` variant.

**Warning signs:** Compiler error when trying to put data in a unit variant.

### Pitfall 7: diff_tree_to_tree Requires Both Trees — None Means Empty Tree

**What goes wrong:** Passing `None` for `old_tree` means "diff against an empty tree" (like
diffing a first commit). For branch comparison, both trees must be `Some(&tree)`.

**Why it happens:** libgit2 allows `None` as "empty tree" — this is correct for first-commit
diffs but wrong for branch comparison.

**How to avoid:** Always resolve both ref names to trees for branch comparison:
```rust
let base = repo.revparse_single("main")?.peel_to_commit()?.tree()?;
let head = repo.revparse_single("HEAD")?.peel_to_commit()?.tree()?;
let diff = repo.diff_tree_to_tree(Some(&base), Some(&head), None)?;
```

---

## Code Examples

Verified patterns from official sources:

### Four Diff Modes

```rust
// Source: git2 docs.rs + Rust Forum verified examples
// Mode: Unstaged (git diff)
let diff = repo.diff_index_to_workdir(None, None)?;

// Mode: Staged (git diff --cached)
let head = repo.head()?.peel_to_commit()?.tree()?;
let diff = repo.diff_tree_to_index(Some(&head), None, None)?;

// Mode: Commit range (git diff A..B)
let old_tree = repo.revparse_single(old_ref)?.peel_to_commit()?.tree()?;
let new_tree = repo.revparse_single(new_ref)?.peel_to_commit()?.tree()?;
let diff = repo.diff_tree_to_tree(Some(&old_tree), Some(&new_tree), None)?;

// Mode: Branch comparison (git diff main..HEAD)
let base = repo.revparse_single("main")?.peel_to_commit()?.tree()?;
let head = repo.revparse_single("HEAD")?.peel_to_commit()?.tree()?;
let diff = repo.diff_tree_to_tree(Some(&base), Some(&head), None)?;
```

### Syntect + syntect-tui Integration

```rust
// Source: syntect-tui README + lib.rs/crates/syntect-tui
use syntect::easy::HighlightLines;
use syntect::highlighting::ThemeSet;
use syntect::parsing::SyntaxSet;
use syntect_tui::into_span;
use ratatui::text::{Line, Span};

// Load once, share via Arc or LazyLock (both Send+Sync since syntect 3.0)
let ps = SyntaxSet::load_defaults_newlines();
let ts = ThemeSet::load_defaults();
let syntax = ps.find_syntax_by_extension("rs")
    .unwrap_or_else(|| ps.find_syntax_plain_text());
let mut h = HighlightLines::new(syntax, &ts.themes["base16-ocean.dark"]);

// Per line (strip +/-/ prefix first):
let code_line = &raw_content[1..];  // strip origin char
let spans: Vec<Span<'static>> = h
    .highlight_line(code_line, &ps)
    .unwrap_or_default()
    .into_iter()
    .filter_map(|seg| into_span(seg).ok())
    .collect();
let line = Line::from(spans);
```

### Similar Inline Word Diff

```rust
// Source: similar docs.rs - requires features = ["inline"]
use similar::{ChangeTag, TextDiff};

let diff = TextDiff::from_words(old_line, new_line);
for op in diff.ops() {
    for change in diff.iter_inline_changes(op) {
        match change.tag() {
            ChangeTag::Delete | ChangeTag::Insert | ChangeTag::Equal => {
                for (emphasized, value) in change.iter_strings_lossy() {
                    // emphasized = true means this word actually changed
                    // render with highlight bg if emphasized
                }
            }
        }
    }
}
```

### AppEvent::GitResult with Payload

```rust
// event.rs — extend the existing GitResult variant
#[derive(Debug)]
pub struct GitResultPayload {
    pub mode: DiffMode,
    pub hunks: Vec<OwnedDiffHunk>,
    pub files: Vec<FileSummary>,
    pub highlighted_lines: Vec<ratatui::text::Line<'static>>,
}

#[non_exhaustive]
pub enum AppEvent {
    Key(KeyEvent),
    Resize(u16, u16),
    Tick,
    Render,
    FileChanged,
    GitResult(Box<GitResultPayload>),  // changed from unit variant
    DbResult,
    Quit,
}
```

---

## State of the Art

| Old Approach | Current Approach | When Changed | Impact |
|--------------|------------------|--------------|--------|
| `Paragraph::scroll((u16, 0))` for diff | `List` widget with manual slice + `usize` offset | This phase | Enables >65535 line diffs, per-line `Span` styles |
| Unit `AppEvent::GitResult` placeholder | `AppEvent::GitResult(Box<GitResultPayload>)` with data | This phase | Must update variant; `#[non_exhaustive]` protects other match arms |
| Placeholder diff content in `render_diff()` | Real lines from `AppState.diff_lines` | This phase | Remove `build_diff_placeholder()` |
| Placeholder file list in `render_file_list()` | Real `FileSummary` list from `AppState.file_summaries` | This phase | Remove hardcoded items |
| `diff_scroll: u16` | `diff_scroll: usize` | This phase | Required for large diffs |

**Deprecated/outdated:**
- `HighlightLines::highlight()` — deprecated in syntect 5.0, renamed to `highlight_line()`
- `diff_tree_to_workdir` — does NOT account for staged deletes; use
  `diff_tree_to_workdir_with_index` if you want parity with `git diff <treeish>`

---

## Open Questions

1. **Theme selection for syntect**
   - What we know: `ThemeSet::load_defaults()` includes `base16-ocean.dark`, `InspiredGitHub`, `Solarized (dark/light)`, etc.
   - What's unclear: Whether the project's existing `Theme` struct (catppuccin-mocha palette) should be adapted as a syntect `Theme`, or if a separate pre-bundled syntect theme is used alongside the UI theme.
   - Recommendation: Use `InspiredGitHub` or `base16-ocean.dark` from syntect defaults for code highlighting; keep the catppuccin palette for non-code UI chrome. They operate on different elements.

2. **Word-diff matching algorithm (pairing removed/added lines)**
   - What we know: `similar` diffs arbitrary slices; `TextDiff::from_words()` works per pair of lines.
   - What's unclear: How to pair a removed line with its corresponding added line when there are multiple removes and adds in one hunk (the "best match" problem).
   - Recommendation: Use a simple heuristic: match consecutive remove/add pairs within a hunk in order. This matches how `git diff --word-diff` behaves and is sufficient for Phase 3.

3. **Loading indicator implementation**
   - What we know: `diff_loading: bool` in AppState should drive a status bar message "Computing diff...".
   - What's unclear: Whether to use `throbber-widgets-tui` for an animated spinner or a static string.
   - Recommendation: Start with a static "Computing diff..." string; add animated spinner only if the background thread takes visibly longer than one tick (250ms). The git background thread is likely fast enough that the spinner is rarely seen.

4. **Hunk navigation index**
   - What we know: `[` and `]` keybindings should jump between `@@` headers.
   - What's unclear: Whether to store hunk line offsets as a `Vec<usize>` in AppState or compute them on-the-fly from `diff_lines`.
   - Recommendation: Store `hunk_offsets: Vec<usize>` alongside `diff_lines` — computed once in the background thread when assembling highlighted lines, populated with the line index of every `H` (hunk header) origin line.

---

## Sources

### Primary (HIGH confidence)

- `git2` 0.20.4 docs.rs — diff methods, DiffHunk, DiffLine, DiffLine::origin(), Repository::revparse_single
- `crossbeam-channel` 0.5.15 — bounded/unbounded channel APIs, blocking recv
- `syntect` 5.3.0 — HighlightLines, SyntaxSet, ThemeSet, default-fancy feature, Send+Sync since 3.0
- `syntect-tui` 3.0.6 — into_span(), translate_style(), lib.rs code example verified
- `similar` 2.7.0 — TextDiff::from_words(), iter_inline_changes(), InlineChange, iter_strings_lossy()
- tokio docs: "Bridging with sync code" — pattern for UnboundedSender from sync thread
- cargo search output (live) — confirmed all version numbers above

### Secondary (MEDIUM confidence)

- `asyncgit` / gitui codebase architecture — confirms the Repository-per-thread + crossbeam pattern is production-tested
- git2-rs diff.rs example (GitHub) — confirms `foreach` callback pattern for extracting hunk/line data
- syntect-tui lib.rs example — confirms `highlight_line` + `into_span` workflow

### Tertiary (LOW confidence)

- Ratatui issue #1514 / PR #1553 — multi-line List scrolling was being fixed as of early 2025; behavior for single-line-per-item `ListItem` (our use case) is stable

---

## Metadata

**Confidence breakdown:**
- Standard stack: HIGH — all versions verified via `cargo search` (live); APIs verified via docs.rs
- Architecture: HIGH — background thread + crossbeam pattern is the verified `asyncgit` production pattern
- Pitfalls: HIGH — most are compiler-enforced (`!Send`) or directly observable from the codebase
- Word diff: MEDIUM — `similar` API verified; the line-pairing heuristic is a design choice

**Research date:** 2026-02-18
**Valid until:** 2026-04-18 (stable crates; syntect-tui and similar are slow-moving)
