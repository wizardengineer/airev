---
phase: 03-git-layer
plan: 06
subsystem: ui
tags: [git2, ratatui, diff, file-list, status-bar]

# Dependency graph
requires:
  - phase: 03-git-layer
    provides: "AsyncGit worker thread with FileSummary types, hunk_offsets, and file jump keybinding"

provides:
  - "Per-file added/removed line counts populated from diff line origins"
  - "file_line_offsets field in GitResultPayload mapping file index to highlighted_lines position"
  - "jump_to_selected_file uses file_line_offsets lookup (not hardcoded 0)"
  - "Status bar file count span ('12 files') alongside diff mode label"

affects:
  - phase: 04-persistence-layer

# Tech tracking
tech-stack:
  added: []
  patterns:
    - "diff.foreach() with all four callbacks for per-file line counting in a single pass"
    - "RefCell shared between multiple foreach closures for file_hunk_starts tracking"
    - "file_hunk_starts -> hunk_offsets -> file_line_offsets index chain for file jump navigation"

key-files:
  created: []
  modified:
    - airev/src/git/worker.rs
    - airev/src/git/types.rs
    - airev/src/app.rs
    - airev/src/ui/layout.rs

key-decisions:
  - "extract_files uses diff.foreach() with line_cb to count +/- origins per file (single pass, no second foreach call)"
  - "extract_hunks returns (hunks, file_hunk_starts) where file_hunk_starts[i] is the hunk index at the start of file i"
  - "file_line_offsets = file_hunk_starts mapped through hunk_offsets — no new data structure needed, reuses existing hunk offset table"
  - "jump_to_selected_file falls back to 0 via .get(idx).copied().unwrap_or(0) when no diff loaded yet"
  - "Status bar file count uses same Color::DarkGray as diff mode label; only shown when file_summaries is non-empty"

patterns-established:
  - "File boundary tracking in extract_hunks: file_cb records current hunk vec length as the file's start index"

requirements-completed:
  - File List Panel

# Metrics
duration: 2min
completed: 2026-02-19
---

# Phase 3 Plan 06: Gap Closure — File List Panel Verification Fixes Summary

**Three targeted fixes closing all File List Panel verification gaps: real per-file +N/-N line counts via diff.foreach(), file jump navigation using file_line_offsets index chain, and status bar file count display**

## Performance

- **Duration:** 2 min
- **Started:** 2026-02-19T18:26:53Z
- **Completed:** 2026-02-19T18:28:59Z
- **Tasks:** 3
- **Files modified:** 4

## Accomplishments
- Gap 1 closed: `extract_files()` now calls `diff.foreach()` with all four callbacks to count `+`/`-` line origins per file — `FileSummary.added` and `.removed` are real values, file list renders `+N -M` badges
- Gap 2 closed: `extract_hunks()` returns `(hunks, file_hunk_starts)` from the file callback; `process_diff()` maps these through `hunk_offsets` to produce `file_line_offsets`; `jump_to_selected_file()` does an index lookup instead of hardcoding 0
- Gap 3 closed: `render_status_bar()` inserts a `'N files'` span between the diff mode label and the loading indicator using the same `'  |  '` separator pattern

## Task Commits

Each task was committed atomically:

1. **Task 1: Populate per-file added/removed line counts in extract_files** - `bbdb834` (feat)
2. **Task 2: Compute per-file line offsets and fix jump_to_selected_file** - `71b203f` (feat)
3. **Task 3: Add file count to status bar** - `67ebffa` (feat)

**Plan metadata:** TBD (docs: complete plan)

## Files Created/Modified
- `airev/src/git/worker.rs` - extract_files rewritten with diff.foreach(); extract_hunks returns file_hunk_starts; process_diff computes file_line_offsets; empty payload fallback updated
- `airev/src/git/types.rs` - file_line_offsets field added to GitResultPayload
- `airev/src/app.rs` - file_line_offsets field in AppState; apply_git_result stores it; jump_to_selected_file uses index lookup
- `airev/src/ui/layout.rs` - file count span in render_status_bar; doc comment updated

## Decisions Made
- `extract_files` uses `diff.foreach()` with a line_cb instead of a second separate foreach call — single-pass, avoids iterating the diff twice
- `extract_hunks` promoted from returning `Vec<OwnedDiffHunk>` to `(Vec<OwnedDiffHunk>, Vec<usize>)` — minimal change, the file_cb already fired once per delta so adding `file_hunk_starts.push(hunks.borrow().len())` costs nothing
- `file_line_offsets` computed by mapping `file_hunk_starts` through `hunk_offsets` — reuses the existing hunk offset table without adding new data structures or scanning `highlighted_lines` for `@@` markers
- `jump_to_selected_file` fallback to 0 via `.get(idx).copied().unwrap_or(0)` — safe for the no-diff-loaded case without panicking

## Deviations from Plan

None - plan executed exactly as written.

## Issues Encountered
None - all three changes compiled on first attempt with no new warnings introduced.

## User Setup Required
None - no external service configuration required.

## Next Phase Readiness
- All 3 File List Panel verification gaps are now closed
- `cargo build --workspace` exits 0 with only pre-existing warnings (not related to changed code)
- Phase 3 Git Layer is fully complete and verified; Phase 4 Persistence Layer can begin

---
## Self-Check: PASSED

All files verified present on disk. All task commits confirmed in git log:
- `bbdb834` feat(03-06): populate per-file added/removed line counts in extract_files
- `71b203f` feat(03-06): compute file_line_offsets and fix jump_to_selected_file
- `67ebffa` feat(03-06): add file count span to render_status_bar
- `7a368ea` docs(03-06): complete gap-closure plan for File List Panel requirement

*Phase: 03-git-layer*
*Completed: 2026-02-19*
