---
phase: 01-foundation
plan: "02"
subsystem: ui
tags: [rust, ratatui, crossterm, tokio, signal-hook, tui, event-bus, stderr]

# Dependency graph
requires:
  - phase: 01-01
    provides: Cargo workspace with airev crate, crossterm/ratatui/tokio/signal-hook/futures in workspace.dependencies
provides:
  - airev::tui: Tui type alias, init_tui(), restore_tui(), install_panic_hook(), register_sigterm()
  - airev::event: AppEvent enum (8 variants), EventHandler struct, spawn_event_task()
  - Terminal lifecycle on stderr (not stdout) with explicit restore at every exit path
  - Panic hook that calls restore_tui() before the original hook so terminal is usable on panic
  - SIGTERM detection via Arc<AtomicBool> from signal_hook::flag::register
  - Unified event bus with two independent intervals: 250ms tick (4Hz) and 33ms render (~30FPS)
affects:
  - 01-03: Git layer (spawns background thread, sends GitResult events via EventHandler.tx)
  - 01-04: Persistence layer (sends DbResult events via EventHandler.tx)
  - 02: Rendering skeleton (uses Tui type alias, calls init_tui/restore_tui, drives render loop from AppEvent::Render)

# Tech tracking
tech-stack:
  added: []  # All dependencies were already present from 01-01 (crossterm, ratatui, tokio, signal-hook, futures)
  patterns:
    - Terminal renders to stderr (BufWriter<Stderr>) — stdout reserved for MCP JSON-RPC
    - restore_tui() called explicitly at every exit path — no Drop impl (ratatui 0.30 GitHub #2087)
    - Panic hook chains: restore_tui() → original hook (terminal usable when panic message prints)
    - Two independent tokio::time::interval timers for render (33ms) and logic (250ms)
    - KeyEventKind::Press filter in event task prevents double-fire on Windows
    - reader.next().fuse() prevents polling completed EventStream future in tokio::select!
    - AppEvent marked #[non_exhaustive] so new variants don't break existing match arms

key-files:
  created:
    - airev/src/tui.rs
    - airev/src/event.rs
  modified:
    - airev/src/main.rs  # Added mod tui; mod event;

key-decisions:
  - "No Drop impl on Tui — ratatui 0.30 does not auto-restore terminal on Drop (issue #2087); restore_tui() called explicitly at every exit path"
  - "EventHandler uses unbounded mpsc channel — producer (terminal + timers) is bounded by hardware rate, consumer (main loop) always keeps up"
  - "AppEvent marked #[non_exhaustive] — future variants (LSP diagnostics, AI tokens) added without breaking exhaustive match arms in existing handlers"
  - "reader.next().fuse() required to prevent tokio::select! from polling a completed EventStream future"

patterns-established:
  - "Pattern 4: TUI writes to stderr (BufWriter<Stderr>), never stdout — stdout stays clean for piping and MCP JSON-RPC"
  - "Pattern 5: install_panic_hook() called before init_tui() — panic hook chain: restore_tui() then original hook"
  - "Pattern 6: spawn_event_task() drives two independent intervals — render (33ms) and tick (250ms) tuned separately"
  - "Pattern 7: KeyEventKind::Press filter on all key events before sending to AppEvent::Key"

requirements-completed:
  - Terminal Lifecycle and Safety
  - Event Architecture constraints

# Metrics
duration: 2min
completed: 2026-02-18
---

# Phase 01 Plan 02: Terminal Lifecycle and Event Bus Summary

**stderr-backed ratatui terminal with chained panic hook, SIGTERM AtomicBool flag, and unified tokio event bus with independent 250ms tick and 33ms render intervals**

## Performance

- **Duration:** 2 min
- **Started:** 2026-02-18T06:04:08Z
- **Completed:** 2026-02-18T06:06:05Z
- **Tasks:** 2
- **Files modified:** 3 (2 created, 1 modified)

## Accomplishments
- `tui.rs` implements the complete terminal lifecycle: `init_tui()` on `BufWriter<Stderr>`, `restore_tui()` at every exit path, `install_panic_hook()` chaining `restore_tui()` before the original hook, and `register_sigterm()` returning `Arc<AtomicBool>`
- `event.rs` implements the unified event bus: `AppEvent` with 8 `#[non_exhaustive]` variants, `EventHandler` with unbounded MPSC channel, and `spawn_event_task()` with two independent `tokio::time::interval` timers (250ms tick, 33ms render) and `KeyEventKind::Press` filtering
- All patterns from the must_haves verified: stderr backend confirmed, no Drop impl on Tui, two distinct interval durations, `.fuse()` on EventStream, Press-only key filter

## Task Commits

Each task was committed atomically:

1. **Task 1: tui.rs — stderr backend, panic hook, SIGTERM** - `6f32d2e` (feat)
2. **Task 2: event.rs — AppEvent enum, EventHandler, spawn_event_task** - `fc111b1` (feat)

**Plan metadata:** (docs commit follows)

## Files Created/Modified
- `airev/src/tui.rs` - Terminal lifecycle: Tui type alias, init_tui(), restore_tui(), install_panic_hook(), register_sigterm()
- `airev/src/event.rs` - Event bus: AppEvent enum (8 variants), EventHandler struct with unbounded channel, spawn_event_task() with dual intervals
- `airev/src/main.rs` - Added `mod event; mod tui;` module declarations

## Decisions Made
- No `Drop` impl on `Tui` — ratatui 0.30 does not auto-restore the terminal on Drop (GitHub issue #2087). Explicit `restore_tui()` calls at every exit path in main are the correct approach per plan specification.
- `EventHandler` uses unbounded `mpsc` channel — producer side (terminal events + timers) generates events at a bounded hardware rate and the consumer (main loop) always keeps up. No backpressure needed at this scale.
- `AppEvent` marked `#[non_exhaustive]` — allows future variants (e.g., LSP diagnostics, AI streaming tokens) to be added in later phases without breaking exhaustive match arms in existing handlers.

## Deviations from Plan

None - plan executed exactly as written.

## Issues Encountered
None — both modules compiled on first attempt with warnings only (dead code, expected since main.rs stub does not yet call the functions).

## User Setup Required
None - no external service configuration required. All dependencies were already present from plan 01-01.

## Next Phase Readiness
- `airev::tui` and `airev::event` modules exported and ready for use
- `cargo build --workspace` exits 0 with zero errors
- Plan 01-03 (Git Layer) can proceed: `EventHandler.tx` is available to send `AppEvent::GitResult` from the background git thread
- Plan 01-04 (Persistence Layer) can proceed: `EventHandler.tx` is available to send `AppEvent::DbResult` from async DB tasks
- Phase 02 (Rendering Skeleton) can proceed: `Tui` type, `init_tui()`, `restore_tui()`, and `AppEvent::Render` are all in place

## Self-Check: PASSED

- FOUND: airev/src/tui.rs
- FOUND: airev/src/event.rs
- FOUND: .planning/phases/01-foundation/01-02-SUMMARY.md
- FOUND commit 6f32d2e (Task 1: tui.rs)
- FOUND commit fc111b1 (Task 2: event.rs)
- cargo build --workspace exits 0 (warnings only — dead code expected)

---
*Phase: 01-foundation*
*Completed: 2026-02-18*
