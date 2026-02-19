---
phase: quick
plan: 3
type: execute
wave: 1
depends_on: []
files_modified:
  - airev/src/app.rs
  - airev/src/ui/help.rs
  - airev/src/ui/layout.rs
  - airev/src/ui/keybindings.rs
  - airev/src/ui/mod.rs
  - airev/src/event.rs
  - airev/src/tui.rs
  - airev/src/main.rs
autonomous: true
requirements: []

must_haves:
  truths:
    - "Help overlay scrolls with j/k when terminal is shorter than the help text"
    - "At 80-119 cols, file list panel is visible alongside the diff panel"
    - "Mouse click on a panel focuses it"
    - "Mouse scroll wheel scrolls the focused panel"
  artifacts:
    - path: "airev/src/app.rs"
      provides: "help_scroll field, panel_rects field"
      contains: "help_scroll"
    - path: "airev/src/ui/help.rs"
      provides: "Scrollable help overlay"
      contains: ".scroll("
    - path: "airev/src/ui/layout.rs"
      provides: "3-breakpoint responsive layout (>=120, 80-119, <80)"
      contains: "80"
    - path: "airev/src/event.rs"
      provides: "AppEvent::Mouse variant"
      contains: "Mouse"
    - path: "airev/src/tui.rs"
      provides: "EnableMouseCapture / DisableMouseCapture"
      contains: "MouseCapture"
  key_links:
    - from: "airev/src/ui/keybindings.rs"
      to: "airev/src/app.rs"
      via: "handle_help j/k mutates help_scroll"
      pattern: "help_scroll"
    - from: "airev/src/main.rs"
      to: "airev/src/ui/keybindings.rs"
      via: "AppEvent::Mouse arm calls handle_mouse"
      pattern: "handle_mouse"
    - from: "airev/src/ui/mod.rs"
      to: "airev/src/app.rs"
      via: "stores panel_rects after compute_layout"
      pattern: "panel_rects"
---

<objective>
Fix narrow terminal layout, add scrollable help overlay, and add mouse support.

Purpose: The help overlay text is cut off on short terminals, side panels collapse too aggressively
at medium widths (80-119 cols should show file list + diff), and there is no mouse interaction.

Output: Scrollable help, 3-breakpoint responsive layout, click-to-focus and scroll-wheel mouse support.
</objective>

<execution_context>
@/Users/juliusalexandre/.claude/get-shit-done/workflows/execute-plan.md
@/Users/juliusalexandre/.claude/get-shit-done/templates/summary.md
</execution_context>

<context>
@airev/src/app.rs
@airev/src/ui/help.rs
@airev/src/ui/layout.rs
@airev/src/ui/keybindings.rs
@airev/src/ui/mod.rs
@airev/src/event.rs
@airev/src/tui.rs
@airev/src/main.rs
</context>

<tasks>

<task type="auto">
  <name>Task 1: Scrollable help overlay + 3-breakpoint responsive layout</name>
  <files>
    airev/src/app.rs
    airev/src/ui/help.rs
    airev/src/ui/layout.rs
    airev/src/ui/keybindings.rs
    airev/src/ui/mod.rs
  </files>
  <action>
**app.rs — Add help_scroll field:**
- Add `pub help_scroll: u16` to `AppState` struct (after `comments_scroll`), with doc comment: "Vertical scroll offset for the help overlay Paragraph. Reset to 0 when entering help mode."
- Default to `0` in `Default` impl.

**ui/help.rs — Make help overlay scrollable:**
- Change `render_help_overlay` signature from `(frame: &mut Frame, theme: &Theme)` to `(frame: &mut Frame, theme: &Theme, help_scroll: u16)`.
- Add `.scroll((help_scroll, 0))` to the Paragraph chain (after `.wrap(Wrap { trim: false })`).
- Update the block title to `" Help  — j/k scroll, ? or Esc to dismiss "`.

**ui/keybindings.rs — Add j/k/g/G scroll in help mode:**
- In `handle_help()`, add arms before the dismiss keys:
  - `KeyCode::Char('j')` => `state.help_scroll = state.help_scroll.saturating_add(1); KeyAction::Continue`
  - `KeyCode::Char('k')` => `state.help_scroll = state.help_scroll.saturating_sub(1); KeyAction::Continue`
  - `KeyCode::Char('g')` => `state.help_scroll = 0; KeyAction::Continue`
  - `KeyCode::Char('G')` => `state.help_scroll = u16::MAX; KeyAction::Continue` (ratatui clamps)
- In `handle_normal()`, in the `KeyCode::Char('?')` arm, add `state.help_scroll = 0;` before setting mode to HelpOverlay (reset scroll on open).

**ui/layout.rs — 3-breakpoint responsive layout:**
- Change the `compute_layout` horizontal split logic from a 2-way branch to a 3-way branch:
  - `>= 120`: Keep existing 3-panel layout with `Spacing::Overlap(1)` — no change.
  - `80..=119`: 2-panel layout. Use `Layout::horizontal([Constraint::Percentage(25), Constraint::Percentage(75), Constraint::Length(0)])`. Do NOT use `Spacing::Overlap` here — the right panel is Length(0) and Overlap causes u16 underflow. Use `Spacing::Overlap(1)` only between the two visible panels: actually use `Layout::horizontal([Constraint::Percentage(25), Constraint::Fill(1), Constraint::Length(0)])` with no spacing (default) to avoid the underflow issue. The left panel gets 25% for file names, diff gets the rest.
  - `< 80`: Keep existing diff-only layout — no change.
- Update the module-level doc comment to document the 3 breakpoints.

**ui/mod.rs — Wire scrollable help and 2-panel mode:**
- In the `render()` function, change the help overlay call from `help::render_help_overlay(frame, theme)` to `help::render_help_overlay(frame, theme, state.help_scroll)`.
- The `if left.width > 0` and `if right.width > 0` guards already handle the 2-panel case correctly — file list renders when left.width > 0, comments panel is skipped when right.width == 0. No changes needed for the panel rendering guards.

**Update help text in build_help_text():**
- Add a line under "General" section: `"  j / k         Scroll this help overlay"` (only applies in help mode, but good to document).
  </action>
  <verify>
Run `cargo build` in airev/ — no compile errors. Run `cargo clippy` — no warnings.
Manually verify: `cargo run` in a git repo, press `?`, confirm j/k scrolls the help text. Resize terminal to ~100 cols wide, confirm file list + diff are visible (no comments panel). Resize to <80 cols, confirm diff-only.
  </verify>
  <done>
Help overlay scrolls with j/k/g/G. Layout has 3 breakpoints: >=120 (3 panels), 80-119 (file list + diff), <80 (diff only). Help scroll resets to 0 when opened.
  </done>
</task>

<task type="auto">
  <name>Task 2: Mouse support (click-to-focus, scroll wheel)</name>
  <files>
    airev/src/tui.rs
    airev/src/event.rs
    airev/src/app.rs
    airev/src/ui/keybindings.rs
    airev/src/ui/mod.rs
    airev/src/main.rs
  </files>
  <action>
**tui.rs — Enable mouse capture:**
- Add `use crossterm::event::{EnableMouseCapture, DisableMouseCapture};` to imports.
- In `init_tui()`, change `execute!(out, EnterAlternateScreen)` to `execute!(out, EnterAlternateScreen, EnableMouseCapture)`.
- In `restore_tui()`, change `execute!(stderr(), LeaveAlternateScreen)` to `execute!(stderr(), LeaveAlternateScreen, DisableMouseCapture)`.

**event.rs — Add Mouse variant:**
- Add `use crossterm::event::MouseEvent;` to imports.
- Add variant to `AppEvent`: `/// A mouse event from the terminal (click, scroll, move). Mouse(MouseEvent),`
- In `spawn_event_task`, add a new arm inside the `match maybe_event` block, after the `Event::Resize` arm:
  ```
  Some(Ok(Event::Mouse(mouse))) => {
      let _ = tx.send(AppEvent::Mouse(mouse));
  }
  ```

**app.rs — Store panel rects for hit-testing:**
- Add `use ratatui::layout::Rect;` to imports (already has ratatui deps via ListState — check if Rect is accessible, it may need explicit import).
- Add field `pub panel_rects: [Rect; 3]` to AppState. Doc comment: "Panel Rects [left, center, right] cached after compute_layout for mouse hit-testing. Updated every render frame."
- Default to `[Rect::default(); 3]` in `Default` impl.

**ui/mod.rs — Cache panel rects in AppState:**
- After `let [left, center, right, status_bar] = compute_layout(frame, state);`, add: `state.panel_rects = [left, center, right];`
- This must come BEFORE the viewport height caching and panel rendering (both already happen after compute_layout, so insert right after the compute_layout call).

**ui/keybindings.rs — Add handle_mouse function:**
- Add imports: `use crossterm::event::{MouseEvent, MouseEventKind, MouseButton};`
- Add a new public function `handle_mouse`:
  ```rust
  /// Handles a mouse event: click-to-focus and scroll-wheel.
  ///
  /// Left click on a panel sets focus to that panel. Scroll wheel up/down
  /// scrolls the focused panel by 3 lines (matching typical terminal scroll
  /// speed). Mouse events in HelpOverlay mode scroll the help overlay.
  ///
  /// # Arguments
  ///
  /// * `mouse` — the crossterm mouse event
  /// * `state` — mutable reference to all UI state
  pub fn handle_mouse(mouse: MouseEvent, state: &mut AppState) -> KeyAction {
      match mouse.kind {
          MouseEventKind::Down(MouseButton::Left) => {
              handle_mouse_click(mouse.column, mouse.row, state)
          }
          MouseEventKind::ScrollUp => {
              handle_mouse_scroll_up(state)
          }
          MouseEventKind::ScrollDown => {
              handle_mouse_scroll_down(state)
          }
          _ => KeyAction::Continue,
      }
  }
  ```
- Add private helper `handle_mouse_click(col: u16, row: u16, state: &mut AppState) -> KeyAction`:
  - Check each panel rect in `state.panel_rects` using `.contains(Position { x: col, y: row })` (import `ratatui::layout::Position`).
  - If click is inside `panel_rects[0]` (left) and `panel_rects[0].width > 0`, set `state.focus = PanelFocus::FileList`.
  - If click is inside `panel_rects[1]` (center), set `state.focus = PanelFocus::Diff`.
  - If click is inside `panel_rects[2]` (right) and `panel_rects[2].width > 0`, set `state.focus = PanelFocus::Comments`.
  - Return `KeyAction::Continue`.
- Add private helper `handle_mouse_scroll_up(state: &mut AppState) -> KeyAction`:
  - If `state.mode == Mode::HelpOverlay`, do `state.help_scroll = state.help_scroll.saturating_sub(3);` and return Continue.
  - Otherwise call `state.scroll_up(3)` and return Continue.
- Add private helper `handle_mouse_scroll_down(state: &mut AppState) -> KeyAction`:
  - If `state.mode == Mode::HelpOverlay`, do `state.help_scroll = state.help_scroll.saturating_add(3);` and return Continue.
  - Otherwise call `state.scroll_down(3)` and return Continue.

**main.rs — Wire Mouse event:**
- Add `use ui::keybindings::handle_mouse;` alongside the existing `handle_key` import (update the existing use line to `use ui::keybindings::{handle_key, handle_mouse, KeyAction};`).
- Add a new arm in the event loop match, after the `AppEvent::Key` arm:
  ```
  Some(event::AppEvent::Mouse(mouse)) => {
      handle_mouse(mouse, &mut state);
  }
  ```
  (Mouse events never return Quit, so we just call and continue. The next Render tick redraws.)
  </action>
  <verify>
Run `cargo build` in airev/ — no compile errors. Run `cargo clippy` — no warnings.
Manually verify: `cargo run`, click on different panels — focus border changes. Use scroll wheel — content scrolls. Press `?` to open help, scroll wheel scrolls the help overlay.
  </verify>
  <done>
Mouse capture enabled/disabled in terminal lifecycle. Click on any panel focuses it (border changes to thick). Scroll wheel scrolls focused panel by 3 lines. Scroll wheel works in help overlay too. Panel rects are cached in AppState every frame for hit-testing.
  </done>
</task>

</tasks>

<verification>
1. `cargo build` passes with no errors
2. `cargo clippy` passes with no warnings
3. Help overlay: Press `?`, text scrollable with j/k/g/G and mouse scroll wheel
4. Responsive layout at 3 breakpoints:
   - >= 120 cols: file list + diff + comments (3 panels)
   - 80-119 cols: file list + diff (2 panels, no comments)
   - < 80 cols: diff only
5. Mouse: click panels to focus, scroll wheel to scroll, works in help overlay
</verification>

<success_criteria>
- Help overlay fully visible and scrollable on short terminals
- Medium-width terminals (80-119 cols) show file list alongside diff
- Mouse click-to-focus works on all visible panels
- Mouse scroll wheel scrolls focused panel content
- All existing keybindings remain functional (no regressions)
</success_criteria>

<output>
After completion, create `.planning/quick/3-fix-narrow-terminal-layout-panels-cut-of/3-SUMMARY.md`
</output>
