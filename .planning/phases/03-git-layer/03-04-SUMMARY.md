---
phase: 03-git-layer
plan: 04
subsystem: ui
tags: [git2, crossbeam-channel, ratatui, asyncgit, keybindings, status-bar]

# Dependency graph
requires:
  - phase: 03-git-layer-plan-01
    provides: AsyncGit facade, GitRequest/GitResultPayload types, AppEvent::GitResult variant
  - phase: 03-git-layer-plan-02
    provides: git_worker_loop, apply_git_result(), prev_hunk()/next_hunk() AppState methods
  - phase: 03-git-layer-plan-03
    provides: render_diff and render_file_list modules wired into ui/mod.rs
provides:
  - AsyncGit spawned at startup from main.rs with git repo auto-detection
  - AppEvent::GitResult arm in event loop applies payload and triggers Render
  - git_tx field in AppState routes keybinding requests to background thread
  - Tab keybinding cycles DiffMode and sends GitRequest::LoadDiff
  - Enter/l keybindings on FileList panel call jump_to_selected_file()
  - Status bar shows DiffMode label (UNSTAGED/STAGED/BRANCH/RANGE) and loading indicator
affects:
  - 03-git-layer-plan-05
  - 04-persistence-layer
  - 05-comment-ui

# Tech tracking
tech-stack:
  added: []
  patterns:
    - Option<AsyncGit> + Option<Sender<GitRequest>> in AppState: graceful no-repo mode without branching event loop
    - handle_file_list_key() helper extracted from handle_normal() to stay under 50-line function limit
    - AppEvent::GitResult explicit arm before _ catch-all takes priority without breaking #[non_exhaustive] match

key-files:
  created: []
  modified:
    - airev/src/main.rs
    - airev/src/app.rs
    - airev/src/ui/keybindings.rs
    - airev/src/ui/layout.rs

key-decisions:
  - "git_tx stored in AppState (Option<Sender<GitRequest>>) so keybindings.rs can send requests without adding a parameter to handle_key()"
  - "Tab key cycles DiffMode globally (not scoped to FileList focus) so any panel can switch diff modes"
  - "Enter and l are scoped to PanelFocus::FileList via match guard to avoid shadowing other panels"
  - "handle_file_list_key() extracted as private helper to keep handle_normal() within 50-line function limit"
  - "Status bar DiffMode label uses Color::DarkGray; loading indicator uses Color::Yellow for visual prominence"
  - "git2::Repository::discover('.') walk from cwd — graceful None if no repo found, diff panel shows placeholder"

patterns-established:
  - "Pattern: Option<AsyncGit> + state.git_tx = None — TUI works without git repo, diff panel shows placeholder"
  - "Pattern: AppEvent::GitResult arm triggers Render immediately after apply_git_result() for instant diff display"

requirements-completed:
  - Diff Modes
  - File List Panel
  - Diff View

# Metrics
duration: 2min
completed: 2026-02-18
---

# Phase 3 Plan 04: Wire Git Layer into Live Application Summary

**AsyncGit background thread wired into main.rs event loop with repo auto-detection, Tab diff mode cycling, Enter/l file-list jump keybindings, and DiffMode + loading status bar**

## Performance

- **Duration:** 2 min
- **Started:** 2026-02-18T09:25:54Z
- **Completed:** 2026-02-18T09:27:54Z
- **Tasks:** 2
- **Files modified:** 4

## Accomplishments
- main.rs now spawns AsyncGit at startup when inside a git repo (graceful no-repo mode via `git2::Repository::discover(".")`)
- AppEvent::GitResult arm applies payload to AppState and triggers immediate Render event
- Tab keybinding cycles DiffMode through Unstaged -> Staged -> BranchComparison -> CommitRange -> Unstaged, sending GitRequest to background thread
- Enter and l keybindings on the FileList panel call `jump_to_selected_file()` and shift focus to the Diff panel
- Status bar now shows UNSTAGED/STAGED/BRANCH/RANGE label and "Computing diff..." while diff_loading is true

## Task Commits

Each task was committed atomically:

1. **Task 1: Wire AsyncGit into main.rs and handle AppEvent::GitResult** - `cd393d7` (feat)
2. **Task 2: Add diff mode keybindings, file-list jump, and status bar loading indicator** - `c3f7bce` (feat)

**Plan metadata:** (docs commit follows)

## Files Created/Modified
- `airev/src/main.rs` - git repo detection, AsyncGit spawn, git_tx assignment, GitResult event arm
- `airev/src/app.rs` - added `pub git_tx: Option<Sender<GitRequest>>` field; crossbeam_channel + GitRequest imports
- `airev/src/ui/keybindings.rs` - added handle_file_list_key() helper with Tab/Enter/l; DiffMode + GitRequest imports
- `airev/src/ui/layout.rs` - render_status_bar extended with DiffMode label and "Computing diff..." indicator; DiffMode import

## Decisions Made
- `git_tx` stored in `AppState` (not passed as parameter to `handle_key`) to avoid changing the public API signature
- `handle_file_list_key()` private helper extracted to keep `handle_normal()` within 50-line function limit
- Tab key operates globally (not scoped to FileList focus) so users can switch diff modes from any panel
- `Enter` and `Char('l')` combined in a single `|` match arm with a `if state.focus == PanelFocus::FileList` guard

## Deviations from Plan

None - plan executed exactly as written.

## Issues Encountered
None.

## User Setup Required
None - no external service configuration required.

## Next Phase Readiness
- Full git data pipeline is live: AsyncGit spawns at startup, loads real diff data, and populates the diff panel
- Status bar communicates current mode and loading state to the user
- Phase 3 Plan 05 can proceed to wire the file-watcher and any remaining git layer polish
- Plans 04+ phases (Persistence Layer, Comment UI) are unblocked

## Self-Check: PASSED

- FOUND: `.planning/phases/03-git-layer/03-04-SUMMARY.md`
- FOUND: `airev/src/main.rs` (modified)
- FOUND: `airev/src/app.rs` (modified)
- FOUND: `airev/src/ui/keybindings.rs` (modified)
- FOUND: `airev/src/ui/layout.rs` (modified)
- FOUND commit: `cd393d7` (Task 1)
- FOUND commit: `c3f7bce` (Task 2)

---
*Phase: 03-git-layer*
*Completed: 2026-02-18*
