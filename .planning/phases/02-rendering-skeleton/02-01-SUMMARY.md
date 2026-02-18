---
phase: 02-rendering-skeleton
plan: "01"
subsystem: ui
tags: [rust, ratatui, ratatui-0.30, crossterm, tui, layout, appstate, scroll]

# Dependency graph
requires:
  - phase: 01-03
    provides: ui::render() placeholder, main.rs event loop with single terminal.draw()
  - phase: 01-02
    provides: tui.rs terminal lifecycle, event.rs AppEvent bus

provides:
  - airev/src/app.rs with AppState, Mode, PanelFocus and all scroll/panel-resize methods
  - airev/src/ui/layout.rs with compute_layout(), panel_block(), render_status_bar()
  - airev/src/ui/mod.rs with render(frame, &mut AppState, &Theme) replacing ui.rs
  - main.rs AppState instantiation and updated draw() call
  - Responsive 3-panel layout collapsing side panels below 120 columns
  - Status bar always showing NORMAL/INSERT mode indicator
  - Placeholder file-list, diff, comments content with scroll support

affects:
  - 02-02: keybindings plan reads PanelFocus.prev()/next() and all AppState scroll methods
  - 02-03: help overlay plan adds render_help_overlay() at the TODO comment in ui/mod.rs
  - 03: Git layer replaces placeholder diff content in render_diff()

# Tech tracking
tech-stack:
  added: []  # All in workspace already — ratatui 0.30, crossterm 0.29
  patterns:
    - Spacing::Overlap(1) + Block::merge_borders(MergeStrategy::Fuzzy) for shared border collapsing
    - Rect::layout(&Layout) returning [Rect; N] typed array (ratatui 0.30 pattern)
    - Manual u16 scroll offset via Paragraph::scroll((y, 0)) for non-List panels
    - ListState for file-list panel (stateful widget with built-in bounds clamping)
    - Viewport heights cached in AppState after each render, used at next keypress
    - Side panels skipped (not rendered) when area.width == 0 to avoid border artifacts

key-files:
  created:
    - airev/src/app.rs
    - airev/src/ui/layout.rs
    - airev/src/ui/mod.rs
  modified:
    - airev/src/main.rs
  deleted:
    - airev/src/ui.rs

key-decisions:
  - "MergeStrategy::Fuzzy used instead of Exact — Fuzzy handles Thick+Plain border junctions better (pitfall 5 from research)"
  - "ListState::scroll_down_by/scroll_up_by take u16 not usize in ratatui 0.30 (research doc showed usize cast; actual API differs)"
  - "Side panels skipped entirely (not rendered) when area.width == 0 — prevents stray border artifact from collapsed Constraint::Length(0)"
  - "panel_block renders Block with merge_borders separately; paragraph rendered on inner Rect — avoids double-border issue"

patterns-established:
  - "Pattern 10: compute_layout() returns [Rect; 4] typed array — always called inside terminal.draw() for fresh terminal dimensions"
  - "Pattern 11: AppState viewport heights cached before panel renders — one-frame lag is imperceptible, avoids double-draw"
  - "Pattern 12: if area.width > 0 guard before rendering collapsed panels — prevents border artifacts from zero-width Rects"

requirements-completed:
  - Layout

# Metrics
duration: 4min
completed: 2026-02-18
---

# Phase 02 Plan 01: AppState and Responsive 3-Panel Layout Summary

**ratatui 0.30 3-panel layout with Spacing::Overlap(1) border merging, AppState with 14 scroll methods, and responsive collapse below 120 columns wired into a running airev binary**

## Performance

- **Duration:** ~4 min
- **Started:** 2026-02-18T07:31:52Z
- **Completed:** 2026-02-18T07:35:52Z
- **Tasks:** 2
- **Files modified:** 5 (3 created, 1 modified, 1 deleted)

## Accomplishments
- `app.rs` defines `AppState`, `Mode`, `PanelFocus` with full scroll method suite (14 methods: scroll_down/up, top/bottom, half/full page, prev/next file, prev/next hunk, shrink/grow diff panel)
- `ui/layout.rs` provides `compute_layout()` returning `[Rect; 4]` with `Spacing::Overlap(1)` for border merging, `panel_block()` with Thick/Plain borders and `MergeStrategy::Fuzzy`, and `render_status_bar()` always showing NORMAL/INSERT
- `ui/mod.rs` replaces `ui.rs` — `render(frame, &mut AppState, &Theme)` caches viewport heights before rendering panels, skips collapsed panels (width == 0), renders placeholder List and Paragraph content with live scroll offsets
- `main.rs` instantiates `AppState::default()` and passes `&mut state` to `ui::render()` inside `terminal.draw()`
- `cargo build -p airev` exits 0; binary shows 3-panel TUI with NORMAL status bar

## Task Commits

Each task was committed atomically:

1. **Task 1: app.rs — AppState, Mode, PanelFocus, all scroll methods** - `abcb37a` (feat)
2. **Task 2: ui/layout.rs + ui/mod.rs + main.rs** - `2c9494d` (feat)

**Plan metadata:** (docs commit follows)

## Files Created/Modified
- `airev/src/app.rs` - AppState struct with Mode/PanelFocus enums, manual Default impl (20/55/25 pct), 14 scroll/resize methods
- `airev/src/ui/layout.rs` - compute_layout(), inner_rect(), panel_block(), render_status_bar() — pure layout arithmetic, no mutable state
- `airev/src/ui/mod.rs` - render() entry point, viewport height caching, file-list/diff/comments panel renderers, placeholder content builders
- `airev/src/main.rs` - Added mod app; instantiated AppState::default(); updated terminal.draw() call to ui::render(frame, &mut state, &theme)
- `airev/src/ui.rs` - DELETED (replaced by ui/ module directory)

## Decisions Made
- Used `MergeStrategy::Fuzzy` instead of `Exact` — research pitfall 5 documents that `Exact` falls back to `Replace` for missing Thick+Plain junctions; `Fuzzy` applies approximation rules for better visual output.
- Side panels are not rendered at all when `area.width == 0` (collapsed). Rendering a zero-width `Block` would produce border artifacts per research pitfall 4.
- `panel_block()` renders the `Block` to the outer area and the `Paragraph` to the inner rect explicitly — this is the correct pattern for `Paragraph` content inside a `Block` when separate widget calls are needed.

## Deviations from Plan

### Auto-fixed Issues

**1. [Rule 1 - Bug] Fixed ListState::scroll_down_by/scroll_up_by type: u16 not usize**
- **Found during:** Task 2 (build check after Task 1 commit)
- **Issue:** The research doc's pattern showed `file_list_state.scroll_down_by(lines as usize)` and `scroll_up_by(lines as usize)`, but ratatui 0.30's actual `ListState` API uses `u16` as the parameter type, not `usize`. This caused E0308 type mismatch errors.
- **Fix:** Changed `lines as usize` to `lines` directly (the `lines` parameter is already `u16`).
- **Files modified:** `airev/src/app.rs`
- **Verification:** `cargo build -p airev` exits 0 after the fix.
- **Committed in:** `2c9494d` (Task 2 commit, app.rs included in stage)

---

**Total deviations:** 1 auto-fixed (1 Rule 1 type error bug)
**Impact on plan:** Required for compilation. The fix is minimal — removing an incorrect type cast. No scope creep.

## Issues Encountered
None beyond the ListState API type mismatch documented above.

## User Setup Required
None - no external service configuration required.

## Next Phase Readiness
- `cargo build -p airev` exits 0 (warnings only — expected dead code for unused Phase 2+ variants)
- `cargo run -p airev` launches a 3-panel TUI with NORMAL status bar; 'q' exits cleanly
- Plan 02-02 (keybindings) can proceed: `AppState` scroll methods and `PanelFocus.prev()/next()` are ready
- Plan 02-03 (help overlay) can proceed: the `// TODO: render_help_overlay(frame, theme)` comment marks the injection point in `ui/mod.rs`

## Self-Check: PASSED

- FOUND: airev/src/app.rs
- FOUND: airev/src/ui/layout.rs
- FOUND: airev/src/ui/mod.rs
- CONFIRMED: airev/src/ui.rs deleted
- FOUND commit abcb37a (Task 1: app.rs)
- FOUND commit 2c9494d (Task 2: ui module + main.rs)
- cargo build -p airev exits 0 (7 warnings, 0 errors)

---
*Phase: 02-rendering-skeleton*
*Completed: 2026-02-18*
