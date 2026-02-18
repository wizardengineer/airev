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
        let green = Color::Rgb(166, 227, 161);    // #a6e3a1
        let red = Color::Rgb(243, 139, 168);      // #f38ba8
        let yellow = Color::Rgb(249, 226, 175);   // #f9e2af
        let blue = Color::Rgb(137, 180, 250);     // #89b4fa
        let teal = Color::Rgb(148, 226, 213);     // #94e2d5
        let lavender = Color::Rgb(180, 190, 254); // #b4befe
        let overlay1 = Color::Rgb(127, 132, 156); // #7f849c
        let surface1 = Color::Rgb(69, 71, 90);   // #45475a
        let base = Color::Rgb(30, 30, 46);        // #1e1e2e
        let text = Color::Rgb(205, 214, 244);     // #cdd6f4
        let peach = Color::Rgb(250, 179, 135);    // #fab387

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
