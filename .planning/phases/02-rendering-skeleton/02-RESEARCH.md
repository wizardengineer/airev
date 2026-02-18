# Phase 2: Rendering Skeleton - Research

**Researched:** 2026-02-18
**Domain:** ratatui 0.30 layout engine, stateful widget scrolling, keybinding dispatch, modal overlay pattern
**Confidence:** HIGH (core APIs verified via docs.rs and official ratatui docs)

---

## Summary

Phase 2 transforms the Phase 1 stub into a navigable 3-panel TUI. The work splits into four
distinct technical domains: (1) responsive layout engine using ratatui 0.30's
`Constraint`-based `Layout` with `Spacing::Overlap` border merging, (2) `AppState` with a
focus enum and per-panel scroll offsets wired to keyboard dispatch, (3) vim-style keybindings
implemented as match guards on `crossterm::event::KeyCode` + `KeyModifiers`, and (4) a modal
help overlay rendered with `Clear` + centered `Rect::centered()` inside the single
`terminal.draw()` closure.

The Phase 1 codebase already provides the event bus, terminal lifecycle, and a static 3-panel
placeholder. Phase 2 replaces that placeholder with a fully wired layout module. The three
scroll models differ by panel type: `ListState` (with `select_next/previous/first/last` +
`scroll_down_by/scroll_up_by`) for the file-list panel; a manual `u16` offset passed to
`Paragraph::scroll((y, 0))` for the diff and comments panels. Viewport height for half-page
scrolling must be stored in `AppState` after each render and looked up at keypress time.

ratatui 0.30 introduced `merge_borders(MergeStrategy::Exact)` on `Block` and
`Spacing::Overlap(1)` on `Layout`, which cleanly collapse shared borders between adjacent
panels. This replaces error-prone manual border-set arithmetic from earlier versions. The
`Rect::centered(h_constraint, v_constraint)` helper and the `Clear` widget are the canonical
pattern for full-screen or popup modal overlays.

**Primary recommendation:** Introduce `ui/layout.rs` as a pure layout module (no state),
`AppState` in `app.rs` (owns focus, mode, scroll offsets, viewport cache), and a keybinding
dispatcher in `ui/keybindings.rs`. The existing `ui.rs` renders by calling into these modules.

---

<phase_requirements>
## Phase Requirements

| ID | Description | Research Support |
|----|-------------|-----------------|
| Navigation and Keybindings | `j`/`k` one-line scroll; `H`/`L`/`Ctrl-h`/`Ctrl-l` focus switch; `g`/`G` top/bottom; `Ctrl-d`/`Ctrl-u` half-page; `Ctrl-f`/`Ctrl-b` full-page; `{`/`}` file jump; `[h`/`]h` hunk nav; `q` quit with confirmation guard; `Esc` dismiss modal; `?` help overlay | crossterm `KeyCode` + `KeyModifiers::CONTROL` match guards; `ListState` scroll methods; manual `u16` scroll offset for Paragraph panels; `AppState.mode` enum gates quit confirmation |
| Layout | 3-panel horizontal split 20/55/25%; `<`/`>` resize ±5%; side panels collapsible below 120 cols; minimum 80-col; resize-safe recompute on every Render event; single `terminal.draw()` call | `Layout::horizontal` with `Constraint::Percentage`/`Min`; `Spacing::Overlap(1)` for shared borders; `frame.area()` recomputes constraints every draw call automatically; collapse via `Constraint::Length(0)` |
| Help Overlay | Full-screen modal listing all keybindings grouped by context; dismissed with `?`, `Esc`, or `q`; content accurate | `Rect::centered(Constraint::Percentage(80), Constraint::Percentage(80))`; `Clear` widget rendered first; `Block::bordered()` with `Paragraph` for content; rendered inside same `terminal.draw()` closure |
</phase_requirements>

---

## Standard Stack

### Core
| Library | Version | Purpose | Why Standard |
|---------|---------|---------|--------------|
| ratatui | 0.30 | Layout engine, widget rendering | Already in workspace; provides `Layout`, `Constraint`, `Block`, `Paragraph`, `List`, `Clear`, `ListState`, `Spacing`, `MergeStrategy` |
| crossterm | 0.29 | Key event decoding | Already in workspace; `KeyCode`, `KeyModifiers::CONTROL`, `KeyEvent` are the input primitives |

No new crates are needed for Phase 2. All capabilities are available in the existing workspace
dependencies.

### Supporting (already in workspace)
| Library | Version | Purpose | When to Use |
|---------|---------|---------|-------------|
| tokio | 1.49 | Async runtime, event channel | Already wired; no changes needed |
| airev-core | workspace | Shared types | If `AppState` or `PanelFocus` enums are promoted to shared types |

### Alternatives Considered
| Instead of | Could Use | Tradeoff |
|------------|-----------|----------|
| Manual `u16` scroll offset for Paragraph | `tui-scrollview` crate | `tui-scrollview` provides a `ScrollView` widget with full viewport management; overkill for Phase 2 static content — add in Phase 3+ when real diff content arrives |
| `ListState` for file panel | Manual index tracking | `ListState` provides `scroll_down_by`, `scroll_up_by`, `select_first`, `select_last`, `offset_mut` built in; no reason to hand-roll |
| `Rect::centered()` for modal | Layout-based centering with Fill constraints | `Rect::centered()` is simpler and more explicit for a single modal; Layout with Fill is better for multi-element composition |

---

## Architecture Patterns

### Recommended Project Structure
```
airev/src/
├── main.rs              # Event loop (Phase 1) — gains AppState, key dispatch call
├── app.rs               # NEW: AppState struct, Mode enum, PanelFocus enum
├── event.rs             # Phase 1 — no changes
├── theme.rs             # Phase 1 — no changes
├── tui.rs               # Phase 1 — no changes
└── ui/
    ├── mod.rs           # Re-exports render() — replaces top-level ui.rs
    ├── layout.rs        # NEW: compute_layout() → [Rect; 4] (3 panels + status bar)
    ├── keybindings.rs   # NEW: handle_key() dispatches KeyEvent → AppState mutation
    └── help.rs          # NEW: render_help_overlay() using Clear + Paragraph
```

### Pattern 1: AppState struct with Mode and PanelFocus
**What:** Central mutable state struct owns all UI state: which panel is focused, current mode,
scroll offsets per panel, last known viewport heights, and a dirty flag for unsaved-state guard.
**When to use:** Pass `&mut AppState` to keybinding handler; pass `&AppState` to render.

```rust
// Source: ratatui community best practices (forum.ratatui.rs discussion #54)
#[derive(Debug, Default)]
pub enum Mode {
    #[default]
    Normal,
    Insert,
    HelpOverlay,
    ConfirmQuit,
}

#[derive(Debug, Default, Clone, Copy, PartialEq)]
pub enum PanelFocus {
    #[default]
    FileList,
    Diff,
    Comments,
}

#[derive(Debug, Default)]
pub struct AppState {
    pub mode: Mode,
    pub focus: PanelFocus,

    // Per-panel scroll state
    pub file_list_state: ListState,        // ListState for file panel (List widget)
    pub diff_scroll: u16,                  // Manual offset for Paragraph diff panel
    pub comments_scroll: u16,             // Manual offset for Paragraph comments panel

    // Viewport heights cached after each render for half-page calculation
    pub diff_viewport_height: u16,
    pub comments_viewport_height: u16,
    pub file_list_viewport_height: u16,

    // Unsaved state guard
    pub has_unsaved_comments: bool,
}
```

### Pattern 2: Responsive 3-Panel Layout with Collapsible Side Panels
**What:** `Layout::horizontal` recomputes constraints on every `terminal.draw()` call because
`frame.area()` reflects current terminal size. Side panels collapse by switching their
constraint to `Constraint::Length(0)` when terminal width is below threshold.
**When to use:** Called inside `terminal.draw()` so every frame gets a fresh layout.

```rust
// Source: docs.rs/ratatui/0.30.0/ratatui/layout/struct.Layout.html
// Source: ratatui.rs/recipes/layout/collapse-borders/
use ratatui::{
    layout::{Constraint, Direction, Layout, Spacing},
    symbols::merge::MergeStrategy,
    widgets::Block,
};

pub fn compute_layout(frame: &Frame, state: &AppState) -> [Rect; 5] {
    let term_width = frame.area().width;

    // Default percentages: file-list 20%, diff 55%, comments 25%
    // Collapse side panels at narrow widths:
    //   < 80 cols: both side panels collapsed (diff only, minimum functional width)
    //   80..120 cols: both side panels collapsed
    //   >= 120 cols: all three panels visible
    let (left_constraint, center_constraint, right_constraint) = if term_width < 80 {
        (Constraint::Length(0), Constraint::Fill(1), Constraint::Length(0))
    } else if term_width < 120 {
        (Constraint::Length(0), Constraint::Fill(1), Constraint::Length(0))
    } else {
        (
            Constraint::Percentage(state.left_pct),       // default 20
            Constraint::Percentage(state.center_pct),     // default 55
            Constraint::Percentage(state.right_pct),      // default 25
        )
    };

    // Split terminal area into status bar (last 1 row) + main area
    let [main_area, status_bar_area] = frame.area().layout(
        Layout::vertical([Constraint::Fill(1), Constraint::Length(1)])
    );

    // Split main area into 3 panels with overlapping borders for merging
    let [left, center, right] = main_area.layout(
        Layout::horizontal([left_constraint, center_constraint, right_constraint])
            .spacing(Spacing::Overlap(1))
    );

    [left, center, right, status_bar_area, main_area]
}
```

### Pattern 3: Border Merging Between Adjacent Panels (ratatui 0.30)
**What:** `Block::merge_borders(MergeStrategy::Exact)` combined with `Spacing::Overlap(1)` on
the layout automatically joins shared borders into clean Unicode box-drawing junctions.
**When to use:** Apply to all three panel blocks. Not needed for the status bar.

```rust
// Source: ratatui.rs/recipes/layout/collapse-borders/
// Source: docs.rs/ratatui/latest/ratatui/symbols/merge/enum.MergeStrategy.html
use ratatui::symbols::merge::MergeStrategy;
use ratatui::widgets::{Block, BorderType};

fn panel_block(title: &str, is_focused: bool, theme: &Theme) -> Block<'_> {
    let border_style = if is_focused {
        Style::default().fg(theme.border_active)
    } else {
        Style::default().fg(theme.border_inactive)
    };
    let border_type = if is_focused {
        BorderType::Thick      // visually distinct active border
    } else {
        BorderType::Plain
    };
    Block::bordered()
        .title(title)
        .border_type(border_type)
        .border_style(border_style)
        .merge_borders(MergeStrategy::Exact)
}
```

**Important:** `BorderType::Rounded` cannot be used with `merge_borders` — rounded corners
cannot be merged with plain segments in Unicode. Use `Plain` or `Thick` only.

### Pattern 4: Keybinding Dispatch with Match Guards for Ctrl Combinations
**What:** A single function matches `KeyEvent` by mode and panel focus, mutates `AppState`,
and returns a control flow indicator. Ctrl-combos use `.contains(KeyModifiers::CONTROL)` guards.
**When to use:** Called from `main.rs` `AppEvent::Key` arm; never inline in the event loop.

```rust
// Source: docs.rs/crossterm/latest/crossterm/event/struct.KeyModifiers.html
// Source: github.com/crossterm-rs/crossterm/blob/master/examples/event-match-modifiers.rs
use crossterm::event::{KeyCode, KeyEvent, KeyModifiers};

pub enum KeyAction {
    Continue,
    Quit,
    ForceQuit,
}

pub fn handle_key(key: KeyEvent, state: &mut AppState) -> KeyAction {
    match state.mode {
        Mode::HelpOverlay => handle_key_help(key, state),
        Mode::ConfirmQuit => handle_key_confirm_quit(key, state),
        Mode::Normal => handle_key_normal(key, state),
        Mode::Insert => handle_key_insert(key, state),
    }
}

fn handle_key_normal(key: KeyEvent, state: &mut AppState) -> KeyAction {
    match key.code {
        // Panel focus
        KeyCode::Char('H') | KeyCode::Char('h')
            if key.modifiers.contains(KeyModifiers::CONTROL) => {
            state.focus = state.focus.prev();
            KeyAction::Continue
        }
        KeyCode::Char('L') | KeyCode::Char('l')
            if key.modifiers.contains(KeyModifiers::CONTROL) => {
            state.focus = state.focus.next();
            KeyAction::Continue
        }
        KeyCode::Char('H') => { state.focus = state.focus.prev(); KeyAction::Continue }
        KeyCode::Char('L') => { state.focus = state.focus.next(); KeyAction::Continue }

        // Scrolling — dispatches to focused panel
        KeyCode::Char('j') => { state.scroll_down(1); KeyAction::Continue }
        KeyCode::Char('k') => { state.scroll_up(1); KeyAction::Continue }
        KeyCode::Char('g') => { state.scroll_top(); KeyAction::Continue }
        KeyCode::Char('G') => { state.scroll_bottom(); KeyAction::Continue }

        // Half-page scroll (Ctrl-d / Ctrl-u)
        KeyCode::Char('d') if key.modifiers.contains(KeyModifiers::CONTROL) => {
            state.half_page_down(); KeyAction::Continue
        }
        KeyCode::Char('u') if key.modifiers.contains(KeyModifiers::CONTROL) => {
            state.half_page_up(); KeyAction::Continue
        }

        // Full-page scroll (Ctrl-f / Ctrl-b)
        KeyCode::Char('f') if key.modifiers.contains(KeyModifiers::CONTROL) => {
            state.full_page_down(); KeyAction::Continue
        }
        KeyCode::Char('b') if key.modifiers.contains(KeyModifiers::CONTROL) => {
            state.full_page_up(); KeyAction::Continue
        }

        // File jumps
        KeyCode::Char('{') => { state.prev_file(); KeyAction::Continue }
        KeyCode::Char('}') => { state.next_file(); KeyAction::Continue }

        // Hunk navigation ([ and ] with h)
        // Note: '[' and ']' alone cannot carry a following 'h' in a single KeyEvent.
        // Implement as two-key chord or use '[' as prev-hunk, ']' as next-hunk directly.
        KeyCode::Char('[') => { state.prev_hunk(); KeyAction::Continue }
        KeyCode::Char(']') => { state.next_hunk(); KeyAction::Continue }

        // Help overlay
        KeyCode::Char('?') => { state.mode = Mode::HelpOverlay; KeyAction::Continue }

        // Quit
        KeyCode::Char('q') | KeyCode::Esc => {
            if state.has_unsaved_comments {
                state.mode = Mode::ConfirmQuit;
                KeyAction::Continue
            } else {
                KeyAction::Quit
            }
        }

        // Layout resize
        KeyCode::Char('<') => { state.shrink_diff_panel(); KeyAction::Continue }
        KeyCode::Char('>') => { state.grow_diff_panel(); KeyAction::Continue }

        _ => KeyAction::Continue,
    }
}
```

### Pattern 5: Modal Help Overlay with Clear + Rect::centered
**What:** The overlay is rendered inside the same `terminal.draw()` closure after all panels.
`Clear` erases the background cells; then a `Block`+`Paragraph` renders on top.
**When to use:** Conditional on `state.mode == Mode::HelpOverlay` inside the draw closure.

```rust
// Source: ratatui.rs/recipes/layout/center-a-widget/
// Source: docs.rs/ratatui/0.30.0/ratatui/widgets/struct.Clear.html
use ratatui::{
    layout::Constraint,
    widgets::{Block, Clear, Paragraph, Wrap},
};

fn render_help_overlay(frame: &mut Frame, theme: &Theme) {
    // Center the modal: 80% width, 80% height
    let area = frame.area().centered(
        Constraint::Percentage(80),
        Constraint::Percentage(80),
    );
    // Step 1: erase background
    frame.render_widget(Clear, area);
    // Step 2: render modal border
    let block = Block::bordered()
        .title(" Help — ? or Esc to dismiss ")
        .border_style(Style::default().fg(theme.border_active));
    let help_text = build_help_text(); // returns Text<'static>
    let paragraph = Paragraph::new(help_text)
        .block(block)
        .wrap(Wrap { trim: false });
    // Step 3: render content
    frame.render_widget(paragraph, area);
}
```

### Pattern 6: Scroll State Management per Panel
**What:** Three scroll models coexist. File-list panel uses `ListState` (stateful widget).
Diff and comments panels use `Paragraph::scroll((y, 0))` driven by a manual `u16` offset.
Viewport heights are captured after each render and stored in `AppState`.
**When to use:** Centralized in `AppState` methods; render code reads state, never modifies it.

```rust
// Source: docs.rs/ratatui/0.30.0/ratatui/widgets/struct.ListState.html
// Source: docs.rs/ratatui/0.30.0/ratatui/widgets/struct.Paragraph.html#method.scroll
impl AppState {
    pub fn scroll_down(&mut self, lines: u16) {
        match self.focus {
            PanelFocus::FileList => {
                self.file_list_state.scroll_down_by(lines as usize);
            }
            PanelFocus::Diff => {
                self.diff_scroll = self.diff_scroll.saturating_add(lines);
            }
            PanelFocus::Comments => {
                self.comments_scroll = self.comments_scroll.saturating_add(lines);
            }
        }
    }

    pub fn half_page_down(&mut self) {
        let half = match self.focus {
            PanelFocus::FileList => self.file_list_viewport_height / 2,
            PanelFocus::Diff => self.diff_viewport_height / 2,
            PanelFocus::Comments => self.comments_viewport_height / 2,
        };
        self.scroll_down(half.max(1));
    }

    pub fn scroll_top(&mut self) {
        match self.focus {
            PanelFocus::FileList => { self.file_list_state.select_first(); }
            PanelFocus::Diff => { self.diff_scroll = 0; }
            PanelFocus::Comments => { self.comments_scroll = 0; }
        }
    }

    pub fn scroll_bottom(&mut self) {
        match self.focus {
            PanelFocus::FileList => { self.file_list_state.select_last(); }
            // For Paragraph panels, need total line count — set to a large sentinel
            // that ratatui clamps silently (Paragraph ignores out-of-bounds offsets)
            PanelFocus::Diff => { self.diff_scroll = u16::MAX; }
            PanelFocus::Comments => { self.comments_scroll = u16::MAX; }
        }
    }
}

// In the render function — capture viewport heights for next keypress cycle:
fn render_diff_panel(frame: &mut Frame, area: Rect, state: &mut AppState, theme: &Theme) {
    // Inner area after Block borders
    let inner = area.inner(Margin { vertical: 1, horizontal: 1 });
    state.diff_viewport_height = inner.height;  // cache for half-page calc

    let paragraph = Paragraph::new("placeholder diff content")
        .scroll((state.diff_scroll, 0));
    frame.render_widget(
        Block::bordered().title("Diff")
            .merge_borders(MergeStrategy::Exact),
        area,
    );
    frame.render_widget(paragraph, inner);
}
```

### Pattern 7: Status Bar with Mode Indicator
**What:** A 1-row `Paragraph` at the bottom of the terminal, always showing `NORMAL` or
`INSERT`. Styled with theme status bar colors. Never blank.
**When to use:** Rendered last in the draw closure (on top of panels).

```rust
// Source: community pattern — status bar via Paragraph, verified against ratatui docs
fn render_status_bar(frame: &mut Frame, area: Rect, state: &AppState, theme: &Theme) {
    let mode_text = match state.mode {
        Mode::Normal | Mode::ConfirmQuit | Mode::HelpOverlay => " NORMAL ",
        Mode::Insert => " INSERT ",
    };
    let mode_span = Span::styled(
        mode_text,
        Style::default()
            .fg(match state.mode {
                Mode::Insert => theme.status_mode_insert,
                _ => theme.status_mode_normal,
            })
            .add_modifier(Modifier::BOLD),
    );
    let status_line = Line::from(vec![mode_span]);
    frame.render_widget(
        Paragraph::new(status_line)
            .style(Style::default().bg(theme.status_bar_bg).fg(theme.status_bar_fg)),
        area,
    );
}
```

### Anti-Patterns to Avoid
- **Calling `terminal.draw()` twice per frame:** The Phase 1 decision is firm — one draw per Render event. All overlays, the status bar, and all panels are rendered inside one closure.
- **Using `BorderType::Rounded` with `merge_borders`:** Rounded corners cannot be merged with adjacent plain borders in Unicode. This produces garbled junction characters.
- **Storing `Rect` values across frames:** `Rect` values from one frame's layout are stale by the next frame if the terminal has been resized. Always recompute inside `terminal.draw()`. The one exception is caching viewport height for half-page scroll calculations — these values lag by one frame, which is acceptable.
- **Using `u16::MAX` as a sentinel for `G` (bottom) without knowing line count:** `Paragraph::scroll` clamps silently, so passing `u16::MAX` scrolls as far as possible — this is acceptable for Phase 2 with static placeholder content but must be replaced with actual line count tracking in Phase 3 when real diff content arrives.
- **Two-character keychords for `[h`/`]h` hunk navigation:** A crossterm `KeyEvent` is a single keypress. Vim's `[h`/`]h` requires a sequence state machine. For Phase 2, use `[` and `]` alone as hunk nav keys (which are close to the spec intent); document the divergence.
- **Keeping `H` and `Ctrl-H` as separate keys for panel focus:** In most terminals, `Ctrl-H` generates the same byte as `Backspace`. Add both `H`/`L` (uppercase) and `Ctrl-h`/`Ctrl-l` but test for terminal compatibility; prefer uppercase `H`/`L` as primary.

---

## Don't Hand-Roll

| Problem | Don't Build | Use Instead | Why |
|---------|-------------|-------------|-----|
| Adjacent border collapsing | Manual border-set arithmetic selecting junction chars | `Spacing::Overlap(1)` + `.merge_borders(MergeStrategy::Exact)` | ratatui 0.30 handles all Unicode box-drawing combinations; manual approaches miss corner cases |
| Scroll state for list panels | Custom `selected: usize` + `offset: usize` | `ratatui::widgets::ListState` with `scroll_down_by/scroll_up_by/select_first/select_last` | Built-in bounds clamping, viewport tracking, and stateful rendering integration |
| Modal overlay centering | `Layout::horizontal/vertical` with nested Fill constraints | `Rect::centered(h_constraint, v_constraint)` | One-liner; no layout struct needed for a single centered rect |
| Background erasure for modal | Drawing a filled rectangle in the background color | `frame.render_widget(Clear, area)` | `Clear` resets cells to empty so the overlay draws cleanly over any panel content |
| Keybinding sequence handling (`[h`) | State machine in the event loop | Simplify to single-key `[`/`]` for Phase 2 | Two-key sequences are a Phase 4+ concern; add sequence machine only when real hunk data exists |

**Key insight:** ratatui's constraint solver (Cassowary) handles all the math for responsive
layouts. Never precompute pixel positions — pass constraints and let `split()` / `areas()` /
`frame.area().layout()` do the arithmetic on every frame.

---

## Common Pitfalls

### Pitfall 1: Viewport Height Stale by One Frame for Ctrl-D/Ctrl-U
**What goes wrong:** The viewport height cached in `AppState` is from the previous render.
If the user resizes the terminal and immediately presses Ctrl-D, the half-page distance is
based on the old height.
**Why it happens:** The render loop runs after the key event is processed.
**How to avoid:** Accept the one-frame lag — it is imperceptible in practice. The alternative
(computing layout twice per event) violates the single-draw constraint.
**Warning signs:** Half-page scroll jumps an unexpected number of lines after a resize.

### Pitfall 2: Ctrl-H Conflicts with Backspace in Many Terminals
**What goes wrong:** In many terminal emulators, `Ctrl-H` produces the same escape sequence as
`Backspace` (ASCII DEL or BS). A handler matching `Ctrl-H` may inadvertently activate on
Backspace keypresses in Insert mode.
**Why it happens:** Legacy terminal control codes; no crossterm workaround without keyboard
enhancement flags (which require kitty protocol support).
**How to avoid:** Use uppercase `H`/`L` (via `KeyCode::Char('H')` — crossterm sets
`KeyModifiers::SHIFT` automatically) as the primary panel focus keys; make `Ctrl-H`/`Ctrl-L`
secondary and only wire them in Normal mode where Backspace is irrelevant.
**Warning signs:** Pressing Backspace in a text field switches panel focus.

### Pitfall 3: `[h`/`]h` Hunk Navigation Requires Chord State Machine
**What goes wrong:** The spec says `[h`/`]h`; crossterm delivers one `KeyEvent` per keypress.
Implementing `[h` requires storing "previous key was `[`" state and acting on the next `h`.
**Why it happens:** Vim supports multi-key sequences; crossterm does not natively.
**How to avoid:** For Phase 2, map `[` → prev_hunk and `]` → next_hunk (no `h` suffix).
Document this as a simplification. A chord engine can be added in Phase 3 when actual hunks
are rendered.
**Warning signs:** None in Phase 2 (placeholder content); will manifest when hunk navigation
is functionally tested in Phase 3.

### Pitfall 4: Collapsed Panel Constraint Leaves Border Artifacts
**What goes wrong:** Setting a panel to `Constraint::Length(0)` collapses its content area
but the overlapping-border approach may still draw one character of border from the adjacent
panel.
**Why it happens:** `Spacing::Overlap(1)` shifts panels left by 1; a zero-width panel becomes
negative width, which ratatui clamps to zero.
**How to avoid:** When a side panel is collapsed, use `Constraint::Length(0)` and do NOT
render that panel's `Block` at all (skip the `render_widget` call). Check `area.width == 0`
before rendering any widget into that area.
**Warning signs:** A stray vertical line appears at position 0 or at the terminal edge when
panels are collapsed.

### Pitfall 5: `MergeStrategy::Exact` Fails for Thick + Plain Borders
**What goes wrong:** Using `BorderType::Thick` on the focused panel while adjacent panels use
`BorderType::Plain` may not have a Unicode junction character for every combination.
`MergeStrategy::Exact` falls back to `Replace` (the last-rendered widget wins) for missing
characters.
**Why it happens:** The Unicode Box Drawing block does not cover all thick/plain combos.
**How to avoid:** Use `MergeStrategy::Fuzzy` when mixing border weights — it applies
transformation rules to find an approximate merge. Or keep all panels on `BorderType::Plain`
and distinguish focused panels via color only (theme `border_active` vs `border_inactive`).
**Warning signs:** T-junctions between a thick focused panel and plain adjacent panels show
incomplete or wrong junction characters.

### Pitfall 6: `Rect::centered()` Returns Wrong Size When Terminal Too Narrow
**What goes wrong:** `Rect::centered(Constraint::Percentage(80), ...)` with a terminal
narrower than the overlay's minimum viable content width clips the modal or causes a Rect of
height 0.
**Why it happens:** Percentage constraints operate on absolute dimensions.
**How to avoid:** Check `area.width > MINIMUM_OVERLAY_WIDTH` before rendering the help overlay;
if the terminal is too narrow, render a minimal "? to dismiss" status bar message instead.
**Warning signs:** Help overlay is invisible or renders as a 0-height block.

### Pitfall 7: `H`/`L` Uppercase Requires Shift — Check `KeyModifiers::SHIFT`
**What goes wrong:** `KeyCode::Char('H')` in crossterm already encodes the uppercase character;
the `modifiers` field also contains `KeyModifiers::SHIFT`. A match guard checking
`!key.modifiers.contains(KeyModifiers::CONTROL)` avoids accidental overlap, but naively
matching `Char('H')` alone works correctly because crossterm delivers uppercase `H` only when
Shift is held.
**Why it happens:** Crossterm delivers character case in `KeyCode::Char`; no extra guard needed
for shift on letters.
**How to avoid:** Match `KeyCode::Char('H')` directly — crossterm handles shift. No manual
modifier check needed for uppercase letters.
**Warning signs:** None; this is clarifying correct behavior, not a failure mode.

---

## Code Examples

Verified patterns from official sources:

### Layout with Overlap Spacing and Border Merging
```rust
// Source: ratatui.rs/recipes/layout/collapse-borders/
use ratatui::{
    layout::{Constraint, Layout, Spacing},
    symbols::merge::MergeStrategy,
};

let [left, center, right] = main_area.layout(
    Layout::horizontal([
        Constraint::Percentage(20),
        Constraint::Percentage(55),
        Constraint::Percentage(25),
    ])
    .spacing(Spacing::Overlap(1))
);

// Render each panel block with merge_borders
frame.render_widget(
    Block::bordered()
        .title("Files")
        .border_style(inactive_style)
        .merge_borders(MergeStrategy::Exact),
    left,
);
```

### Rect::centered for Modal Overlay
```rust
// Source: ratatui.rs/recipes/layout/center-a-widget/
let overlay_area = frame.area().centered(
    Constraint::Percentage(80),  // horizontal: 80% of terminal width
    Constraint::Percentage(80),  // vertical: 80% of terminal height
);
frame.render_widget(Clear, overlay_area);
frame.render_widget(help_paragraph, overlay_area);
```

### Vertical Split: Main Area + Status Bar
```rust
// Source: docs.rs/ratatui/0.30.0/ratatui/layout/struct.Rect.html (layout method)
let [main_area, status_bar_area] = frame.area().layout(
    Layout::vertical([
        Constraint::Fill(1),
        Constraint::Length(1),
    ])
);
```

### ListState Scroll Methods
```rust
// Source: docs.rs/ratatui/0.30.0/ratatui/widgets/struct.ListState.html
let mut list_state = ListState::default();
list_state.scroll_down_by(1);          // j
list_state.scroll_up_by(1);            // k
list_state.select_first();             // g
list_state.select_last();              // G
list_state.scroll_down_by(height / 2); // Ctrl-d (half-page)
list_state.scroll_up_by(height / 2);   // Ctrl-u (half-page)
```

### Paragraph Scroll (non-List panels)
```rust
// Source: docs.rs/ratatui/0.30.0/ratatui/widgets/struct.Paragraph.html#method.scroll
// Note: scroll takes (y, x) — vertical offset FIRST
let paragraph = Paragraph::new(content)
    .scroll((state.diff_scroll, 0));  // scroll vertically, no horizontal offset
```

### Ctrl-D Pattern with KeyModifiers Guard
```rust
// Source: docs.rs/crossterm/0.29.0/crossterm/event/struct.KeyModifiers.html
// Source: github.com/crossterm-rs/crossterm/blob/master/examples/event-match-modifiers.rs
KeyCode::Char('d') if key.modifiers.contains(KeyModifiers::CONTROL) => {
    state.half_page_down();
}
KeyCode::Char('u') if key.modifiers.contains(KeyModifiers::CONTROL) => {
    state.half_page_up();
}
```

### Mode Indicator in Status Bar
```rust
// Pattern: always shows NORMAL or INSERT — never blank
let mode_str = match state.mode {
    Mode::Insert => " INSERT ",
    _ => " NORMAL ",          // HelpOverlay and ConfirmQuit show NORMAL underneath
};
```

---

## State of the Art

| Old Approach | Current Approach | When Changed | Impact |
|--------------|------------------|--------------|--------|
| Manual border-set arithmetic for shared panel borders | `Spacing::Overlap(1)` + `Block::merge_borders(MergeStrategy::Exact)` | ratatui 0.30.0 | Eliminates complex manual junction character selection |
| `Layout::default().constraints([...]).split(area)` | `area.layout(Layout::horizontal([...]))` returning `[Rect; N]` | ratatui ~0.28 | Type-safe N-element array return; no slice indexing |
| Hand-rolled `selected: usize` + bounds check | `ListState::scroll_down_by(n)` / `select_first/last` | ratatui ~0.26 | Built-in clamping; no off-by-one risk |
| Separate `Layout::vertical` call to center popups | `Rect::centered(h_constraint, v_constraint)` | ratatui 0.30.0 | Single-line centering; no nested layout struct |

**Deprecated/outdated:**
- `Layout::default().direction(Direction::Horizontal).constraints([...]).split(area)` returning `Vec<Rect>`: Still works but the `areas::<N>()` / `Rect::layout()` pattern returning `[Rect; N]` is preferred — avoids indexing and provides compile-time checked length.
- `BorderType::Rounded` for merged layouts: Cannot be merged; do not use in multi-panel layouts with `merge_borders`.

---

## Open Questions

1. **`[h`/`]h` hunk navigation chord handling**
   - What we know: crossterm delivers one `KeyEvent` per physical key; multi-key sequences require application-level state
   - What's unclear: Whether the spec intends `[h` as a true two-key chord or accepts `[`/`]` alone
   - Recommendation: Phase 2 uses `[`/`]` alone; document the simplification; chord engine in Phase 3+

2. **`Ctrl-H` terminal compatibility for panel focus**
   - What we know: Many terminals map `Ctrl-H` to Backspace (ASCII 0x08); crossterm may deliver `KeyCode::Backspace` instead of `KeyCode::Char('h')` with CONTROL modifier
   - What's unclear: Exact crossterm behavior on macOS Terminal.app and iTerm2 for Ctrl-H
   - Recommendation: Test empirically; use uppercase `H`/`L` as primary (reliable); document Ctrl-h/l as "may not work on all terminals"

3. **`MergeStrategy::Exact` vs `MergeStrategy::Fuzzy` for mixed border types**
   - What we know: `Exact` falls back to `Replace` when no Unicode junction exists; `Fuzzy` applies approximation rules
   - What's unclear: Which produces better output for the focused (Thick) vs inactive (Plain) panel border junction
   - Recommendation: Start with all-Plain borders differentiated by color; switch to Thick+Fuzzy if visual differentiation is insufficient after manual testing

4. **`Paragraph::scroll` behavior at `u16::MAX` (sentinel for `G` in diff panel)**
   - What we know: The docs say scroll offset is applied after wrapping; no explicit mention of clamping to content height
   - What's unclear: Whether ratatui silently clamps or panics when offset > line count
   - Recommendation: Empirically verify; the confirmed behavior is silent clamping based on community reports, but Phase 3 should replace the sentinel with an actual line count

---

## Sources

### Primary (HIGH confidence)
- [docs.rs/ratatui/0.30.0/ratatui/layout](https://docs.rs/ratatui/0.30.0/ratatui/layout/index.html) — Layout, Constraint, Flex, Spacing, Direction, Rect
- [docs.rs/ratatui/0.30.0/ratatui/layout/struct.Layout.html](https://docs.rs/ratatui/0.30.0/ratatui/layout/struct.Layout.html) — split, areas, spacing, flex methods
- [docs.rs/ratatui/0.30.0/ratatui/layout/struct.Rect.html](https://docs.rs/ratatui/0.30.0/ratatui/layout/struct.Rect.html) — centered, layout, centered_horizontally, centered_vertically
- [docs.rs/ratatui/0.30.0/ratatui/layout/enum.Constraint.html](https://docs.rs/ratatui/0.30.0/ratatui/layout/enum.Constraint.html) — Min, Max, Length, Percentage, Ratio, Fill
- [docs.rs/ratatui/0.30.0/ratatui/widgets/struct.Clear.html](https://docs.rs/ratatui/0.30.0/ratatui/widgets/struct.Clear.html) — Clear widget for modal background erasure
- [docs.rs/ratatui/0.30.0/ratatui/widgets/struct.Paragraph.html](https://docs.rs/ratatui/0.30.0/ratatui/widgets/struct.Paragraph.html) — scroll((y,x)) method, wrap
- [docs.rs/ratatui/0.30.0/ratatui/widgets/struct.ListState.html](https://docs.rs/ratatui/0.30.0/ratatui/widgets/struct.ListState.html) — scroll_down_by, scroll_up_by, select_first, select_last, offset_mut
- [docs.rs/ratatui/0.30.0/ratatui/widgets/enum.BorderType.html](https://docs.rs/ratatui/0.30.0/ratatui/widgets/enum.BorderType.html) — all 12 BorderType variants; Rounded incompatible with merge_borders
- [docs.rs/ratatui/latest/ratatui/symbols/merge/enum.MergeStrategy.html](https://docs.rs/ratatui/latest/ratatui/symbols/merge/enum.MergeStrategy.html) — Replace, Exact, Fuzzy
- [docs.rs/crossterm/0.29.0/crossterm/event/struct.KeyModifiers.html](https://docs.rs/crossterm/0.29.0/crossterm/event/struct.KeyModifiers.html) — CONTROL, SHIFT, ALT constants; contains() method
- [docs.rs/crossterm/0.29.0/crossterm/event/struct.KeyEvent.html](https://docs.rs/crossterm/0.29.0/crossterm/event/struct.KeyEvent.html) — code, modifiers, kind, state fields

### Secondary (MEDIUM confidence)
- [ratatui.rs/highlights/v030](https://ratatui.rs/highlights/v030/) — Rect::centered, merge_borders, Spacing::Overlap confirmed as 0.30 features
- [ratatui.rs/recipes/layout/collapse-borders](https://ratatui.rs/recipes/layout/collapse-borders/) — Spacing::Overlap(1) + merge_borders(MergeStrategy::Exact) full pattern
- [ratatui.rs/recipes/layout/center-a-widget](https://ratatui.rs/recipes/layout/center-a-widget/) — Rect::centered + Clear pattern for modal overlay
- [ratatui.rs/concepts/layout](https://ratatui.rs/concepts/layout/) — Cassowary solver, Flex alignment, constraint priority
- [github.com/crossterm-rs/crossterm examples/event-match-modifiers.rs](https://github.com/crossterm-rs/crossterm/blob/master/examples/event-match-modifiers.rs) — KeyModifiers::CONTROL match guard pattern
- [jslazak.com/ratatui-border-merging](https://jslazak.com/ratatui-border-merging/) — MergeStrategy visual comparison for Exact vs Fuzzy

### Tertiary (LOW confidence)
- Rust forum / ratatui forum community pattern for `AppState` struct with `Mode` enum — widely used but no single authoritative reference; pattern is idiomatic
- `u16::MAX` scroll sentinel for Paragraph bottom — reported as silently clamped by community; not documented in official Paragraph docs; validate empirically

---

## Metadata

**Confidence breakdown:**
- Standard stack: HIGH — all APIs verified on docs.rs for ratatui 0.30.0 and crossterm 0.29.0
- Layout (Constraint, Spacing, merge_borders): HIGH — confirmed as 0.30 features in release notes and docs
- Scroll patterns (ListState, Paragraph offset): HIGH — API verified; half-page pattern is community-derived but straightforward
- Keybinding dispatch (KeyModifiers guards): HIGH — verified against crossterm official docs and examples
- Modal overlay (Clear + Rect::centered): HIGH — both verified against ratatui 0.30 docs
- Pitfalls (Ctrl-H/Backspace, Rounded borders, chord sequences): MEDIUM — Ctrl-H behavior is terminal-dependent; confirmed for common terminals

**Research date:** 2026-02-18
**Valid until:** 2026-03-20 (30 days — ratatui 0.30 and crossterm 0.29 are stable; verify if workspace bumps versions)
