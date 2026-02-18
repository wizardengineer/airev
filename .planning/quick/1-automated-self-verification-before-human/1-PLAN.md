---
phase: quick
plan: "01"
type: execute
wave: 1
depends_on: []
files_modified:
  - .planning/phases/01-foundation/01-04-PLAN.md
  - .planning/ROADMAP.md
  - Cargo.toml
  - airev/Cargo.toml
  - airev/src/theme.rs
  - airev/src/main.rs
  - airev/src/ui.rs
autonomous: true
requirements:
  - Automated pre-check before checkpoint
  - Theme system foundation

must_haves:
  truths:
    - "01-04-PLAN.md has an auto pre-check task that runs before the human-verify checkpoint"
    - "cargo build and WAL check run as scripted steps with exit-code verification"
    - "Theme struct with dark and catppuccin-mocha variants compiles without errors"
    - "ui::render accepts &Theme and uses theme colors for panel borders"
    - "main.rs loads theme from ~/.config/airev/config.toml, defaulting to dark"
  artifacts:
    - path: ".planning/phases/01-foundation/01-04-PLAN.md"
      provides: "Checkpoint plan with automated pre-check task prepended"
      contains: "task type=\"auto\""
    - path: "airev/src/theme.rs"
      provides: "Theme struct with dark() and catppuccin_mocha() constructors"
      exports: ["Theme", "Theme::from_name"]
    - path: "airev/src/main.rs"
      provides: "Theme loaded from config and passed to render"
      contains: "ui::render(frame, &theme)"
    - path: "airev/src/ui.rs"
      provides: "render(frame, theme: &Theme) using theme.border_active/border_inactive"
      contains: "pub fn render(frame: &mut Frame, theme: &Theme)"
  key_links:
    - from: "airev/src/main.rs"
      to: "airev/src/theme.rs"
      via: "Theme::from_name call with config-loaded theme name"
      pattern: "Theme::from_name"
    - from: "airev/src/ui.rs"
      to: "airev/src/theme.rs"
      via: "&Theme parameter used on Block border styling"
      pattern: "theme\\.border"
---

<objective>
Two improvements: (1) Add an automated pre-check task to the Phase 1 checkpoint plan so
cargo build, WAL mode, and startup timing run as scripted steps before the human is asked
to verify visually. (2) Lay down the theme system foundation in airev: a Theme struct with
dark (ANSI 16) and catppuccin-mocha (RGB) variants, config loading from XDG path, and
border colors wired into ui::render.

Purpose: Automated checks catch regressions without human attention. The theme system
establishes the color abstraction before Phase 2 adds real content — retrofitting it later
means touching every widget at once.
Output: Updated 01-04-PLAN.md, updated ROADMAP.md note, new theme.rs, updated Cargo.tomls,
updated main.rs and ui.rs.
</objective>

<execution_context>
@/Users/juliusalexandre/.claude/get-shit-done/workflows/execute-plan.md
@/Users/juliusalexandre/.claude/get-shit-done/templates/summary.md
</execution_context>

<context>
@.planning/PROJECT.md
@.planning/ROADMAP.md
@.planning/phases/01-foundation/01-04-PLAN.md
@airev/src/main.rs
@airev/src/ui.rs
@Cargo.toml
@airev/Cargo.toml
</context>

<tasks>

<task type="auto">
  <name>Task 1: Insert automated pre-check into 01-04-PLAN.md and note in ROADMAP.md</name>
  <files>.planning/phases/01-foundation/01-04-PLAN.md</files>
  <files>.planning/ROADMAP.md</files>
  <action>
Edit `.planning/phases/01-foundation/01-04-PLAN.md`:

1. Insert a new `<task type="auto">` block BEFORE the existing
   `<task type="checkpoint:human-verify">` block. The existing checkpoint task stays
   unchanged — only prepend the new auto task.

2. Also update the frontmatter field `autonomous: false` — it stays `false` because the
   plan still has a checkpoint. No change needed there.

The new auto task to insert:

```xml
<task type="auto">
  <name>Automated pre-check: build, WAL mode, startup timing</name>
  <files>.airev/reviews.db</files>
  <action>
Run the three automatable Phase 1 exit criteria as scripted checks. All three must pass
before the human verification step runs.

**Check 1 — Workspace build:**
```bash
cargo build --workspace 2>&1
if [ $? -ne 0 ]; then
  echo "FAIL: cargo build --workspace exited non-zero"
  exit 1
fi
echo "PASS: cargo build --workspace"
```

**Check 2 — WAL mode:**
Run airev briefly (send 'q' via stdin), then inspect the database:
```bash
echo q | cargo run -p airev -- 2>/dev/null || true
JOURNAL=$(sqlite3 .airev/reviews.db 'PRAGMA journal_mode;' 2>/dev/null)
if [ "$JOURNAL" != "wal" ]; then
  echo "FAIL: PRAGMA journal_mode returned '$JOURNAL', expected 'wal'"
  exit 1
fi
echo "PASS: journal_mode = $JOURNAL"
```

**Check 3 — Startup timing (informational, not blocking):**
```bash
START=$(date +%s%3N)
echo q | cargo run -p airev -- 2>/dev/null || true
END=$(date +%s%3N)
MS=$((END - START))
echo "Startup + quit round-trip: ${MS}ms (warm build cache — subtract cargo overhead manually)"
if [ $MS -gt 5000 ]; then
  echo "WARN: startup took over 5s — investigate cold-start path"
fi
```
Note: `time cargo run` includes cargo's own overhead on a warm cache. The 100ms threshold
from the exit criteria refers to the UI appearing after the binary starts, which requires
visual confirmation (Test 5 in the human checkpoint below). This check flags pathological
slowness (>5s) only.

If any check exits non-zero, fix the underlying issue in the appropriate prior plan before
proceeding to the human verification step.
  </action>
  <verify>
All three bash blocks exit 0 (or print PASS). `sqlite3 .airev/reviews.db 'PRAGMA journal_mode;'`
returns `wal`. `cargo build --workspace` exits 0.
  </verify>
  <done>
All automatable checks pass: workspace compiles, WAL confirmed, startup within threshold.
Execution proceeds automatically to the human-verify checkpoint.
  </done>
</task>
```

Edit `.planning/ROADMAP.md`:

In the Phase 1 **Exit criteria** paragraph (the block starting "**Exit criteria:**"),
append this sentence at the end of the paragraph:

"Automated pre-checks (cargo build exit code, sqlite3 WAL mode query) run as a scripted
auto task before the human checkpoint — the human is only asked to verify what cannot be
scripted (panic recovery, SIGTERM behavior, visual startup timing)."
  </action>
  <verify>
Read the updated 01-04-PLAN.md and confirm: (a) the new `<task type="auto">` block appears
before the `<task type="checkpoint:human-verify">` block, (b) all three check scripts are
present. Read ROADMAP.md and confirm the appended sentence appears in the Phase 1 exit
criteria block.
  </verify>
  <done>
01-04-PLAN.md has two tasks: auto pre-check first, then the unchanged human-verify
checkpoint. ROADMAP.md notes that checkpoint phases include automated pre-checks.
  </done>
</task>

<task type="auto">
  <name>Task 2: Add theme system — Theme struct, deps, config loading, border wiring</name>
  <files>Cargo.toml</files>
  <files>airev/Cargo.toml</files>
  <files>airev/src/theme.rs</files>
  <files>airev/src/main.rs</files>
  <files>airev/src/ui.rs</files>
  <action>
**Step A — Workspace Cargo.toml: add serde and toml to workspace deps.**

In `/Users/juliusalexandre/Projects/PersonalWork/diff-grief/Cargo.toml`, add under
`[workspace.dependencies]`:

```toml
serde = { version = "1", features = ["derive"] }
toml = "0.8"
```

**Step B — airev/Cargo.toml: pull serde and toml from workspace.**

Add to `[dependencies]` in `/Users/juliusalexandre/Projects/PersonalWork/diff-grief/airev/Cargo.toml`:

```toml
serde = { workspace = true }
toml = { workspace = true }
```

**Step C — Create airev/src/theme.rs.**

Full file content:

```rust
//! Color theme system for airev.
//!
//! A `Theme` holds named `ratatui::style::Color` fields covering every UI surface
//! airev renders. Two built-in themes are provided:
//!
//! - `dark` — uses ANSI 16 colors (`Color::Reset`, `Color::DarkGray`, etc.) so it
//!   works on any terminal including 256-color SSH sessions with no truecolor support.
//! - `catppuccin_mocha` — Catppuccin Mocha palette in RGB; requires truecolor.
//!
//! Phase 1 only uses `border_active` and `border_inactive`. All other fields are
//! defined now so Phase 2+ can use them without a schema change.

use ratatui::style::Color;

/// All color values used across airev's UI surfaces.
///
/// Every field is a `ratatui::style::Color`. Callers use `theme.field` directly
/// inside `Style::default().fg(theme.border_active)`.
#[derive(Debug, Clone)]
pub struct Theme {
    // Panel borders
    /// Border color for the currently focused panel.
    pub border_active: Color,
    /// Border color for unfocused panels.
    pub border_inactive: Color,

    // Diff view
    /// Background/foreground for added lines (`+`).
    pub diff_added: Color,
    /// Background/foreground for removed lines (`-`).
    pub diff_removed: Color,
    /// Color for unchanged context lines.
    pub diff_context: Color,
    /// Color for hunk header lines (`@@ ... @@`).
    pub diff_hunk_header: Color,

    // File tree
    /// File status: newly added.
    pub file_added: Color,
    /// File status: deleted.
    pub file_removed: Color,
    /// File status: modified.
    pub file_modified: Color,

    // Comment severity badges
    /// Badge color for critical severity.
    pub badge_critical: Color,
    /// Badge color for major severity.
    pub badge_major: Color,
    /// Badge color for minor severity.
    pub badge_minor: Color,
    /// Badge color for info severity.
    pub badge_info: Color,

    // Status bar
    /// Status bar background.
    pub status_bar_bg: Color,
    /// Status bar foreground (general text).
    pub status_bar_fg: Color,
    /// Mode indicator color when in NORMAL mode.
    pub status_mode_normal: Color,
    /// Mode indicator color when in INSERT mode.
    pub status_mode_insert: Color,

    // General
    /// Application background (used for clearing areas).
    pub background: Color,
}

impl Theme {
    /// Returns the built-in dark theme using ANSI 16 colors.
    ///
    /// Works on all terminals: 16-color, 256-color, and truecolor. Suitable
    /// as the default when no config is present or color capability is unknown.
    pub fn dark() -> Self {
        Self {
            border_active: Color::Cyan,
            border_inactive: Color::DarkGray,

            diff_added: Color::Green,
            diff_removed: Color::Red,
            diff_context: Color::Reset,
            diff_hunk_header: Color::Cyan,

            file_added: Color::Green,
            file_removed: Color::Red,
            file_modified: Color::Yellow,

            badge_critical: Color::Red,
            badge_major: Color::Yellow,
            badge_minor: Color::Blue,
            badge_info: Color::DarkGray,

            status_bar_bg: Color::DarkGray,
            status_bar_fg: Color::White,
            status_mode_normal: Color::Cyan,
            status_mode_insert: Color::Green,

            background: Color::Reset,
        }
    }

    /// Returns the Catppuccin Mocha theme using RGB truecolor values.
    ///
    /// Requires a truecolor terminal. Falls back gracefully in ratatui — colors
    /// degrade to the nearest ANSI 256-color approximation on non-truecolor terms,
    /// but visual fidelity is reduced. Use `dark()` on SSH or 256-color terminals.
    ///
    /// Palette source: <https://github.com/catppuccin/catppuccin> Mocha variant.
    pub fn catppuccin_mocha() -> Self {
        // Catppuccin Mocha palette (selected subset)
        let green = Color::Rgb(166, 227, 161);   // #a6e3a1
        let red = Color::Rgb(243, 139, 168);     // #f38ba8
        let yellow = Color::Rgb(249, 226, 175);  // #f9e2af
        let blue = Color::Rgb(137, 180, 250);    // #89b4fa
        let teal = Color::Rgb(148, 226, 213);    // #94e2d5
        let lavender = Color::Rgb(180, 190, 254); // #b4befe
        let overlay1 = Color::Rgb(127, 132, 156); // #7f849c
        let surface1 = Color::Rgb(69, 71, 90);   // #45475a
        let base = Color::Rgb(30, 30, 46);       // #1e1e2e
        let text = Color::Rgb(205, 214, 244);    // #cdd6f4
        let peach = Color::Rgb(250, 179, 135);   // #fab387

        Self {
            border_active: lavender,
            border_inactive: overlay1,

            diff_added: green,
            diff_removed: red,
            diff_context: text,
            diff_hunk_header: teal,

            file_added: green,
            file_removed: red,
            file_modified: yellow,

            badge_critical: red,
            badge_major: peach,
            badge_minor: blue,
            badge_info: overlay1,

            status_bar_bg: surface1,
            status_bar_fg: text,
            status_mode_normal: lavender,
            status_mode_insert: green,

            background: base,
        }
    }

    /// Resolves a theme name string to the corresponding built-in theme.
    ///
    /// Unknown names fall back to `dark()` so a typo in config never prevents
    /// startup. The fallback is logged to stderr (not a hard error).
    ///
    /// # Arguments
    ///
    /// * `name` — theme name from config, e.g. `"dark"` or `"catppuccin-mocha"`.
    pub fn from_name(name: &str) -> Self {
        match name {
            "catppuccin-mocha" | "catppuccin_mocha" => Self::catppuccin_mocha(),
            "dark" => Self::dark(),
            other => {
                eprintln!(
                    "airev: unknown theme '{}', falling back to 'dark'",
                    other
                );
                Self::dark()
            }
        }
    }
}
```

**Step D — Update airev/src/main.rs.**

Add `mod theme;` with the other mod declarations (after `mod ui;`).

Add a config-loading helper just above `async fn main()` — a private free function, not
inside main, to keep main under 50 lines:

```rust
/// Loads the theme name from `~/.config/airev/config.toml`.
///
/// Returns `"dark"` if the file does not exist, cannot be parsed, or has no
/// `theme` key. Never panics — config errors are soft failures printed to stderr.
fn load_theme_name() -> String {
    let config_path = dirs_or_manual_path();
    let raw = match std::fs::read_to_string(&config_path) {
        Ok(s) => s,
        Err(_) => return "dark".to_owned(),
    };
    let table: toml::Table = match toml::from_str(&raw) {
        Ok(t) => t,
        Err(e) => {
            eprintln!("airev: config parse error in {:?}: {}", config_path, e);
            return "dark".to_owned();
        }
    };
    table
        .get("theme")
        .and_then(|v| v.as_str())
        .unwrap_or("dark")
        .to_owned()
}

/// Returns the path to the airev config file.
///
/// Prefers `$XDG_CONFIG_HOME/airev/config.toml`; falls back to
/// `~/.config/airev/config.toml` when the env var is absent.
fn config_path() -> std::path::PathBuf {
    let base = std::env::var("XDG_CONFIG_HOME")
        .ok()
        .map(std::path::PathBuf::from)
        .or_else(|| {
            std::env::var("HOME")
                .ok()
                .map(|h| std::path::PathBuf::from(h).join(".config"))
        })
        .unwrap_or_else(|| std::path::PathBuf::from(".config"));
    base.join("airev").join("config.toml")
}
```

Wait — `load_theme_name` calls `dirs_or_manual_path()` which doesn't exist. Simplify:
merge them. `load_theme_name` calls `config_path()` directly. Here is the corrected pair:

```rust
fn config_path() -> std::path::PathBuf {
    let base = std::env::var("XDG_CONFIG_HOME")
        .ok()
        .map(std::path::PathBuf::from)
        .or_else(|| {
            std::env::var("HOME")
                .ok()
                .map(|h| std::path::PathBuf::from(h).join(".config"))
        })
        .unwrap_or_else(|| std::path::PathBuf::from(".config"));
    base.join("airev").join("config.toml")
}

fn load_theme_name() -> String {
    let path = config_path();
    let raw = match std::fs::read_to_string(&path) {
        Ok(s) => s,
        Err(_) => return "dark".to_owned(),
    };
    let table: toml::Table = match toml::from_str(&raw) {
        Ok(t) => t,
        Err(e) => {
            eprintln!("airev: config parse error in {:?}: {}", path, e);
            return "dark".to_owned();
        }
    };
    table
        .get("theme")
        .and_then(|v| v.as_str())
        .unwrap_or("dark")
        .to_owned()
}
```

In `async fn main()`, add theme loading as Step 0 (before the panic hook, since it is
read-only and cannot corrupt terminal state):

```rust
// Step 0: load theme from config — read-only, safe before terminal init.
let theme = theme::Theme::from_name(&load_theme_name());
```

Update the Render arm draw call from:
```rust
terminal.draw(|frame| ui::render(frame))?;
```
to:
```rust
terminal.draw(|frame| ui::render(frame, &theme))?;
```

**Step E — Update airev/src/ui.rs.**

Add `use crate::theme::Theme;` import.
Update the `render` signature from `pub fn render(frame: &mut Frame)` to
`pub fn render(frame: &mut Frame, theme: &Theme)`.
Update the module-level docstring to mention the theme parameter.

Apply border colors to all three panels using `ratatui::style::Style`:

```rust
use ratatui::style::Style;

frame.render_widget(
    Block::default()
        .title("Files")
        .borders(Borders::ALL)
        .border_style(Style::default().fg(theme.border_inactive)),
    chunks[0],
);
frame.render_widget(
    Block::default()
        .title("Diff")
        .borders(Borders::ALL)
        .border_style(Style::default().fg(theme.border_inactive)),
    chunks[1],
);
frame.render_widget(
    Block::default()
        .title("Comments")
        .borders(Borders::ALL)
        .border_style(Style::default().fg(theme.border_inactive)),
    chunks[2],
);
```

Note: All three panels use `border_inactive` in Phase 1 because there is no focus model yet.
Phase 2 will pass the focused panel index and switch the active panel to `border_active`.
Add a comment above the render_widget calls: `// Phase 2 will switch the focused panel to theme.border_active`.

After all edits, run `cargo build --workspace` and fix any compiler errors before marking done.
  </action>
  <verify>
```bash
cargo build --workspace
```
Must exit 0 with zero errors. Then:
```bash
echo q | cargo run -p airev -- 2>/dev/null; echo "exit: $?"
```
Must exit 0. Then confirm theme.rs exists:
```bash
ls airev/src/theme.rs
```
  </verify>
  <done>
`cargo build --workspace` exits 0. `airev/src/theme.rs` exists with `Theme`, `dark()`,
`catppuccin_mocha()`, and `from_name()`. `ui::render` accepts `&Theme` and applies
`border_inactive` to all three panels. `main.rs` loads theme from XDG config path,
defaulting to `"dark"` when config is absent.
  </done>
</task>

</tasks>

<verification>
1. `.planning/phases/01-foundation/01-04-PLAN.md` has a `<task type="auto">` block that
   appears before the `<task type="checkpoint:human-verify">` block.
2. The auto task contains cargo build, WAL mode sqlite3 check, and startup timing scripts.
3. `airev/src/theme.rs` defines `Theme` struct with all required fields and two constructors.
4. `cargo build --workspace` exits 0.
5. `ui::render(frame, &theme)` compiles with the new signature.
6. `main.rs` calls `Theme::from_name(&load_theme_name())` before terminal init.
</verification>

<success_criteria>
Phase 1 checkpoint plan has automated pre-checks that run before the human. The theme
system is in place so Phase 2 can use `theme.border_active` on the focused panel without
any structural changes to the color abstraction.
</success_criteria>

<output>
After completion, create `.planning/quick/1-automated-self-verification-before-human/1-SUMMARY.md`
following the template at @/Users/juliusalexandre/.claude/get-shit-done/templates/summary.md
</output>
