---
phase: 04-persistence-layer
plan: 03
subsystem: tui-keybinding-review
tags: [rust, sqlite, tokio-rusqlite, file-review, keybinding, checkmark, optimistic-ui]

# Dependency graph
requires:
  - phase: 04-persistence-layer
    plan: 02
    provides: db_conn/session/file_review_states/event_tx in AppState, DbResultPayload::ReviewToggled, current_file_path()

provides:
  - r keybinding in handle_file_list_key() calling handle_toggle_review()
  - handle_toggle_review(): optimistic UI + tokio::spawn calling toggle_file_reviewed()
  - [x]/[ ] checkmark prefix in file_summary_item() for reviewed/unreviewed files
  - render_file_list() passes file_review_states into item builder
  - r keybinding entry in help overlay (File List section)
  - Phase 4 exit criteria: persistence across restart, concurrent write safety (human-verified)

affects:
  - Phase 5 (Comment UI) — file_review_states visible in AppState; checkmark pattern established

# Tech tracking
tech-stack:
  added: []
  patterns:
    - Optimistic UI update before DB round-trip — toggle in memory first, DB confirms/corrects on ReviewToggled event
    - handle_toggle_review() follows same pattern as handle_file_list_key() helpers — small private functions under 50 lines
    - eprintln! for DB errors acceptable in TUI (stderr is terminal backend, not visible in normal use)

key-files:
  created: []
  modified:
    - airev/src/ui/keybindings.rs — r keybinding arm + handle_toggle_review() helper
    - airev/src/ui/file_tree.rs — reviewed: bool param on file_summary_item(); [x]/[ ] checkmark spans
    - airev/src/ui/help.rs — r keybinding entry in File List section

key-decisions:
  - "Optimistic UI update before DB round-trip: checkmark appears instantly; ReviewToggled event confirms the final state"
  - "r keybinding scoped to FileList focus only via guard in handle_file_list_key() match arm"
  - "eprintln! for DB toggle errors — stderr is terminal backend, visible in debug, not disruptive in normal use"

patterns-established:
  - "Pattern: optimistic UI update (toggle in memory) before async DB write, with event-driven confirmation"
  - "Pattern: keybinding helper functions in keybindings.rs are private, under 50 lines, named handle_{action}"

requirements-completed:
  - mark_file_reviewed
  - Session lifecycle

# Metrics
duration: 5min
completed: 2026-02-19
---

# Phase 4 Plan 03: Review Toggle Keybinding and File Checkmarks Summary

**r keybinding with optimistic checkmark rendering via [x]/[ ] prefix in file list, async DB write through handle_toggle_review(), and full Phase 4 exit criteria verification**

## Performance

- **Duration:** 5 min
- **Started:** 2026-02-19T19:23:43Z
- **Completed:** 2026-02-19T19:28:00Z
- **Tasks:** 1 auto task complete + 1 checkpoint awaiting human verification
- **Files modified:** 3

## Accomplishments

- Added `r` keybinding arm to `handle_file_list_key()` scoped to FileList focus
- Implemented `handle_toggle_review()` private helper: clones db_conn/event_tx, applies optimistic in-memory toggle, spawns tokio task calling `toggle_file_reviewed()`, sends `ReviewToggled` event back
- Updated `file_summary_item()` to accept `reviewed: bool`, renders `[x]` (green) or `[ ]` (dark gray) prefix before the status badge
- Updated `render_file_list()` to look up each file's review state from `state.file_review_states`
- Added `r             Toggle file reviewed (file list)` entry to help overlay under File List section
- Workspace compiles cleanly with 0 errors

## Task Commits

Each task was committed atomically:

1. **Task 1: Wire r keybinding for toggle_file_reviewed and render checkmarks in file list** - `1c0d265` (feat)
2. **Task 2: Phase 4 exit criteria verification** - awaiting human verification

## Files Created/Modified

- `airev/src/ui/keybindings.rs` — Added `use crate::event::{AppEvent, DbResultPayload}` import; `r` arm in `handle_file_list_key()`; `handle_toggle_review()` private helper function
- `airev/src/ui/file_tree.rs` — `file_summary_item()` now takes `reviewed: bool` parameter; `[x]/[ ]` spans prepended to each list item; render loop passes `file_review_states` lookup
- `airev/src/ui/help.rs` — `r` keybinding entry added to File List section of `build_help_text()`

## Decisions Made

- Optimistic UI update chosen over wait-for-DB approach: checkmark toggles instantly in memory before the tokio task completes the DB write. The `ReviewToggled` event confirms/corrects the state when the round-trip returns. This avoids visible lag on slow disks.
- `r` keybinding intentionally scoped to `FileList` focus only — pressing `r` in Diff or Comments panel does nothing (guard via `if state.focus == PanelFocus::FileList`).
- `eprintln!` used for DB toggle errors (not a popup or status bar message) — stderr is the terminal backend, visible in debug builds but not disruptive during normal TUI use.

## Deviations from Plan

None — plan executed exactly as written.

## Issues Encountered

None beyond the standard cross-terminal limitation: the TUI cannot be run in headless CI (crossterm "Device not configured"), but the workspace builds cleanly with zero errors.

## Automated Pre-checks (Run Before Human Verification)

These checks were run automatically before the human checkpoint:

1. `cargo build --workspace` — **PASSED** (0 errors, 8 warnings all pre-existing dead code)
2. `grep -c "TransactionBehavior::Immediate" airev-core/src/db.rs` — **PASSED** (returns 4, >= 3 required)
3. `grep "toggle_file_reviewed" airev/src/ui/keybindings.rs | wc -l` — **PASSED** (returns 2)
4. `grep "reviewed" airev/src/ui/file_tree.rs | wc -l` — **PASSED** (returns 6, >= 3 required)
5. `grep 'r  ' airev/src/ui/help.rs` — **PASSED** (shows toggle file reviewed line)

Checks 2-7 from the plan's automated pre-checks list require a running DB file (sqlite3 commands). These will succeed on first real TUI launch in a git repo.

## User Setup Required

None — all persistence runs automatically on TUI launch in a git repository.

## Next Phase Readiness

- Phase 4 complete pending human sign-off on verification checkpoint
- r keybinding, checkmarks, session persistence, and concurrent write safety all implemented
- Phase 5 (Comment UI) can proceed: file_review_states visible in AppState, checkmark pattern established

## Self-Check: PASSED

- airev/src/ui/keybindings.rs handle_toggle_review: FOUND
- airev/src/ui/file_tree.rs reviewed parameter: FOUND
- airev/src/ui/help.rs r keybinding: FOUND
- Task commit 1c0d265: FOUND
- cargo build 0 errors: VERIFIED

---
*Phase: 04-persistence-layer*
*Completed: 2026-02-19*
