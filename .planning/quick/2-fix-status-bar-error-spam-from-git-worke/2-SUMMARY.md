---
phase: quick
plan: 2
subsystem: ui
tags: [git2, ratatui, tui, error-handling, keybindings]

# Dependency graph
requires:
  - phase: 03-git-layer
    provides: AsyncGit worker thread, Tab diff mode cycle, Enter/l file-list jump wired in plan 04
provides:
  - Silent git worker errors — no eprintln! calls corrupt TUI stderr backend
  - Accurate help overlay listing Tab, Enter/l, and [ / ] as active keybindings
affects: [03-git-layer, ui-polish]

# Tech tracking
tech-stack:
  added: []
  patterns:
    - "Silent failure with empty payload is the correct pattern for TUI apps where stderr is the terminal backend"
    - "Help overlay reflects only wired keybindings — no placeholder descriptions"

key-files:
  created: []
  modified:
    - airev/src/git/worker.rs
    - airev/src/ui/help.rs

key-decisions:
  - "eprintln! removed entirely from git worker — no replacement logging added; empty payload is the correct graceful degradation signal"
  - "handle_request doc comment updated to say 'returns an empty payload for graceful degradation' (not 'logs to stderr')"

patterns-established:
  - "TUI stderr = terminal backend: never write raw text to stderr while TUI is active"

requirements-completed: []

# Metrics
duration: 2min
completed: 2026-02-18
---

# Quick Task 2: Fix Status Bar Error Spam from Git Worker Summary

**Silent git worker errors via empty-payload degradation and accurate Phase 3 keybinding documentation in help overlay**

## Performance

- **Duration:** 2 min
- **Started:** 2026-02-18T09:38:22Z
- **Completed:** 2026-02-18T09:39:25Z
- **Tasks:** 2
- **Files modified:** 2

## Accomplishments

- Removed both `eprintln!` calls from `git_worker_loop` and `handle_request` in `git/worker.rs` — no raw text will corrupt the TUI display on stderr
- Updated `build_help_text()` in `ui/help.rs` to list Tab (diff mode cycle) and Enter/l (file-list jump) as active keybindings, and removed the "(placeholder)" label from the [ / ] hunk navigation entry
- Added a "Diff Mode" section to the help overlay documenting the Tab cycle: Unstaged -> Staged -> Branch vs main -> Commit Range

## Task Commits

Each task was committed atomically:

1. **Task 1: Remove eprintln! calls from git worker** - `82a0684` (fix)
2. **Task 2: Update help overlay with Phase 3 keybindings** - `1aba9e2` (feat)

**Plan metadata:** `(see final docs commit)` (docs: complete quick task 2)

## Files Created/Modified

- `airev/src/git/worker.rs` - Removed two `eprintln!` calls; updated `handle_request` doc comment
- `airev/src/ui/help.rs` - Added Enter/l and Diff Mode section; removed placeholder label from [ / ]

## Decisions Made

- No replacement logging mechanism added to the git worker — silent failure with empty payload is the correct behaviour for a TUI app where stderr is the terminal backend.
- `Err(e)` bindings changed to `Err(_)` since the error value is no longer used.

## Deviations from Plan

None - plan executed exactly as written.

## Issues Encountered

None.

## User Setup Required

None - no external service configuration required.

## Next Phase Readiness

- Git worker is now display-safe: running airev against a repo without a "main" branch (or any other git error) will show a blank diff panel without corrupting the TUI.
- Help overlay accurately reflects all keybindings wired through Phase 3 Plan 04.
- Ready to continue with Phase 3 Plan 05.

---
*Phase: quick*
*Completed: 2026-02-18*

## Self-Check: PASSED

- FOUND: airev/src/git/worker.rs
- FOUND: airev/src/ui/help.rs
- FOUND: .planning/quick/2-fix-status-bar-error-spam-from-git-worke/2-SUMMARY.md
- FOUND commits: 82a0684 (fix), 1aba9e2 (feat)
