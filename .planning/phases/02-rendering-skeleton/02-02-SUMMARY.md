---
phase: 02-rendering-skeleton
plan: "02"
subsystem: ui

tags: [ratatui, crossterm, keybindings, vim, modal, help-overlay]

# Dependency graph
requires:
  - phase: 02-01
    provides: AppState mutations (scroll_down/up/top/bottom, half/full_page, prev/next_file/hunk, shrink/grow_diff_panel), Mode and PanelFocus enums, render() entry point with TODO help overlay comment
provides:
  - handle_key() keybinding dispatcher routing by Mode (Normal / HelpOverlay / ConfirmQuit / Insert)
  - KeyAction enum (Continue / Quit) as control-flow signal for the event loop
  - All vim navigation bindings: j/k/g/G, Ctrl-d/u/f/b, H/L focus, {/} file nav, [/] hunk nav, </> panel resize
  - render_help_overlay() modal using Clear + Rect::centered (80x80%) + bordered Paragraph
  - Conditional help overlay call in render() guarded by state.mode == Mode::HelpOverlay
affects:
  - 02-03 (event loop wires handle_key into terminal key events; help overlay tested end-to-end)
  - 05 (Insert mode handler placeholder wired here; Phase 5 adds comment text editing)

# Tech tracking
tech-stack:
  added: []
  patterns:
    - "handle_scroll_key() private helper keeps handle_normal() under 50 lines by splitting scroll keys out"
    - "frame.render_widget(Clear, area) erases background before modal content — single-draw modal pattern"
    - "Narrow-terminal guard (width < 60) prevents zero-height Rect panic when computing centered overlay"
    - "KeyAction enum returned from dispatcher — clean separation between state mutation and event-loop flow"

key-files:
  created:
    - airev/src/ui/keybindings.rs
    - airev/src/ui/help.rs
  modified:
    - airev/src/ui/mod.rs

key-decisions:
  - "handle_scroll_key() private helper splits j/k/g/G/Ctrl combos out of handle_normal() to stay under 50 lines per function"
  - "No SHIFT guard needed for uppercase H/L — crossterm encodes uppercase directly in KeyCode::Char('H')"
  - "Ctrl-h/l for focus NOT implemented — Ctrl-H conflicts with Backspace on most terminals; H/L uppercase is the only binding"
  - "frame.area().width < 60 guard in render_help_overlay() prevents Pitfall 6 zero-height Rect on narrow terminals"
  - "No color styling on help text body — theme colors for overlay content reserved for Phase 5+ polish"
  - "Mode imported in ui/mod.rs for HelpOverlay conditional (was not previously imported in that file)"

patterns-established:
  - "Dispatcher pattern: handle_key() branches on state.mode, each mode has its own private handler fn"
  - "Modal overlay pattern: Clear widget erases background, then Block+Paragraph draws on top in same draw closure"
  - "Narrow-terminal guard pattern: check frame.area().width before computing centered Rect"

requirements-completed:
  - Navigation and Keybindings
  - Help Overlay

# Metrics
duration: 2min
completed: "2026-02-18"
---

# Phase 02 Plan 02: Keybinding Dispatcher and Modal Help Overlay Summary

**vim keybinding dispatcher (handle_key) with mode-branching + Clear-based centred help overlay modal wired into render()**

## Performance

- **Duration:** 2 min
- **Started:** 2026-02-18T07:39:52Z
- **Completed:** 2026-02-18T07:41:59Z
- **Tasks:** 2
- **Files modified:** 3

## Accomplishments

- Full vim keybinding dispatcher in `ui/keybindings.rs`: j/k/g/G scroll, Ctrl-d/u/f/b half/full page, H/L focus cycling, {/} file nav, [/] hunk nav, </> panel resize, ? help, q/Esc quit with unsaved-comment guard
- `render_help_overlay()` in `ui/help.rs` using the single-draw `Clear + Rect::centered + Paragraph` modal pattern with narrow-terminal protection
- `ui/mod.rs` updated with `pub mod help`, `pub mod keybindings`, `Mode` import, and conditional `help::render_help_overlay()` call inside `render()`

## Task Commits

Each task was committed atomically:

1. **Task 1: ui/keybindings.rs — handle_key() dispatch with all vim bindings** - `ca7bf11` (feat)
2. **Task 2: ui/help.rs — modal help overlay with Clear + Rect::centered** - `42830b7` (feat)

**Plan metadata:** (docs commit follows)

## Files Created/Modified

- `airev/src/ui/keybindings.rs` — KeyAction enum and handle_key() top-level dispatcher; private handle_normal, handle_scroll_key, handle_help, handle_confirm_quit, handle_insert functions
- `airev/src/ui/help.rs` — render_help_overlay() with Clear + Rect::centered(80%, 80%) modal; build_help_text() returns Text with all keybinding sections
- `airev/src/ui/mod.rs` — pub mod help/keybindings declarations; Mode import added; TODO comment replaced with conditional render_help_overlay call

## Decisions Made

- `handle_scroll_key()` private helper splits j/k/g/G and Ctrl combos out of `handle_normal()` to keep each function under 50 lines (hard rule compliance).
- Ctrl-H/L focus bindings deliberately NOT implemented — Ctrl-H conflicts with Backspace on most terminals (confirmed in Phase 2 research). Uppercase H/L is the only focus keybinding.
- `frame.area().width < 60` guard added to `render_help_overlay()` before calling `Rect::centered()` — prevents zero-height `Rect` panic on narrow terminals (Pitfall 6 from research).
- `Mode` enum imported in `ui/mod.rs` — was not previously imported there, required for the `Mode::HelpOverlay` comparison in the conditional.

## Deviations from Plan

None - plan executed exactly as written.

## Issues Encountered

None.

## User Setup Required

None - no external service configuration required.

## Next Phase Readiness

- `handle_key()` is fully implemented and exported; Plan 03 wires it into the crossterm event loop
- `render_help_overlay()` is wired into `render()` — the `?` key will show the modal as soon as Plan 03 connects keyboard events
- All AppState mutations called by the dispatcher already exist from Plan 01

---
*Phase: 02-rendering-skeleton*
*Completed: 2026-02-18*
