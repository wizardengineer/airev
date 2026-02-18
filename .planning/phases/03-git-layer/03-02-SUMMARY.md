---
phase: 03-git-layer
plan: "02"
subsystem: git-background-thread
tags: [git2, syntect, similar, async-thread, app-state, word-diff]
dependency_graph:
  requires: [03-01]
  provides: [AsyncGit-facade, git-worker-loop, AppState-phase3-fields]
  affects: [airev/src/app.rs, airev/src/git/mod.rs, airev/src/git/worker.rs, airev/src/ui/mod.rs]
tech_stack:
  added: [RefCell-for-foreach-closures, syntect-manual-span-builder]
  patterns: [background-thread-owns-Repository, LazyLock-for-statics, RefCell-sequential-borrows, word-diff-pairing-heuristic]
key_files:
  created: []
  modified:
    - airev/src/git/worker.rs
    - airev/src/git/mod.rs
    - airev/src/app.rs
    - airev/src/ui/mod.rs
decisions:
  - "RefCell used in extract_hunks to share hunks Vec between two diff.foreach closures without raw pointers or unsafe — safe because git2 calls them sequentially on the same thread"
  - "Custom syntect_to_span replaces into_span to avoid ratatui::style::Style vs ratatui::prelude::Style type split between syntect-tui 3.0 and ratatui 0.30"
  - "Word-level diff implemented in Task 1 commit (8a5dbad) rather than a separate Task 3 commit — the highlight_hunks function was designed to incorporate it from the start"
  - "diff_scroll changed from u16 to usize; ui/mod.rs temporarily casts as u16 for Paragraph::scroll until Plan 03 replaces with List widget"
  - "scroll_bottom for Diff uses diff_lines.len().saturating_sub(1) rather than usize::MAX for more precise bottom-of-content clamping"
metrics:
  duration: 5min
  completed: 2026-02-18
  tasks: 3
  files: 4
---

# Phase 3 Plan 02: Git Background Worker Thread and AppState Extension Summary

AsyncGit worker thread with git2 Repository owning all four diff modes, syntect syntax highlighting, and word-level diff via similar; AppState extended with all Phase 3 diff display fields.

## Tasks Completed

| # | Task | Commit | Files |
|---|------|--------|-------|
| 1 | Implement git/worker.rs and git/mod.rs AsyncGit facade | 8a5dbad | airev/src/git/worker.rs, airev/src/git/mod.rs |
| 2 | Extend AppState with Phase 3 diff display fields | af4c44f | airev/src/app.rs, airev/src/ui/mod.rs |
| 3 | Word-level diff spans (integrated in Task 1) | 8a5dbad | airev/src/git/worker.rs |

## Implementation Notes

### git/worker.rs

The `git_worker_loop` function opens `git2::Repository` inside `std::thread::spawn` (the compiler-enforced !Send constraint). LazyLock statics `PS` (SyntaxSet) and `TS` (ThemeSet) are eagerly initialized at startup to avoid first-request latency.

All four diff modes are handled in `get_diff_for_mode`:
- `Unstaged` → `repo.diff_index_to_workdir`
- `Staged` → `repo.head().peel_to_commit().tree()` + `diff_tree_to_index`
- `BranchComparison` → resolves "main" and HEAD to trees + `diff_tree_to_tree`
- `CommitRange` → only reachable via `GitRequest::LoadDiffRange { from, to }`

The `highlight_hunks` function applies word-level diff emphasis for consecutive -/+ line pairs using `pending_removed` buffering — a removed line is buffered until the next line arrives; if it is `+`, `word_diff_spans` applies `similar::TextDiff::from_words` with bold emphasis on changed words.

### RefCell in extract_hunks

The `diff.foreach` API requires separate closures for hunk headers and line content. Both closures need mutable access to the same `hunks: Vec<OwnedDiffHunk>`. Rust's borrow checker rejects two `&mut` closures over the same binding, even though git2 guarantees sequential (not concurrent) execution. `RefCell` solves this cleanly without unsafe code — `borrow_mut()` panics if called simultaneously, but since both closures run on the same thread sequentially, this never occurs.

### Custom syntect span builder

syntect-tui 3.0.4 links against `ratatui-core 0.1` (a ratatui workspace split), which defines `ratatui::style::Style` as a separate type from `ratatui::prelude::Style` in the main `ratatui 0.30` crate. The types are structurally identical but Rust's type system treats them as distinct, causing an E0277 when calling `Span::styled(content, syntect_tui_style)`. Solution: `syntect_to_span` manually reconstructs color and modifier fields using our own `ratatui::style::Style` builder, bypassing `into_span` entirely.

### AppState Phase 3 fields

The `diff_scroll: u16` field was replaced with `diff_scroll: usize` throughout `app.rs`. All scroll arithmetic was updated to `saturating_add(lines as usize)` / `saturating_sub(lines as usize)`. The `ui/mod.rs` render_diff function still uses `Paragraph::scroll((diff_scroll as u16, 0))` as a temporary cast until Plan 03 switches to a List widget.

## Verification Results

```
cargo build --workspace         → Finished (0 errors, warnings only)
grep -r "unsafe {" airev/src/git/  → (empty — no unsafe blocks)
cargo check --workspace | grep E0277 → (empty — no Send/Sync violations)
grep "diff_scroll: usize" app.rs    → pub diff_scroll: usize
grep "TextDiff::from_words" worker.rs → confirmed
```

## Deviations from Plan

### Integrated Implementation

**1. [Efficiency] Task 3 word-level diff implemented during Task 1**
- **Found during:** Task 1 implementation
- **Issue:** The `highlight_hunks` function was designed to incorporate word-level diff from the start. Writing it as a separate pass would have required refactoring the same function twice.
- **Fix:** Implemented `word_diff_spans` and `pending_removed` buffering logic inline with `highlight_hunks` during Task 1.
- **Files modified:** airev/src/git/worker.rs
- **Commit:** 8a5dbad

### Auto-fixed Issues

**2. [Rule 1 - Bug] into_span type mismatch (ratatui::style::Style vs ratatui::prelude::Style)**
- **Found during:** Task 1 — first build attempt
- **Issue:** syntect-tui 3.0.4 and ratatui 0.30 define `Style` in different crates (`ratatui-core-0.1` vs `ratatui`). `into_span` returns `Span<'a>` with the wrong Style type, causing E0277.
- **Fix:** Replaced `into_span` usage with `syntect_to_span` — a custom 15-line function that reconstructs color and modifier fields using our own `ratatui::style::Style`.
- **Files modified:** airev/src/git/worker.rs
- **Commit:** 8a5dbad

**3. [Rule 1 - Bug] Double mutable borrow in diff.foreach closures**
- **Found during:** Task 1 — first build attempt
- **Issue:** Both the hunk callback and line callback in `diff.foreach()` needed mutable access to `hunks: Vec<OwnedDiffHunk>`. Rust rejects two `&mut` closures over the same variable.
- **Fix:** Wrapped `hunks` in `RefCell` so both closures can call `borrow_mut()` without requiring raw pointers or unsafe code. git2 guarantees sequential execution, so `RefCell` panics are impossible at runtime.
- **Files modified:** airev/src/git/worker.rs
- **Commit:** 8a5dbad

## Self-Check: PASSED

- airev/src/git/worker.rs — FOUND
- airev/src/git/mod.rs — FOUND
- airev/src/app.rs — FOUND
- .planning/phases/03-git-layer/03-02-SUMMARY.md — FOUND
- commit 8a5dbad — FOUND
- commit af4c44f — FOUND
