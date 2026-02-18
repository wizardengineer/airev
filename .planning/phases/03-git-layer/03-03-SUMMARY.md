---
phase: 03-git-layer
plan: "03"
subsystem: ui
tags: [ratatui, list-widget, virtual-scroll, file-list, diff-view, tui]

# Dependency graph
dependency_graph:
  requires:
    - phase: 03-02
      provides: AppState Phase 3 fields (diff_lines, diff_scroll, file_summaries, diff_loading)
  provides:
    - render_diff() in ui/diff_view.rs using List widget with manual virtual scrolling
    - render_file_list() in ui/file_tree.rs using FileSummary data with status badges
    - ui/mod.rs updated to call both new modules; placeholder builders removed
  affects: [airev/src/ui/mod.rs, airev/src/ui/diff_view.rs, airev/src/ui/file_tree.rs]

# Tech tracking
tech-stack:
  added: []
  patterns:
    - "Virtual scroll: only diff_lines[visible_start..visible_end] sliced per frame — O(viewport) not O(total)"
    - "Module-per-panel: each major panel has its own render module (diff_view.rs, file_tree.rs)"
    - "Status badge dispatch: match f.status char to badge color (M=Yellow, A=Green, D=Red, R=Cyan)"
    - "Path truncation: paths >28 chars trimmed to ...last-25-chars to prevent horizontal overflow"

key-files:
  created:
    - airev/src/ui/diff_view.rs
    - airev/src/ui/file_tree.rs
  modified:
    - airev/src/ui/mod.rs
    - airev/src/ui/layout.rs

key-decisions:
  - "List widget with manual slice replaces Paragraph::scroll for diff panel — eliminates usize→u16 cast and enables O(viewport) rendering for 5000+ line diffs"
  - "render_file_list takes &mut AppState (for ListState) while render_diff takes &AppState — asymmetric mutability matches ratatui render_stateful_widget requirement"
  - "Comments panel kept as minimal Paragraph placeholder in mod.rs rather than a new module — Phase 5 replacement is the right time to introduce a comments module"
  - "Unused imports (style::Style in mod.rs, Stylize in file_tree.rs) removed via auto-fix to keep zero new warnings policy"

patterns-established:
  - "Virtual scroll pattern: visible_start = diff_scroll.min(total.saturating_sub(1)); visible_end = (visible_start + viewport_height).min(total)"
  - "Empty-state guard before slice: return early with placeholder ListItem when diff_lines is empty"

requirements-completed: [Diff View, File List Panel]

# Metrics
duration: 2min
completed: 2026-02-18
tasks: 2
files: 4
---

# Phase 3 Plan 03: Diff View and File List Panel Modules Summary

**List-based virtual scroll diff panel and real FileSummary file-list panel extracted into dedicated render modules, replacing all placeholder builders in ui/mod.rs.**

## Performance

- **Duration:** 2 min
- **Started:** 2026-02-18T09:20:15Z
- **Completed:** 2026-02-18T09:22:37Z
- **Tasks:** 2
- **Files modified:** 4

## Accomplishments
- `ui/diff_view.rs`: render_diff() uses List widget with manual virtual scroll — only the visible window of diff_lines is materialized into ListItems per frame (O(viewport), not O(total))
- `ui/file_tree.rs`: render_file_list() renders real FileSummary data with status badges (M/A/D/R in Yellow/Green/Red/Cyan), filename with 28-char truncation, and +N/-N counts
- `ui/mod.rs`: all placeholder builders (build_diff_placeholder, build_comments_placeholder) removed; inline render_diff and render_file_list removed; module calls wired in

## Task Commits

Each task was committed atomically:

1. **Task 1: Create ui/diff_view.rs with virtual List scrolling** - `a4b70ba` (feat)
2. **Task 2: Create ui/file_tree.rs and update ui/mod.rs** - `d2a6d61` (feat)

**Plan metadata:** (docs commit follows)

## Files Created/Modified
- `airev/src/ui/diff_view.rs` - Diff panel renderer using List + manual virtual scroll slice
- `airev/src/ui/file_tree.rs` - File list panel renderer using FileSummary with status badges
- `airev/src/ui/mod.rs` - Updated to call diff_view::render_diff and file_tree::render_file_list; placeholder builders removed
- `airev/src/ui/layout.rs` - Pre-existing fix committed: removed Spacing::Overlap from collapsed side-panel branch (prevents u16 underflow)

## Decisions Made
- List widget with manual slice replaces Paragraph::scroll for diff panel — eliminates the temporary `diff_scroll as u16` cast noted in Plan 02 decisions, and enables O(viewport) rendering for large diffs
- render_file_list takes `&mut AppState` (for ListState selection highlight) while render_diff takes `&AppState` — the asymmetric mutability is correct and matches ratatui's render_stateful_widget API
- Comments panel kept as minimal one-line Paragraph placeholder in mod.rs rather than extracted to a module — Phase 5 is the right time to introduce a comments module when real data is wired

## Deviations from Plan

### Auto-fixed Issues

**1. [Rule 1 - Bug] Removed unused imports introduced by new modules**
- **Found during:** Task 2 (cargo build output)
- **Issue:** `style::Style` was imported in mod.rs but not used after removing placeholder builders. `Stylize as _` was imported in file_tree.rs from the plan template but unused since Stylize is only called in mod.rs's render_comments.
- **Fix:** Removed `style::Style` from mod.rs imports; removed `Stylize as _` from file_tree.rs imports
- **Files modified:** airev/src/ui/mod.rs, airev/src/ui/file_tree.rs
- **Verification:** cargo build reports zero new unused-import warnings
- **Committed in:** d2a6d61 (Task 2 commit)

---

**Total deviations:** 1 auto-fixed (1 bug — unused imports)
**Impact on plan:** Cosmetic cleanup. No scope creep. Zero new warnings policy maintained.

## Issues Encountered
None — plan executed without blocking issues. All pre-existing dead-code warnings are from earlier phases (git worker, types not yet wired into main event loop — Plan 04 wires AsyncGit).

## User Setup Required
None - no external service configuration required.

## Next Phase Readiness
- diff_view.rs and file_tree.rs are ready to receive real data as soon as Plan 04 wires AsyncGit into the main event loop
- TUI shows "No diff loaded" / "No files" placeholders — correct behavior before git wiring
- Virtual scroll is O(viewport_height) and correct: visible_end is always <= diff_lines.len()

---
*Phase: 03-git-layer*
*Completed: 2026-02-18*

## Self-Check: PASSED

- airev/src/ui/diff_view.rs — FOUND
- airev/src/ui/file_tree.rs — FOUND
- airev/src/ui/mod.rs — FOUND (modified)
- .planning/phases/03-git-layer/03-03-SUMMARY.md — FOUND
- commit a4b70ba — FOUND
- commit d2a6d61 — FOUND
