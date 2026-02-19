---
phase: 03-git-layer
verified: 2026-02-19T09:15:00Z
status: passed
score: 12/12 must-haves verified
re_verification:
  previous_status: gaps_found
  previous_score: 9/12
  gaps_closed:
    - "File list shows changed line count (+N/-N) per entry — extract_files() now counts '+'/'-' origins in line callback"
    - "Enter or l on file list jumps diff panel to that file's changes — jump_to_selected_file() uses file_line_offsets[idx]"
    - "File count appears in the status bar — render_status_bar() now emits '{N} files' span"
  gaps_remaining: []
  regressions: []
human_verification:
  - test: "First screenful load time under 100ms"
    expected: "Diff panel shows real content within 100ms on a repo with 50+ changed files (ROADMAP exit criterion)"
    why_human: "Cannot measure render latency programmatically without an instrumented test harness"
  - test: "60fps scrolling on 5000+ line diff"
    expected: "Hold j for 3 seconds — no visible stutter or frame drops (ROADMAP exit criterion)"
    why_human: "Frame-drop detection requires visual observation; cannot grep for it"
  - test: "All four diff modes display correct content"
    expected: "Tab cycles Unstaged->Staged->Branch->Range and each loads the appropriate diff"
    why_human: "Requires a live git repository with staged and unstaged changes to verify each mode"
  - test: "syntect syntax highlighting is visually correct"
    expected: "Rust/JS/Python files show token-colored output, not monochrome text"
    why_human: "Color rendering requires terminal observation"
  - test: "Word-level diff emphasis is visible"
    expected: "Modified lines show changed words in bold vs unchanged words in plain diff color"
    why_human: "Bold modifier rendering is terminal-dependent and requires visual inspection"
  - test: "File jump scrolls to correct file position"
    expected: "Selecting the 3rd file in the list and pressing Enter scrolls the diff panel to that file's first hunk, not the top"
    why_human: "Requires a live repo with multiple changed files to confirm file_line_offsets are computed correctly end-to-end"
---

# Phase 3: Git Layer Verification Report (Re-verification)

**Phase Goal:** The diff view renders real git content — syntax-highlighted, virtually scrolled, and produced by a background thread that cannot freeze the UI — across all four diff modes.
**Verified:** 2026-02-19T09:15:00Z
**Status:** passed
**Re-verification:** Yes — after gap closure (previous score 9/12, now 12/12)

## Re-verification Summary

All three gaps from the initial verification have been closed. No regressions were introduced.

| Gap | Previous Status | Current Status | Fix |
|-----|----------------|----------------|-----|
| File list +N/-N counts always 0 | FAILED | CLOSED | `extract_files()` now uses `diff.foreach()` with a line callback counting `'+'`/`'-'` origins per file |
| Enter/l jumps to top of diff, not file | FAILED | CLOSED | `jump_to_selected_file()` now reads `file_line_offsets[idx]`; `process_diff()` computes `file_line_offsets` from `file_hunk_starts` mapped through `hunk_offsets` |
| File count in panel title, not status bar | PARTIAL | CLOSED | `render_status_bar()` now emits a `"{N} files"` span when `!state.file_summaries.is_empty()` |

---

## Goal Achievement

### Observable Truths

| # | Truth | Status | Evidence |
|---|-------|--------|----------|
| 1 | `cargo build --workspace` exits 0 | VERIFIED | Build completes with 0 errors, 7 dead_code warnings only (same as initial) |
| 2 | OwnedDiffHunk, OwnedDiffLine, FileSummary, DiffMode, GitResultPayload are all owned (no lifetimes) and Send | VERIFIED | All fields in `git/types.rs` are String/u32/char/Vec/usize; no lifetime params; compiler accepts Send |
| 3 | AppEvent::GitResult(Box\<GitResultPayload\>) carries real payload | VERIFIED | `event.rs` line 45: `GitResult(Box<crate::git::types::GitResultPayload>)` |
| 4 | DiffMode has four variants: Unstaged, Staged, CommitRange, BranchComparison | VERIFIED | `git/types.rs` lines 64-74: all 4 variants present |
| 5 | Background thread opens Repository inside std::thread::spawn | VERIFIED | `worker.rs` line 40: `Repository::open(&path)` inside spawned thread; no unsafe |
| 6 | All four diff modes handled in worker | VERIFIED | `get_diff_for_mode()` handles Unstaged/Staged/BranchComparison; `get_diff_for_range()` handles CommitRange |
| 7 | Syntax highlighting runs in background thread via syntect LazyLock statics | VERIFIED | PS/TS are LazyLock statics; `highlight_hunks()` called from `handle_request()` inside worker thread |
| 8 | Word-level diff via similar::TextDiff::from_words() for consecutive -/+ pairs | VERIFIED | `word_diff_spans()` at line 296; called at line 385 for paired lines; `TextDiff::from_words` used |
| 9 | AppState has all Phase 3 fields including file_line_offsets: Vec\<usize\> | VERIFIED | All fields confirmed: `diff_lines`, `file_summaries`, `diff_mode`, `diff_loading`, `hunk_offsets`, `selected_file_index`, `hunk_cursor`, `git_tx`, `file_line_offsets` all present |
| 10 | Diff panel renders via List widget with manual virtual slice | VERIFIED | `diff_view.rs` uses List+ListItem; slice `[visible_start..visible_end]` at lines 57-62; Paragraph not used |
| 11 | File list panel renders real FileSummary data with M/A/D/R badges and +N/-N counts | VERIFIED | `file_tree.rs`: badge per status char with correct colors; `extract_files()` now populates `added`/`removed` via line callback; `file_summary_item()` renders counts when > 0 |
| 12 | Enter or l on file list jumps diff to that file's changes | VERIFIED | `jump_to_selected_file()` (app.rs line 360-367): reads `self.file_line_offsets.get(idx).copied().unwrap_or(0)`; `process_diff()` computes `file_line_offsets` from `file_hunk_starts`; `apply_git_result()` stores them |
| 13 | File count shown in status bar | VERIFIED | `render_status_bar()` (layout.rs lines 169-175): emits `"{N} files"` span when `!state.file_summaries.is_empty()` |
| 14 | main.rs spawns AsyncGit and handles GitResult via apply_git_result | VERIFIED | Lines 114-115: `AsyncGit::new` spawned; lines 160-161: `GitResult` arm calls `apply_git_result` |
| 15 | Tab keybinding cycles DiffMode and sends new GitRequest | VERIFIED | `keybindings.rs` line 132: Tab arm cycles all 4 modes and sends via `git_tx` |
| 16 | Status bar shows DiffMode label and loading indicator | VERIFIED | `layout.rs` lines 156-180: `diff_mode_label` and `"Computing diff..."` shown conditionally |
| 17 | [ and ] call prev_hunk() and next_hunk() (real methods, not stubs) | VERIFIED | `keybindings.rs` lines 85-86: calls `state.prev_hunk()` / `state.next_hunk()`; methods navigate `hunk_offsets` |

**Score:** 12/12 truths verified

### Required Artifacts

| Artifact | Expected | Status | Details |
|----------|----------|--------|---------|
| `airev/src/git/types.rs` | OwnedDiffHunk, OwnedDiffLine, FileSummary, DiffMode, GitRequest, GitResultPayload with file_line_offsets | VERIFIED | All 6 types present; `GitResultPayload` now includes `file_line_offsets: Vec<usize>`; all owned, no lifetimes |
| `airev/src/event.rs` | AppEvent::GitResult(Box\<GitResultPayload\>) | VERIFIED | Line 45: `GitResult(Box<crate::git::types::GitResultPayload>)` |
| `airev/src/git/worker.rs` | git_worker_loop + 4 diff modes + syntect + word-level diff + line count pass + file_line_offsets | VERIFIED | `extract_files()` counts '+'/'-' in line callback; `process_diff()` builds `file_line_offsets` from `file_hunk_starts`; full implementation, no unsafe |
| `airev/src/git/mod.rs` | AsyncGit struct with new() and load_diff() | VERIFIED | AsyncGit struct; `new()` spawns thread; `load_diff()` sends request |
| `airev/src/app.rs` | Extended AppState with all Phase 3 fields; apply_git_result, next_hunk, prev_hunk, jump_to_selected_file | VERIFIED | All fields present including `file_line_offsets`; `jump_to_selected_file()` uses `file_line_offsets[idx]`; `apply_git_result()` stores `file_line_offsets` |
| `airev/src/ui/diff_view.rs` | render_diff() using virtual List scrolling | VERIFIED | List widget with bounded `[visible_start..visible_end]` slice; O(viewport) rendering |
| `airev/src/ui/file_tree.rs` | render_file_list() with status badges and +N/-N counts | VERIFIED | Badges work; `file_summary_item()` renders counts (now reachable since `extract_files()` populates them) |
| `airev/src/main.rs` | AsyncGit spawned; GitResult arm; git_tx in AppState | VERIFIED | All three present |
| `airev/src/ui/keybindings.rs` | Tab cycle; Enter/l jump; [/] hunk nav | VERIFIED | Tab cycles correctly; Enter/l calls `jump_to_selected_file()` which now uses file offsets; [/] call real methods |
| `airev/src/ui/layout.rs` | render_status_bar shows DiffMode, loading, and file count | VERIFIED | DiffMode label, "Computing diff...", and "{N} files" all present |

### Key Link Verification

| From | To | Via | Status | Details |
|------|----|-----|--------|---------|
| `event.rs` | `git/types.rs` | `use crate::git::types::GitResultPayload` | WIRED | Line 45: `GitResult(Box<crate::git::types::GitResultPayload>)` |
| `git/mod.rs` | `git/worker.rs` | `std::thread::spawn(move \|\| git_worker_loop(...))` | WIRED | `worker::git_worker_loop(repo_path, request_rx, event_tx)` |
| `git/worker.rs` | `event.rs` | `event_tx.send(AppEvent::GitResult(Box::new(payload)))` | WIRED | Line 49: exact pattern present |
| `main.rs` | `git/mod.rs` | `AsyncGit::new(handler.tx.clone(), repo_path)` | WIRED | Line 115: `crate::git::AsyncGit::new(handler.tx.clone(), path)` |
| `main.rs` | `app.rs` | `state.apply_git_result(*payload)` | WIRED | Line 161: `state.apply_git_result(*payload)` |
| `keybindings.rs` | `app.rs` | `state.jump_to_selected_file()` | WIRED | Line 127: call present; method now reads `file_line_offsets[idx]` (not hardcoded 0) |
| `worker.rs` | `app.rs` (via payload) | `file_line_offsets` computed in `process_diff()`, stored via `apply_git_result()` | WIRED | `process_diff()` lines 139-142 build offsets; `apply_git_result()` line 347 stores them |
| `ui/mod.rs` | `ui/diff_view.rs` | `diff_view::render_diff(frame, center, focus, state, theme)` | WIRED | Confirmed present |
| `ui/mod.rs` | `ui/file_tree.rs` | `file_tree::render_file_list(frame, left, focus, state, theme)` | WIRED | Confirmed present |

### Requirements Coverage

| Requirement ID | Source Plans | Description | Status | Evidence / Notes |
|----------------|-------------|-------------|--------|-----------------|
| Diff View | 03-01, 03-02, 03-03, 03-04, 03-05 | Syntax-highlighted unified diff; virtual scrolling; word-level diff; `@@` hunk headers | SATISFIED | List widget O(viewport) slice; syntect highlighting; `word_diff_spans` for -/+ pairs; hunk headers in Cyan |
| Diff Modes | 03-01, 03-02, 03-04, 03-05 | 4 modes: Unstaged, Staged, CommitRange, BranchComparison; Tab to cycle | SATISFIED | All 4 DiffMode variants; Tab keybinding cycles and sends GitRequest; status bar shows mode label |
| Git Integration | 03-01, 03-02 | git2::Repository on dedicated std::thread; crossbeam-channel; results as owned Vec\<DiffHunk\> | SATISFIED | Repository opened inside thread (compiler-enforced !Send); no unsafe; crossbeam-channel wired; all data owned |
| File List Panel | 03-03, 03-04, 03-05 | Files from git diff; M/A/D/R badges; +N/-N counts; Enter/l jump to file | SATISFIED | Badges work; +N/-N counts now populated (extract_files counts '+'/'-' in line callback); Enter/l navigates to file's actual diff position via file_line_offsets |

**Orphaned requirements check:** No requirements in REQUIREMENTS.md are annotated with Phase 3 mapping tags beyond the four IDs declared in plan frontmatter, all of which are accounted for above.

### Anti-Patterns Found

No new anti-patterns introduced. The three previously flagged anti-patterns (hardcoded zeroes, diff_scroll=0 in jump, and unreachable branch) are all resolved. Remaining info-level item:

| File | Line | Pattern | Severity | Impact |
|------|------|---------|----------|--------|
| `airev/src/git/worker.rs` | (Cargo.toml) | `syntect-tui` crate listed as dep but `syntect_to_span` is a custom replacement | Info | Unused dep (no active `unused-crates` lint); does not affect correctness |

### Human Verification Required

#### 1. First Screenful Load Time Under 100ms

**Test:** Run `cargo run -p airev` in a repo with 50+ changed files. Time from launch to first visible diff content in the panel.
**Expected:** Diff panel shows highlighted content within 100ms; "Computing diff..." may flash briefly.
**Why human:** No instrumented benchmark harness exists; render latency requires stopwatch or visual observation.

#### 2. 60fps Scrolling on 5000+ Line Diff

**Test:** Clone `https://github.com/tokio-rs/tokio` to `/tmp/tokio-test`, run `cargo run -p airev` from that directory with staged changes. Hold `j` for 3 seconds.
**Expected:** No visible stutter, frame drops, or tearing. Virtual scroll (O(viewport)) should keep rendering constant regardless of diff size.
**Why human:** Frame-drop detection requires visual observation; cannot be verified programmatically.

#### 3. All Four Diff Modes Display Correct Content

**Test:** In a repo with staged and unstaged changes, press Tab four times to cycle through all modes. Verify each mode shows different (mode-appropriate) content.
**Expected:** Unstaged -> Staged -> Branch -> Range -> Unstaged. Each mode loads and displays correctly.
**Why human:** Requires live git repository with both staged and unstaged changes.

#### 4. Syntax Highlighting Visually Correct

**Test:** Open a Rust or Python file's diff and observe the syntax highlighting.
**Expected:** Token-colored spans (keywords, strings, types) vs plain monochrome text. The `base16-ocean.dark` theme should be active.
**Why human:** Color rendering is terminal-dependent; cannot grep for visual output.

#### 5. Word-Level Diff Emphasis Visible

**Test:** Edit a single line in a tracked file (change a few words), then view the diff.
**Expected:** The removed line (`-`) shows changed words in bold-red, unchanged words in plain-red. The added line (`+`) shows changed words in bold-green, unchanged in plain-green.
**Why human:** Bold modifier rendering requires visual inspection in a real terminal.

#### 6. File Jump Scrolls to Correct File Position

**Test:** In a repo with at least 3 changed files, select the 3rd file in the file list and press Enter.
**Expected:** The diff panel scrolls to that file's first hunk header, not to the top of the entire diff.
**Why human:** Requires a live repo to confirm `file_line_offsets` are computed and applied correctly end-to-end; cannot verify the actual scroll position programmatically.

---

_Verified: 2026-02-19T09:15:00Z_
_Verifier: Claude (gsd-verifier)_
