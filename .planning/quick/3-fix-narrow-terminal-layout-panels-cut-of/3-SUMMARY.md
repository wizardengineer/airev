---
phase: quick
plan: 3
subsystem: tui-layout-mouse
tags: [layout, mouse, help-overlay, responsive, ratatui]
dependency_graph:
  requires: [Phase 3 Plan 04]
  provides: [scrollable-help, 3-breakpoint-layout, mouse-input]
  affects: [ui/layout.rs, ui/help.rs, ui/keybindings.rs, event.rs, tui.rs, app.rs, main.rs, ui/mod.rs]
tech_stack:
  added: []
  patterns:
    - Paragraph::scroll for scrollable help overlay text
    - crossterm EnableMouseCapture/DisableMouseCapture for mouse events
    - Rect::contains(Position) for panel hit-testing
    - 3-breakpoint responsive layout (>=120, 80-119, <80)
key_files:
  created: []
  modified:
    - airev/src/app.rs
    - airev/src/ui/help.rs
    - airev/src/ui/layout.rs
    - airev/src/ui/keybindings.rs
    - airev/src/ui/mod.rs
    - airev/src/event.rs
    - airev/src/tui.rs
    - airev/src/main.rs
decisions:
  - help_scroll is u16 (matching Paragraph::scroll API); u16::MAX used for G-key jump — ratatui clamps automatically
  - panel_rects cached in AppState every frame (not stored across frames from layout) so mouse hit-testing uses the most recent geometry
  - Overlap(1) spacing NOT applied to 80-119 layout (Length(0) + Overlap causes u16 underflow in ratatui layout engine)
  - handle_mouse returns KeyAction but Mouse arm in main.rs ignores the return value (mouse events never quit)
  - Scroll-wheel in HelpOverlay mode routes to help_scroll (not focused-panel scroll) for intuitive behavior
metrics:
  duration: 6min
  completed: 2026-02-18
  tasks: 2
  files_modified: 8
---

# Quick 3: Fix Narrow Terminal Layout — Scrollable Help, 3-Breakpoint Layout, Mouse Support

One-liner: Scrollable help overlay with j/k/g/G, 3-breakpoint responsive layout (>=120/80-119/<80), and full mouse support (click-to-focus + scroll wheel) via crossterm MouseCapture.

## Tasks Completed

| # | Name | Commit | Files |
|---|------|--------|-------|
| 1 | Scrollable help overlay + 3-breakpoint responsive layout | e25fba2 | app.rs, ui/help.rs, ui/layout.rs, ui/keybindings.rs, ui/mod.rs |
| 2 | Mouse support (click-to-focus, scroll wheel) | addd475 | tui.rs, event.rs, app.rs, ui/keybindings.rs, ui/mod.rs, main.rs |

## What Was Built

### Task 1: Scrollable Help Overlay + 3-Breakpoint Layout

**`app.rs`:** Added `pub help_scroll: u16` field to `AppState` (default 0). This field is the scroll offset for the help overlay `Paragraph` and is reset to 0 whenever the user opens the overlay.

**`ui/help.rs`:** Changed `render_help_overlay` signature to accept `help_scroll: u16`. Added `.scroll((help_scroll, 0))` to the `Paragraph` chain. Updated the block title to `" Help  — j/k scroll, ? or Esc to dismiss "`. Added a help text entry documenting `j / k` scroll within the overlay.

**`ui/keybindings.rs`:** In `handle_help()`, added j/k/g/G arms before dismiss keys. In `handle_normal()`, the `?` arm now sets `state.help_scroll = 0` before entering HelpOverlay mode so the overlay always opens at the top.

**`ui/layout.rs`:** Expanded the 2-way branch in `compute_layout` into a 3-way branch:
- `>= 120 cols`: Full 3-panel with `Spacing::Overlap(1)` (unchanged)
- `80..=119 cols`: File list 25% + Diff Fill(1) + Comments Length(0) — no Overlap (avoids u16 underflow with Length(0))
- `< 80 cols`: Diff-only (unchanged)

**`ui/mod.rs`:** Updated the `render_help_overlay` call to pass `state.help_scroll`.

### Task 2: Mouse Support

**`tui.rs`:** Added `EnableMouseCapture` to `init_tui` and `DisableMouseCapture` to `restore_tui`.

**`event.rs`:** Added `Mouse(MouseEvent)` variant to `AppEvent`. Added `Event::Mouse(mouse)` arm in `spawn_event_task` to forward mouse events onto the channel.

**`app.rs`:** Added `pub panel_rects: [Rect; 3]` field (default `[Rect::default(); 3]`) for mouse hit-testing.

**`ui/mod.rs`:** After `compute_layout`, stores `state.panel_rects = [left, center, right]` so the most recent panel geometry is available for hit-testing.

**`ui/keybindings.rs`:** Added public `handle_mouse` function plus three private helpers:
- `handle_mouse_click`: uses `Rect::contains(Position { x: col, y: row })` to set `PanelFocus`
- `handle_mouse_scroll_up`: scrolls `help_scroll` by 3 in HelpOverlay mode, else calls `state.scroll_up(3)`
- `handle_mouse_scroll_down`: scrolls `help_scroll` by 3 in HelpOverlay mode, else calls `state.scroll_down(3)`

**`main.rs`:** Imported `handle_mouse`. Added `AppEvent::Mouse(mouse)` arm in the event loop.

## Deviations from Plan

None — plan executed exactly as written.

## Self-Check

**Files exist:**
- `airev/src/app.rs` — contains `help_scroll`, `panel_rects`
- `airev/src/ui/help.rs` — contains `.scroll(`
- `airev/src/ui/layout.rs` — contains `80`
- `airev/src/event.rs` — contains `Mouse`
- `airev/src/tui.rs` — contains `MouseCapture`

**Commits exist:**
- e25fba2 — Task 1
- addd475 — Task 2

## Self-Check: PASSED
