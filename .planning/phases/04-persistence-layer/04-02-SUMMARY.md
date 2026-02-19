---
phase: 04-persistence-layer
plan: 02
subsystem: tui-db-wiring
tags: [rust, sqlite, tokio-rusqlite, session-lifecycle, app-state, event-bus]

# Dependency graph
requires:
  - phase: 04-persistence-layer
    plan: 01
    provides: detect_or_create_session/load_file_review_state/open_db in airev-core/db.rs

provides:
  - DbResultPayload enum (SessionLoaded/SessionCreated/FileReviewStateLoaded/ReviewToggled)
  - AppEvent::DbResult(Box<DbResultPayload>) typed payload variant
  - AppState fields: db_conn, session, file_review_states, event_tx
  - AppState methods: apply_db_result(), current_file_path()
  - Startup session lifecycle in main.rs before first frame
  - DbResult match arm in event loop
  - Session UUID prefix in status bar

affects:
  - 04-persistence-layer plan-03 (keybinding toggle uses db_conn/session/event_tx/current_file_path)

# Tech tracking
tech-stack:
  added:
    - tokio-rusqlite added to airev package Cargo.toml (was only in workspace, not in airev binary crate)
  patterns:
    - Box<DbResultPayload> in AppEvent keeps channel variants pointer-sized (matches GitResultPayload pattern)
    - Git repo discovery moved before DB open so repo_path is available for session detection
    - event_tx stored in AppState for keybindings.rs to send async DB results without extra handle_key() parameters
    - apply_db_result() method mirrors apply_git_result() pattern — centralized state mutation

key-files:
  created: []
  modified:
    - airev/src/event.rs — DbResultPayload enum + AppEvent::DbResult(Box<DbResultPayload>)
    - airev/src/app.rs — db_conn/session/file_review_states/event_tx fields + apply_db_result()/current_file_path() methods
    - airev/src/main.rs — startup session lifecycle; DbResult match arm; step renumbering
    - airev/src/ui/layout.rs — session UUID prefix in status bar
    - airev/Cargo.toml — tokio-rusqlite added to [dependencies]

key-decisions:
  - "tokio-rusqlite added directly to airev/Cargo.toml — was in workspace.dependencies but not in airev package (Rule 3 auto-fix, compiler error E0433)"
  - "Git repo discovery moved before DB open to provide repo_path for detect_or_create_session() — required restructuring of startup steps 5-6"
  - "event_tx stored in AppState before spawn_event_task() so keybindings.rs has access without changing handle_key() signature"
  - "Session UUID shown as first 8 chars in status bar — enough to identify, not too long"

patterns-established:
  - "Pattern: startup lifecycle (git discovery → DB open → session detect/create → load review state) all complete before event_loop starts"
  - "Pattern: DbResult match arm follows GitResult pattern (apply state + send Render event)"

requirements-completed:
  - SQLite Persistence
  - Session lifecycle

# Metrics
duration: 4min
completed: 2026-02-19
---

# Phase 4 Plan 02: TUI DB Wiring Summary

**Typed DbResultPayload event variant, AppState persistence fields (db_conn/session/file_review_states/event_tx), startup session detect-or-create before first frame, and session UUID in status bar**

## Performance

- **Duration:** 4 min
- **Started:** 2026-02-19T19:15:45Z
- **Completed:** 2026-02-19T19:20:02Z
- **Tasks:** 2
- **Files modified:** 5

## Accomplishments

- Expanded `AppEvent::DbResult` from unit variant to `DbResult(Box<DbResultPayload>)` with 4-variant typed payload enum
- Added `db_conn`, `session`, `file_review_states`, `event_tx` fields to `AppState` struct with `Default` initialization
- Added `apply_db_result()` method (mirrors `apply_git_result()` pattern) and `current_file_path()` helper
- Restructured main.rs startup: git repo discovery moved before DB open; `detect_or_create_session()` and `load_file_review_state()` called synchronously before first frame
- Stored `event_tx` in `AppState` for future keybinding access to async DB operations
- Added `DbResult` match arm in event loop: applies state update and triggers immediate re-render
- Added session UUID prefix display to status bar in `layout.rs`

## Task Commits

Each task was committed atomically:

1. **Task 1: Expand AppEvent::DbResult to typed payload and add DB fields to AppState** - `9ec7c95` (feat)
2. **Task 2: Wire startup session lifecycle and DbResult event handling in main.rs** - `3950815` (feat)

## Files Created/Modified

- `airev/src/event.rs` — DbResultPayload enum with 4 variants; AppEvent::DbResult changed from unit to Box<DbResultPayload>
- `airev/src/app.rs` — db_conn/session/file_review_states/event_tx fields added to AppState; apply_db_result() and current_file_path() methods added
- `airev/src/main.rs` — startup sequence restructured (git discovery first, then DB lifecycle); DbResult match arm added to event loop
- `airev/src/ui/layout.rs` — session UUID prefix (first 8 chars) shown in status bar
- `airev/Cargo.toml` — tokio-rusqlite added to airev package dependencies

## Decisions Made

- `tokio-rusqlite` added to `airev/Cargo.toml` — was in `[workspace.dependencies]` but not referenced from the binary crate (compiler error E0433); auto-fixed inline
- Git repo discovery (Step 5) moved before DB open (Step 6) to make `repo_path` available for `detect_or_create_session()` — requires knowing the repo before creating/resuming a session
- `event_tx` stored in AppState immediately after `EventHandler::new()` and before `spawn_event_task()` — ensures the sender is available before any events can be produced

## Deviations from Plan

### Auto-fixed Issues

**1. [Rule 3 - Blocking] Added tokio-rusqlite to airev/Cargo.toml**
- **Found during:** Task 1 (app.rs changes referencing tokio_rusqlite::Connection)
- **Issue:** `tokio_rusqlite` was present in `[workspace.dependencies]` but not in `[dependencies]` of the `airev` package. Compiler error E0433: "failed to resolve: use of unresolved module or unlinked crate `tokio_rusqlite`"
- **Fix:** Added `tokio-rusqlite = { workspace = true }` to `airev/Cargo.toml`
- **Files modified:** `airev/Cargo.toml`
- **Verification:** `cargo build -p airev` succeeded after fix
- **Committed in:** 9ec7c95 (Task 1 commit)

---

**Total deviations:** 1 auto-fixed (Rule 3 blocking issue)
**Impact on plan:** Required for compilation. No scope creep.

## Issues Encountered

None beyond the one auto-fixed deviation. The TUI cannot be run without a real TTY (crossterm "Device not configured" in headless environments), but workspace build succeeded cleanly with zero errors.

## User Setup Required

None — session lifecycle runs automatically on next real TUI launch in a git repository.

## Next Phase Readiness

- `db_conn`, `session`, `file_review_states`, `event_tx` all wired into `AppState`
- `current_file_path()` ready for keybindings.rs `r` key handler in Plan 03
- `DbResult` event loop arm ready to process `ReviewToggled` payloads from Plan 03
- Full workspace builds with zero errors

## Self-Check: PASSED

- event.rs DbResultPayload: FOUND
- app.rs db_conn field: FOUND
- main.rs detect_or_create_session call: FOUND
- main.rs DbResult match arm: FOUND
- layout.rs session info: FOUND
- Task commit 9ec7c95: verified below
- Task commit 3950815: verified below

---
*Phase: 04-persistence-layer*
*Completed: 2026-02-19*
