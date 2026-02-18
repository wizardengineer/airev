# Project State

## Current Status
- **Phase:** 01-foundation (executing)
- **Milestone:** 1 (MVP)
- **Last updated:** 2026-02-18
- **Stopped At:** Completed 01-02-PLAN.md

## Completed
- [x] Project initialized (`PROJECT.md`)
- [x] Config saved (`config.json`)
- [x] Research complete (ARCHITECTURE, STACK, FEATURES, PITFALLS, SUMMARY)
- [x] Requirements defined (`REQUIREMENTS.md`)
- [x] Roadmap created (`ROADMAP.md`)
- [x] Phase 1 planned (4 plans)
- [x] Phase 1, Plan 01: Cargo workspace + airev-core types and WAL DB initialization
- [x] Phase 1, Plan 02: tui.rs terminal lifecycle + event.rs unified event bus

## Next Step
Execute plan 03 of phase 01-foundation (`01-03-PLAN.md`).

## Phase Progress
| Phase | Name | Status |
|-------|------|--------|
| 1 | Foundation | in-progress (2/4 plans done) |
| 2 | Rendering Skeleton | pending |
| 3 | Git Layer | pending |
| 4 | Persistence Layer | pending |
| 5 | Comment UI | pending |
| 6 | Live File Watcher | pending |
| 7 | MCP Server | pending |
| 8 | Polish and Compatibility | pending |

## Key Decisions Locked
- Two-binary architecture: `airev` (TUI) + `airev-mcp` (MCP server) sharing SQLite WAL-mode DB
- TUI renders to stderr; MCP stdio owns stdin/stdout
- git2 (not gix) — dedicated background thread owns Repository (not-Send constraint)
- SQLite WAL mode + `BEGIN IMMEDIATE` for all writes
- ratatui 0.30 + crossterm 0.29 + tokio 1.49
- rmcp 0.16 for MCP server (official SDK)
- single `terminal.draw()` per frame (never call twice)
- Multi-round thread schema in SQLite from day one
- rusqlite pinned to 0.37 (tokio-rusqlite 0.7 requires ^0.37; workspace uses 0.37)
- All rusqlite access via tokio_rusqlite::Connection::call() closures
- busy_timeout set via Connection method (not PRAGMA string)
- No Drop impl on Tui — ratatui 0.30 does not auto-restore terminal on Drop (GitHub #2087); restore_tui() called explicitly at every exit path
- AppEvent marked #[non_exhaustive] — future variants added without breaking exhaustive match arms in existing handlers
- EventHandler uses unbounded mpsc channel — producer (terminal + timers) bounded by hardware rate, consumer (main loop) always keeps up

## Performance Metrics
| Phase | Plan | Duration | Tasks | Files |
|-------|------|----------|-------|-------|
| 01-foundation | 01 | 2min | 2 | 11 |
| 01-foundation | 02 | 2min | 2 | 3 |

