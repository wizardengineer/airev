# Project State

## Current Status
- **Phase:** 04-persistence-layer (next)
- **Milestone:** 1 (MVP)
- **Last updated:** 2026-02-19
- **Stopped At:** Completed 03-06-PLAN.md (Gap closure — per-file line counts, file_line_offsets jump, status bar file count)

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
- [x] Phase 3, Plan 02: AsyncGit worker thread (git_worker_loop, 4 diff modes, syntect highlighting, word-level diff via similar) + AppState Phase 3 fields (diff_lines, diff_scroll: usize, file_summaries, diff_mode, diff_loading, hunk_offsets, selected_file_index, hunk_cursor)
- [x] Phase 3, Plan 03: ui/diff_view.rs (render_diff with List virtual scroll) + ui/file_tree.rs (render_file_list with FileSummary badges) + ui/mod.rs updated; placeholder builders removed
- [x] Phase 3, Plan 04: AsyncGit wired at startup with git repo detection; AppEvent::GitResult arm; Tab diff mode cycle; Enter/l file-list jump; status bar DiffMode label + loading indicator
- [x] Phase 3, Plan 05: Exit criteria verification — 6 automated pre-checks passed (no unsafe, cargo build 0, no E0277, Repository in worker, usize scroll, bounded slice); human-verified real diff content, 60fps scrolling, all 4 diff modes, file jump, hunk nav, no crashes
- [x] Phase 3, Plan 06: Gap closure — FileSummary.added/removed populated with real counts; file_line_offsets field and index chain added; jump_to_selected_file uses offset lookup; status bar shows file count

## Next Step
Execute Phase 4: Persistence Layer (`04-persistence-layer`). Begin with plan 01.

## Phase Progress
| Phase | Name | Status |
|-------|------|--------|
| 1 | Foundation | in-progress (3/4 plans done) |
| 2 | Rendering Skeleton | complete (3/3 plans done) |
| 3 | Git Layer | complete (5/5 plans done) |
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
- RefCell used in extract_hunks to share hunks Vec between two diff.foreach closures without unsafe — git2 guarantees sequential execution on same thread
- Custom syntect_to_span replaces into_span to resolve ratatui::style::Style vs ratatui::prelude::Style type split in syntect-tui 3.0 / ratatui 0.30
- diff_scroll: u16 → usize in AppState; ui/mod.rs temporary as u16 cast for Paragraph::scroll until Plan 03 introduces List widget
- List widget with manual slice replaces Paragraph::scroll for diff panel — eliminates usize→u16 cast and enables O(viewport) rendering for 5000+ line diffs
- render_file_list takes &mut AppState (for ListState) while render_diff takes &AppState — asymmetric mutability matches ratatui render_stateful_widget requirement
- Comments panel kept as minimal Paragraph placeholder in mod.rs — Phase 5 is the right time to introduce a comments module when real data is wired
- git_tx stored in AppState (Option<Sender<GitRequest>>) so keybindings.rs can send diff mode requests without adding a parameter to handle_key()
- handle_file_list_key() extracted as private helper from handle_normal() to stay under 50 lines per function limit
- git2::Repository::discover('.') from cwd — graceful None if no repo found; diff panel shows "No diff loaded" placeholder without code path changes
- Tab key cycles DiffMode globally (not scoped to FileList focus) so users can switch diff modes from any panel
- Status bar DiffMode label uses Color::DarkGray; loading indicator uses Color::Yellow for visual prominence
- eprintln! removed entirely from git worker — no replacement logging added; empty payload is the correct graceful degradation signal for a TUI where stderr is the terminal backend
- Help overlay reflects only wired keybindings — placeholder descriptions removed as soon as features ship
- help_scroll is u16 (matching Paragraph::scroll API); u16::MAX used for G-key jump — ratatui clamps automatically
- panel_rects cached in AppState every frame so mouse hit-testing always uses the most recent geometry
- Overlap(1) spacing NOT applied to 80-119 layout (Length(0) + Overlap causes u16 underflow in ratatui layout engine)
- Scroll-wheel in HelpOverlay mode routes to help_scroll (not focused-panel scroll) for intuitive overlay navigation
- Phase 3 exit criteria are compiler-enforced (no unsafe, !Send on Repository) plus human visual sign-off — not just test coverage; the compiler's type system is the authoritative thread-safety check for git2
- Automated pre-checks (6 grep/cargo/rustc assertions) run before blocking human verification checkpoints to catch regressions before user time is spent
- extract_files uses diff.foreach() with line_cb to count +/- origins per file (single pass, no second foreach call)
- extract_hunks returns (hunks, file_hunk_starts) where file_hunk_starts[i] is the hunk index at the start of file i
- file_line_offsets = file_hunk_starts mapped through hunk_offsets — no new data structure needed, reuses existing hunk offset table
- jump_to_selected_file falls back to 0 via .get(idx).copied().unwrap_or(0) when no diff loaded yet
- Status bar file count uses same Color::DarkGray as diff mode label; only shown when file_summaries is non-empty

### Quick Tasks Completed

| # | Description | Date | Commit | Directory |
|---|-------------|------|--------|-----------|
| 1 | automated self-verification before human checkpoint and theme system for airev | 2026-02-18 | 153858c | [1-automated-self-verification-before-human](./quick/1-automated-self-verification-before-human/) |
| 2 | fix status bar error spam from git worker; update help overlay keybindings | 2026-02-18 | 1aba9e2 | [2-fix-status-bar-error-spam-from-git-worke](./quick/2-fix-status-bar-error-spam-from-git-worke/) |
| 3 | fix narrow terminal layout: scrollable help overlay, 3-breakpoint layout, mouse support | 2026-02-18 | addd475 | [3-fix-narrow-terminal-layout-panels-cut-of](./quick/3-fix-narrow-terminal-layout-panels-cut-of/) |

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
| 03-git-layer | 02 | 5min | 3 | 4 |
| 03-git-layer | 03 | 2min | 2 | 4 |
| 03-git-layer | 04 | 2min | 2 | 4 |
| quick | 02 | 2min | 2 | 2 |
| quick | 03 | 6min | 2 | 8 |
| 03-git-layer | 05 | 5min | 2 | 0 |
| 03-git-layer | 06 | 2min | 3 | 4 |

