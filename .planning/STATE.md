# Project State

## Current Status
- **Phase:** 03-git-layer (executing)
- **Milestone:** 1 (MVP)
- **Last updated:** 2026-02-18
- **Stopped At:** Completed 03-01-PLAN.md (Cargo deps + owned git types + AppEvent::GitResult payload); next: Phase 3 Plan 02 (AsyncGit worker thread)

## Completed
- [x] Project initialized (`PROJECT.md`)
- [x] Config saved (`config.json`)
- [x] Research complete (ARCHITECTURE, STACK, FEATURES, PITFALLS, SUMMARY)
- [x] Requirements defined (`REQUIREMENTS.md`)
- [x] Roadmap created (`ROADMAP.md`)
- [x] Phase 1 planned (4 plans)
- [x] Phase 1, Plan 01: Cargo workspace + airev-core types and WAL DB initialization
- [x] Phase 1, Plan 02: tui.rs terminal lifecycle + event.rs unified event bus
- [x] Phase 1, Plan 03: main.rs event loop + ui.rs blank 3-panel layout — running airev binary
- [x] Phase 2, Plan 01: AppState + Mode + PanelFocus + responsive 3-panel layout engine + render()
- [x] Phase 2, Plan 02: vim keybinding dispatcher (handle_key) + modal help overlay (render_help_overlay)
- [x] Phase 2, Plan 03: main.rs wiring (handle_key in AppEvent::Key arm) + human verification of full interactive TUI
- [x] Phase 3, Plan 01: Cargo deps (git2/crossbeam-channel/syntect/syntect-tui/similar) + owned git types (OwnedDiffHunk, OwnedDiffLine, FileSummary, DiffMode, GitRequest, GitResultPayload) + AppEvent::GitResult(Box<GitResultPayload>)

## Next Step
Execute Phase 3: Git Layer (`03-git-layer`). Continue with plan 02 (AsyncGit worker thread).

## Phase Progress
| Phase | Name | Status |
|-------|------|--------|
| 1 | Foundation | in-progress (3/4 plans done) |
| 2 | Rendering Skeleton | complete (3/3 plans done) |
| 3 | Git Layer | in-progress (1/? plans done) |
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
- map_err(|e| std::io::Error::new(ErrorKind::Other, e)) wraps tokio_rusqlite::Error for ? in main() -> std::io::Result<()>
- 50ms tokio::select! heartbeat arm for SIGTERM polling — prevents rx.recv() blocking indefinitely on quiescent terminal
- Theme struct: all 18 color fields defined in Phase 1 (border_inactive used now; border_active and diff/badge/status colors reserved for Phase 2+)
- Theme::from_name() graceful fallback to dark() on unknown names — config errors are soft stderr prints, never panics
- toml 0.8 workspace dep (not 1.x) — compatible with Rust 1.89.0 MSRV; theme config loaded as Step 0 before terminal init (read-only, safe)
- MergeStrategy::Fuzzy used instead of Exact for border merging — handles Thick+Plain border junctions that Exact cannot merge cleanly
- ListState::scroll_down_by/scroll_up_by take u16 not usize in ratatui 0.30 (research doc showed wrong type cast)
- Side panels skipped entirely (not rendered) when area.width == 0 — prevents border artifacts from collapsed Constraint::Length(0)
- handle_scroll_key() private helper splits j/k/g/G/Ctrl combos out of handle_normal() to stay under 50 lines per function
- Ctrl-H/L focus bindings NOT implemented — Ctrl-H conflicts with Backspace on most terminals; uppercase H/L is the only focus keybinding
- frame.area().width < 60 guard in render_help_overlay() prevents zero-height Rect panic on narrow terminals (Pitfall 6)
- No color styling on help overlay text body — theme colors for help content reserved for Phase 5+ polish
- main.rs contains no key-handling logic — all key logic lives in ui/keybindings.rs dispatched via handle_key(); event loop is a thin dispatcher
- AppEvent::Resize immediately sends AppEvent::Render for immediate relayout rather than waiting for next tick interval
- Remove Clone derive from AppEvent — GitResultPayload is not Clone (moved into AppState on receipt, never cloned)
- syntect default-fancy feature uses pure-Rust fancy-regex instead of Oniguruma C library — no C build dependency
- All types crossing thread boundary are fully owned (no lifetimes) — String/u32/char/Vec/usize ensure Send without unsafe
- Box<GitResultPayload> in AppEvent keeps enum variant pointer-sized on channel (payload can be large)

### Quick Tasks Completed

| # | Description | Date | Commit | Directory |
|---|-------------|------|--------|-----------|
| 1 | automated self-verification before human checkpoint and theme system for airev | 2026-02-18 | 153858c | [1-automated-self-verification-before-human](./quick/1-automated-self-verification-before-human/) |

## Performance Metrics
| Phase | Plan | Duration | Tasks | Files |
|-------|------|----------|-------|-------|
| 01-foundation | 01 | 2min | 2 | 11 |
| 01-foundation | 02 | 2min | 2 | 3 |
| 01-foundation | 03 | 2min | 2 | 2 |
| quick | 01 | 3min | 2 | 7 |
| 02-rendering-skeleton | 01 | 4min | 2 | 5 |
| 02-rendering-skeleton | 02 | 2min | 2 | 3 |
| 02-rendering-skeleton | 03 | 20min | 2 | 1 |
| 03-git-layer | 01 | 3min | 2 | 7 |

