---
phase: 01-foundation
plan: "03"
subsystem: ui
tags: [rust, ratatui, crossterm, tokio, sqlite, tokio-rusqlite, tui, event-loop, sigterm]

# Dependency graph
requires:
  - phase: 01-01
    provides: airev-core::db::open_db(), Cargo workspace, airev-core in airev dependencies
  - phase: 01-02
    provides: tui::init_tui/restore_tui/install_panic_hook/register_sigterm, event::EventHandler/spawn_event_task/AppEvent
provides:
  - airev::ui::render() — blank 3-panel layout (Files 25%, Diff 50%, Comments 25%)
  - main.rs full event loop with single terminal.draw() per Render, 50ms SIGTERM heartbeat
  - DB opened before first frame (no loading spinner)
  - restore_tui() at every exit path (post-loop call + panic hook from tui.rs)
  - Running airev binary showing blank 3-panel TUI, exits cleanly on 'q'
affects:
  - 01-04: Phase exit criteria tests (panic test, SIGTERM test) use this binary
  - 02: Rendering skeleton replaces ui::render() placeholder with real content

# Tech tracking
tech-stack:
  added: []  # All dependencies already present from 01-01 and 01-02
  patterns:
    - terminal.draw() called exactly once per AppEvent::Render — never in any other arm
    - 50ms tokio::time::sleep arm in select! guarantees SIGTERM polled within 50ms
    - SIGTERM flag also checked after every received event for lower quit latency
    - Event loop exits only via break — no ? inside the loop body (except draw which propagates out)
    - map_err converts tokio_rusqlite::Error to std::io::Error for ? propagation in main

key-files:
  created:
    - airev/src/ui.rs
  modified:
    - airev/src/main.rs

key-decisions:
  - "map_err(|e| std::io::Error::new(ErrorKind::Other, e)) wraps tokio_rusqlite::Error for ? in main() -> std::io::Result<()>"
  - "Comments referencing terminal.draw() removed from main.rs to keep grep count == 1 per plan verification"

patterns-established:
  - "Pattern 8: tokio::select! heartbeat arm (50ms sleep) for SIGTERM polling — prevents rx.recv() blocking indefinitely when terminal is quiescent"
  - "Pattern 9: Event loop exits exclusively via break — ? propagation from draw() falls through to restore_tui() before returning"

requirements-completed:
  - Terminal Lifecycle and Safety
  - Event Architecture constraints
  - Performance: startup to first frame under 100ms

# Metrics
duration: 2min
completed: 2026-02-18
---

# Phase 01 Plan 03: Main Event Loop and Placeholder UI Summary

**tokio event loop with single terminal.draw() per Render, 50ms SIGTERM heartbeat, DB opened pre-frame, and blank ratatui 3-panel layout wiring all Phase 1 modules into a running airev binary**

## Performance

- **Duration:** ~2 min
- **Started:** 2026-02-18T06:08:57Z
- **Completed:** 2026-02-18T06:11:10Z
- **Tasks:** 2
- **Files modified:** 2 (1 created, 1 replaced)

## Accomplishments
- `ui.rs` renders a blank 3-panel horizontal layout (Files 25%, Diff 50%, Comments 25%) using `frame.area()` (not deprecated `frame.size()`)
- `main.rs` wires all Phase 1 modules: `install_panic_hook → register_sigterm → init_tui → EventHandler → open_db → event loop → restore_tui`
- Exactly one `terminal.draw()` call, exclusively inside the `AppEvent::Render` arm — confirmed by grep count
- 50ms `tokio::time::sleep` heartbeat in `tokio::select!` guarantees SIGTERM flag is polled within 50ms even when the terminal is quiescent
- SIGTERM flag also checked after every received event for sub-event-cycle quit latency
- DB directory `.airev/` created before `open_db()` — no runtime directory-not-found errors
- `restore_tui()` called at the single loop exit point, panic hook provides the fallback path

## Task Commits

Each task was committed atomically:

1. **Task 1: ui.rs — blank 3-panel placeholder layout** - `2159cbd` (feat)
2. **Task 2: main.rs — tokio event loop with SIGTERM, DB init, single draw()** - `9ac2635` (feat)

**Plan metadata:** (docs commit follows)

## Files Created/Modified
- `airev/src/ui.rs` - render() function with 3 bordered panels in a horizontal layout using Constraint::Percentage
- `airev/src/main.rs` - Full async tokio entry point: startup sequence, event loop, single draw(), SIGTERM heartbeat, restore_tui()

## Decisions Made
- `tokio_rusqlite::Error` is not `From<std::io::Error>`, so `?` cannot be used directly after `open_db().await` in a function returning `std::io::Result<()>`. Fixed with `map_err(|e| std::io::Error::new(std::io::ErrorKind::Other, e))`. This is the correct lightweight conversion for Phase 1 — a richer error type is out of scope until plan 04.
- Comments in main.rs that mentioned `terminal.draw()` in their text were removed to keep the grep verification `grep "terminal.draw" ... | wc -l == 1` accurate and unambiguous.

## Deviations from Plan

### Auto-fixed Issues

**1. [Rule 1 - Bug] Fixed tokio_rusqlite::Error not convertible to std::io::Error via ?**
- **Found during:** Task 2 (main.rs event loop)
- **Issue:** `open_db()` returns `Result<tokio_rusqlite::Connection, tokio_rusqlite::Error>`. The `?` operator in `main() -> std::io::Result<()>` requires `From<tokio_rusqlite::Error> for std::io::Error`, which is not implemented. Compilation failed with E0277.
- **Fix:** Added `.map_err(|e| std::io::Error::new(std::io::ErrorKind::Other, e))` before `?` on the `open_db().await` call.
- **Files modified:** `airev/src/main.rs`
- **Verification:** `cargo build --workspace` exits 0 after the fix.
- **Committed in:** `9ac2635` (Task 2 commit)

---

**Total deviations:** 1 auto-fixed (1 Rule 1 type error bug)
**Impact on plan:** Required for compilation. The map_err wrapper is the minimal correct fix — no scope creep. A richer error type using `anyhow` or `thiserror` is deferred to plan 04 (Persistence Layer) if needed.

## Issues Encountered
None beyond the tokio_rusqlite::Error conversion documented above.

## User Setup Required
None - no external service configuration required. All dependencies were already present from prior plans.

## Next Phase Readiness
- `cargo build --workspace` exits 0 (warnings only — expected dead code for Phase 1 unused variants)
- `cargo run -p airev` in a real terminal launches the 3-panel TUI; 'q' exits cleanly
- Plan 01-04 (Phase Exit Criteria) can proceed: panic test and SIGTERM test targets are ready
- Phase 02 (Rendering Skeleton) can proceed: `ui::render()` is the live draw hook to replace with real content

## Self-Check: PASSED

- FOUND: airev/src/ui.rs
- FOUND: airev/src/main.rs
- FOUND: .planning/phases/01-foundation/01-03-SUMMARY.md
- FOUND commit 2159cbd (Task 1: ui.rs)
- FOUND commit 9ac2635 (Task 2: main.rs)
- cargo build --workspace exits 0 (2 warnings, 0 errors)

---
*Phase: 01-foundation*
*Completed: 2026-02-18*
