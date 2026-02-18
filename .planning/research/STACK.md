# Stack Research

**Domain:** Rust terminal TUI — code review tool with live file watching, git diff rendering, SQLite persistence, MCP stdio server
**Researched:** 2026-02-17
**Confidence:** HIGH (all versions verified against crates.io API; architecture patterns verified against official ratatui docs and ecosystem sources)

---

## Recommended Stack

### Core Technologies

| Technology | Version | Purpose | Why Recommended |
|------------|---------|---------|-----------------|
| `ratatui` | 0.30.0 | TUI rendering framework | Dominant Rust TUI crate (18k+ stars, 2300+ dependent crates). v0.30 reorganized into a modular workspace (`ratatui-core` + `ratatui-widgets`) with improved compile times. Active development with stable API guarantees. The only serious TUI framework in the Rust ecosystem since `tui-rs` was abandoned. |
| `crossterm` | 0.29.0 | Terminal backend / keyboard events | Default ratatui backend. Cross-platform (Linux/macOS/Windows). Provides async `EventStream` via `event-stream` feature flag, required for non-blocking keyboard input in Tokio. `termion` is Linux-only; crossterm is the only viable cross-platform choice. |
| `tokio` | 1.49.0 | Async runtime | Required for: async keyboard event stream (crossterm), non-blocking file watcher (notify), background DB calls (tokio-rusqlite), rmcp stdio server. The `full` feature set needed. Ratatui's async-template is Tokio-based — the ecosystem converges here. |
| `git2` | 0.20.4 | Git operations (staged diff, commit range, branch comparison) | Mature libgit2 bindings with stable API. Supports all three diff modes needed (staged index, commit range, branch comparison) via typed `Diff` API. `gix`/gitoxide is the pure-Rust future but still lacks write operations and is pre-1.0. `git2` is what GitUI, Cargo, and production tools use in 2025/2026. |
| `notify` | 8.2.0 | Filesystem watcher (live file change events) | The canonical cross-platform file watching crate for Rust. Uses inotify (Linux), FSEvents (macOS), ReadDirectoryChangesW (Windows). Stable v8.x is the production choice; v9 RC2 is available but pre-release. Pairs with `tokio::sync::mpsc` bridge for async-compatible event delivery. |
| `rusqlite` + `tokio-rusqlite` | 0.38.0 + 0.7.0 | SQLite persistence with async access | `rusqlite` is synchronous; `tokio-rusqlite` wraps it with a background thread + mpsc/oneshot channel pattern so DB calls become `.await`-able without blocking the UI loop. This is the correct pattern for a Tokio-based TUI: DB work offloaded to a dedicated OS thread, UI stays responsive. Pure safe Rust (`forbid(unsafe_code)`). |
| `rmcp` | 0.16.0 | MCP server via stdio transport | The **official** Rust SDK for Model Context Protocol (modelcontextprotocol/rust-sdk). Released 2026-02-17. `stdio()` transport is first-class: `Counter::new().serve(stdio()).await?`. Provides `#[tool]` proc macro for zero-boilerplate tool registration. Active release cadence (0.1 in March 2025 → 0.16 by Feb 2026). |

### Supporting Libraries

| Library | Version | Purpose | When to Use |
|---------|---------|---------|-------------|
| `similar` | 2.7.0 | Diff computation (unified, inline) | Use for computing line-level and character-level diffs from git blob content. Patience algorithm with unicode grapheme support via feature flag. More ergonomic than `diffy` for inline character-level diff (which highlights exactly what changed within a line). Always. |
| `syntect` | 5.3.0 | Syntax highlighting for diff content | Sublime Text syntax definitions covering 500+ languages. Pure-Rust regex engine since v4.0 (no Oniguruma C dependency). Use for coloring source code lines within the diff panel. |
| `syntect-tui` | 3.0.6 | Bridge between syntect styles and ratatui `Style` | Converts syntect's `Color`/`Style` types to ratatui equivalents. Required glue layer — without it you'd hand-roll color conversion. Always use alongside syntect in ratatui. |
| `tui-textarea` | 0.7.0 | Multi-line text input widget | Use for inline comment entry. Supports Vim and Emacs keybindings (essential for vim-native UX target). Backend-agnostic with search/regex feature flag. More battle-tested and stable than `edtui` for basic multi-line input. |
| `tui-scrollview` | 0.6.2 | Scrollable content area larger than terminal | Use for the diff panel — diff content routinely exceeds terminal height. Renders any widget into a virtual canvas, supports both vertical and horizontal scrolling. Updated Dec 2025. |
| `notify-debouncer-full` | 0.7.0 | Debounce rapid filesystem events | File saves generate bursts of Create+Write events. Without debouncing, diff recomputation triggers 10-20x per save. `notify-debouncer-full` (part of the notify-rs ecosystem) coalesces events within a configurable timeout window. Use it as a wrapper over `notify`. |
| `anyhow` | 1.0.101 | Error handling | Standard ergonomic error type for application code (vs library code). Use throughout — avoid unwrap in the event loop. |
| `serde` + `serde_json` | 1.0.228 + 1.0.149 | Serialization for MCP protocol types, config | Required by `rmcp` for tool schema generation via `schemars`. Also useful for persisting structured session metadata. |
| `clap` | 4.5.59 | CLI argument parsing | Startup flags: `--repo-path`, `--diff-mode`, `--session-id`. The standard choice; nothing else is competitive in the Rust CLI space. |
| `tracing` | 0.1.44 | Structured logging | Use `tracing-subscriber` with a file appender (NOT stderr — terminal is occupied by ratatui). Instrument background tasks (file watcher, DB calls, MCP server) for debugging. |
| `unicode-width` | 0.2.2 | Correct column width for unicode/CJK characters | Required for accurate diff line alignment when source contains multi-byte characters. Ratatui uses this internally; use it directly when computing scroll offsets. |

### Development Tools

| Tool | Purpose | Notes |
|------|---------|-------|
| `cargo` + `rustfmt` | Build and format | Standard. Run `cargo fmt` before commits. |
| `clippy` | Linting | Run `cargo clippy -- -D warnings` in CI. Pay attention to clippy's suggestions on the event loop. |
| `cargo-watch` | Dev iteration | `cargo watch -x run` for rapid rebuild during development. |
| `insta` | Snapshot testing | Use for testing diff rendering output — snapshot the ratatui `Buffer` state rather than visual output. |

---

## Installation

```toml
[package]
name = "airev"
version = "0.1.0"
edition = "2021"
rust-version = "1.80"

[dependencies]
# TUI
ratatui = "0.30"
crossterm = { version = "0.29", features = ["event-stream"] }

# Async runtime
tokio = { version = "1.49", features = ["full"] }

# Git
git2 = "0.20"

# File watching
notify = "8.2"
notify-debouncer-full = "0.7"

# Database
rusqlite = { version = "0.38", features = ["bundled"] }
tokio-rusqlite = "0.7"

# MCP server
rmcp = { version = "0.16", features = ["server", "transport-io"] }
schemars = "1.0"

# Diff + syntax highlighting
similar = { version = "2.7", features = ["inline", "unicode"] }
syntect = { version = "5.3", default-features = false, features = ["default-fancy"] }
syntect-tui = "3.0"

# TUI widgets
tui-textarea = { version = "0.7", features = ["search"] }
tui-scrollview = "0.6"

# Supporting
serde = { version = "1.0", features = ["derive"] }
serde_json = "1.0"
anyhow = "1.0"
clap = { version = "4.5", features = ["derive"] }
tracing = "0.1"
tracing-subscriber = { version = "0.3", features = ["env-filter", "json"] }
unicode-width = "0.2"

[dev-dependencies]
insta = "1"
```

**Note on `syntect` features:** Use `default-fancy` to avoid the Oniguruma C dependency. The `fancy-regex` engine covers all practical syntax definition needs and compiles without any system library requirements.

**Note on `rusqlite` bundled feature:** Use `bundled` to embed SQLite statically. Eliminates the system SQLite version dependency — critical for reproducible builds.

---

## Alternatives Considered

| Category | Recommended | Alternative | Why Not |
|----------|-------------|-------------|---------|
| TUI framework | `ratatui` 0.30 | `cursive` | Cursive is retained-mode with an OOP widget model — ratatui's immediate-mode is a better fit for a high-frequency diff viewer that updates on every file save. Ratatui has 5x the ecosystem size. |
| Terminal backend | `crossterm` | `termion` | termion is Linux/macOS only — no Windows support. crossterm is the ratatui default and works everywhere. |
| Git library | `git2` | `gix` (gitoxide) | `gix` 0.79 is pre-1.0 with incomplete write operations (no push, no rebase, no commit hooks). `git2` is what GitUI uses and is battle-tested. Revisit `gix` in 2027 when it reaches 1.0. |
| Git library | `git2` | Raw `git` command (`std::process::Command`) | Spawning `git` processes in a tight render loop is expensive. `git2` gives a typed API without process spawn overhead. Raw commands are acceptable for one-off operations but not for live diff polling. |
| SQLite async | `rusqlite` + `tokio-rusqlite` | `sqlx` (SQLite backend) | `sqlx` 0.9 is still alpha; 0.8 stable works but its async SQLite is still backed by a blocking thread pool, same as `tokio-rusqlite`. `tokio-rusqlite` has simpler setup, smaller dependency surface, and pairs directly with rusqlite's full API. `sqlx` shines for multi-database projects — airev is SQLite-only. |
| SQLite async | `rusqlite` + `tokio-rusqlite` | `async-sqlite` | `async-sqlite` adds a connection pool which is unnecessary for a single-user TUI. `tokio-rusqlite` 0.7 is more widely used and has `forbid(unsafe_code)`. |
| MCP SDK | `rmcp` | `rust-mcp-sdk` | `rmcp` is the **official** SDK from `modelcontextprotocol` organization. `rust-mcp-sdk` is community-maintained and follows the official protocol spec, but official beats community when both are viable. |
| MCP SDK | `rmcp` | `mcp-sdk-rs` | Less active, smaller community, not official. `rmcp` released 0.16 on 2026-02-17 showing strong velocity. |
| Diff computation | `similar` | `diffy` | `diffy` 0.4.2 is primarily focused on patch-format diffs. `similar` supports inline character-level diffs (highlighting changed words within a line), which is essential for a code review diff viewer. `similar` also has better unicode support via the `unicode` feature. |
| File watching | `notify` 8.2 | `notify` 9.0-rc.2 | v9 is pre-release (RC2 as of 2026-02-14). Use stable v8.2 for production. Monitor the v9 release — the API is compatible enough that upgrading should be straightforward. |
| Text input widget | `tui-textarea` | `edtui` | `edtui` has experimental syntax highlighting and system clipboard — nice features, but "experimental" status is risky for a core interaction component. `tui-textarea` is the more stable, widely deployed choice. Use `edtui` if vim-modal editing in the comment box becomes a hard requirement. |
| Scrollable panel | `tui-scrollview` | `ratatui` built-in `List` with scroll state | Built-in `List` requires content to be pre-computed into items. A diff panel has variable-length content that overflows the viewport; `tui-scrollview` handles arbitrary widget content. |

---

## What NOT to Use

| Avoid | Why | Use Instead |
|-------|-----|-------------|
| `tui-rs` | Abandoned predecessor to ratatui. No updates since 2023. Ecosystem has fully migrated to ratatui. | `ratatui` 0.30 |
| `gitoxide` (`gix`) CLI binaries (`gix`, `ein`) | Explicitly marked unstable by the project — must not be used in scripts or automation. | `git2` crate for programmatic access |
| `std::sync::mpsc` for inter-task communication | Blocks the async executor if used across thread boundaries. | `tokio::sync::mpsc` throughout |
| `println!` / `eprintln!` for logging | ratatui takes control of the terminal; any stdout/stderr write corrupts the rendering. | `tracing` + file-based subscriber |
| `flask` / Python web frameworks | Not applicable (hard ban from project standards). | N/A |
| Relative imports (`..`) | Hard ban from project standards. | Absolute module paths |
| `sqlx` 0.9.x | Still in alpha as of 2026-02-17. Unstable API. | `rusqlite` + `tokio-rusqlite` 0.7 |

---

## Stack Patterns by Variant

**For the diff rendering pipeline:**
- Use `git2` to extract blob bytes for old/new versions of a file
- Pass bytes through `similar::TextDiff` to compute hunks and inline changes
- Apply `syntect` highlighting to each line based on file extension
- Convert syntect `Style` to ratatui `Style` via `syntect-tui`
- Render styled `Line`/`Span` widgets inside `tui-scrollview`

**For the live file watch loop:**
- `notify::recommended_watcher` with `notify-debouncer-full` wrapper
- Bridge to Tokio via `tokio::sync::mpsc` channel (blocking_send in watcher callback)
- Receive events in main Tokio task, trigger diff recomputation
- Debounce window: 200ms is a good starting point (avoids editor "atomic save" multi-event bursts)

**For the MCP server:**
- Run as a separate Tokio task: `tokio::spawn(mcp_service.serve(stdio()).await)`
- MCP server and TUI share app state via `Arc<Mutex<AppState>>`
- MCP tools expose: `list_sessions`, `get_comments`, `add_comment`, `get_diff`
- The stdio transport reads from stdin / writes to stdout — ratatui must use alternate screen (`EnterAlternateScreen`) so MCP JSON-RPC and TUI output go to different file descriptors

**If the project is SQLite-only and single-user (which airev is):**
- Use `tokio-rusqlite` with a single `Connection` instance
- No connection pool needed — SQLite WAL mode handles concurrent reads if needed later
- Enable WAL mode on first open: `PRAGMA journal_mode=WAL`

---

## Version Compatibility

| Package | Compatible With | Notes |
|---------|-----------------|-------|
| `ratatui` 0.30 | `crossterm` 0.29 | ratatui 0.30 ships with `crossterm` 0.29 as its default backend. They must match. |
| `rusqlite` 0.38 | `tokio-rusqlite` 0.7 | `tokio-rusqlite` 0.7 depends on `rusqlite` 0.32+; 0.38 is compatible. |
| `rmcp` 0.16 | `tokio` 1.x, `schemars` 1.0 | rmcp requires `schemars` 1.0 (not 0.8) for tool schema generation. This is a breaking change vs older schemars. |
| `syntect` 5.3 | `syntect-tui` 3.0.6 | syntect-tui 3.x targets syntect 5.x. Do not mix syntect 4.x with syntect-tui 3.x. |
| `notify` 8.2 | `notify-debouncer-full` 0.7 | These are co-released in the notify-rs monorepo. Always keep versions aligned. |
| `git2` 0.20 | `libgit2` system library | `git2` will try to link system libgit2. Set `LIBGIT2_NO_PKG_CONFIG=1` and use the `vendored` feature to bundle it: `git2 = { version = "0.20", features = ["vendored"] }` for reproducible builds. |

---

## Sources

- **crates.io API** — Version numbers verified programmatically on 2026-02-17 for all crates listed
- **docs.rs/rmcp 0.16** — rmcp stdio transport API, confirmed 0.16.0 released 2026-02-17
- **github.com/modelcontextprotocol/rust-sdk** — Official rmcp repository, confirmed official SDK status
- **ratatui.rs official docs** — Async event loop pattern, crossterm event-stream, 0.30 modular workspace architecture (HIGH confidence)
- **github.com/ratatui/async-template** — Official ratatui async template confirming tokio::sync::mpsc channel pattern (HIGH confidence)
- **github.com/notify-rs/notify** — notify-rs changelog, v8.2.0 stable confirmed 2025-08-03 (HIGH confidence)
- **github.com/rhysd/tui-textarea** — tui-textarea features and vim keybinding support (MEDIUM confidence)
- **github.com/joshka/tui-scrollview** — tui-scrollview scrollable diff panel widget (MEDIUM confidence)
- **shuttle.dev/blog/2025/07/18/how-to-build-a-stdio-mcp-server-in-rust** — rmcp stdio server walkthrough (MEDIUM confidence — verified against official docs)
- **users.rust-lang.org/t/rust-and-sqlite-which-one-to-use/90780** — rusqlite vs sqlx community analysis (MEDIUM confidence)
- **WebSearch: git2 vs gix 2025 production recommendation** — consensus: git2 for production write ops, gix for pure-Rust read-heavy (MEDIUM confidence — multiple sources agree)
- **WebSearch: similar vs diffy comparison** — similar chosen for inline character-level diff support (MEDIUM confidence)

---
*Stack research for: airev — Rust/ratatui terminal code review TUI*
*Researched: 2026-02-17*
