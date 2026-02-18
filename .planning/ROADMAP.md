# Roadmap

## Milestone 1: MVP

---

### Phase 1: Foundation
**Goal:** A running TUI shell with full panic/signal safety, a healthy WAL-configured database, and
the two-binary architecture committed in code — before any feature is built on top of it.

**Plans:** 4/4 plans executed

Plans:
- [x] 01-01-PLAN.md — Cargo workspace + airev-core shared types and WAL DB initialization
- [ ] 01-02-PLAN.md — tui.rs (stderr backend, panic hook, SIGTERM) + event.rs (AppEvent bus)
- [ ] 01-03-PLAN.md — main.rs event loop + ui.rs blank 3-panel placeholder
- [x] 01-04-PLAN.md — Phase exit criteria verification checkpoint

**Delivers:**
- Cargo workspace with two binaries: `airev` (TUI crate) and `airev-mcp` (MCP server crate),
  plus `airev-core` shared-types crate (`Session`, `Comment`, `Hunk` structs, DB schema migrations)
- `tui.rs`: `Tui` struct using `ratatui::init()` (not `Terminal::new()`), panic hook installed
  automatically, `Drop` impl restoring terminal on normal exit, SIGTERM handler via `signal-hook`
  that sets an `AtomicBool` and drives the event loop to exit cleanly
- `event.rs`: `AppEvent` enum covering `Key`, `Resize`, `Tick`, `Render`, `FileChanged`,
  `GitResult`, `DbResult`, `Quit`; single `tokio::mpsc::UnboundedSender<AppEvent>` as the sole
  event bus; separate tick and render intervals (independent rates, per ratatui official pattern)
- `main.rs` event loop: `tokio::select!` on the unified channel, single `terminal.draw()` call per
  `Render` event, `KeyEventKind::Press` filter to prevent Windows double-fire, explicit
  `Event::Resize` handling to force relayout
- SQLite schema initialized via `airev-core`: `PRAGMA journal_mode=WAL`, `PRAGMA synchronous=NORMAL`,
  `busy_timeout(10s)`, `PRAGMA wal_checkpoint(TRUNCATE)` on open; `BEGIN IMMEDIATE` in all write
  paths; tables for `sessions`, `comments`, `threads` created with the multi-round shape from day one
- Blank 3-panel placeholder rendered (no content, just borders) confirming the draw loop works

**Exit criteria:** Intentionally panic the app and confirm the terminal restores without `reset`.
Send SIGTERM and confirm clean exit. Open the database with `sqlite3 .airev/reviews.db` and
confirm `PRAGMA journal_mode;` returns `wal`. The workspace builds with `cargo build --workspace`
with zero errors on both macOS and Linux. Automated pre-checks (cargo build exit code, sqlite3 WAL
mode query) run as a scripted auto task before the human checkpoint — the human is only asked to
verify what cannot be scripted (panic recovery, SIGTERM behavior, visual startup timing).

---

### Phase 2: Rendering Skeleton
**Goal:** A navigable 3-panel layout with vim keybindings and a mode indicator — enough UI
infrastructure that every subsequent phase can render and test its output.

**Plans:** 3/3 plans executed

Plans:
- [x] 02-01-PLAN.md — AppState + responsive 3-panel layout engine with status bar
- [x] 02-02-PLAN.md — vim keybinding dispatch + modal help overlay
- [x] 02-03-PLAN.md — main.rs wiring + human verification checkpoint

**Delivers:**
- `ui/layout.rs`: 3-panel `Constraint`-based layout (file list | diff view | comments), responsive
  to terminal width, panels collapsible at narrow widths (minimum 80-column usable), resize-safe
  (recomputes constraints on every `Render` event)
- Panel focus model: `H`/`L` (or `Ctrl-h`/`Ctrl-l`) moves focus between panels; active panel
  highlighted with a distinct border style
- Normal-mode navigation keybindings wired to `AppState`: `j`/`k` scroll within focused panel,
  `g`/`G` jump to top/bottom, `Ctrl-d`/`Ctrl-u` half-page scroll, `{`/`}` file jump, `[`/`]`
  hunk navigation, `q`/`Esc` quit (with unsaved-state confirmation guard)
- Mode indicator in status bar: always shows `NORMAL` or `INSERT` — never blank
- `?` help overlay: modal, rendered inside the single `terminal.draw()` closure using `Clear`
  widget, dismissed with `?` or `Esc`; lists all active keybindings for the current panel context
- Placeholder content in each panel (static strings) so layout proportions can be visually verified

**Exit criteria:** Open the TUI in a terminal at 80, 120, and 200 columns. Verify panels resize
correctly without broken borders. Cycle through panel focus with `H`/`L`. Navigate with `j/k/g/G`.
Open `?` overlay, dismiss it — no widget disappears or flickers. Resize the terminal window during
a running session and confirm layout recomputes immediately without stale geometry.

---

### Phase 3: Git Layer
**Goal:** The diff view renders real git content — syntax-highlighted, virtually scrolled, and
produced by a background thread that cannot freeze the UI — across all four diff modes.

**Plans:** 1/5 plans executed

Plans:
- [ ] 03-01-PLAN.md — Cargo deps + owned diff types (OwnedDiffHunk, FileSummary, DiffMode) + AppEvent::GitResult payload
- [ ] 03-02-PLAN.md — git/worker.rs background thread (4 diff modes + syntect highlighting) + AppState Phase 3 fields
- [ ] 03-03-PLAN.md — ui/diff_view.rs virtual List scrolling + ui/file_tree.rs real file summaries
- [ ] 03-04-PLAN.md — main.rs AsyncGit wiring + Tab mode keybinding + Enter/l file jump + status bar loading indicator
- [ ] 03-05-PLAN.md — automated pre-checks + human verification checkpoint

**Delivers:**
- `git/mod.rs`: `AsyncGit` facade; a single `std::thread::spawn` thread owns the
  `git2::Repository` for its lifetime and receives work requests via `crossbeam-channel`; results
  returned as owned `Vec<DiffHunk>` structs (not `git2::Diff` which borrows the repo) and sent
  back as `AppEvent::GitResult` on the unified channel; `git2::Repository` never crosses a thread
  boundary
- Four diff modes fully wired: unstaged workdir, staged index, commit range (`git log --patch`
  equivalent), branch comparison (`main..HEAD` style); mode-switch keybindings in status bar
- `ui/diff_view.rs`: manual virtualization — `Vec<Line>` stored in `AppState`, only
  `&lines[visible_start..visible_end]` passed to the `List` widget per frame; scroll offset tracked
  in `AppState`; tested with a real LLVM-scale diff (5000+ lines) before phase is declared complete
- Syntax highlighting via `syntect` 5.3 + `syntect-tui` 3.0.6 (`default-fancy` feature to avoid
  C dep); highlighting computed in the git background thread, not the render closure; result stored
  as `Vec<ratatui::text::Spans>` in `AppState`
- `ui/file_tree.rs`: file list panel populated from git diff output; shows modified/added/deleted
  per file; `Enter` or `l` jumps to that file in the diff panel; file count in status bar
- Within-line (word-level) diffs via `similar` 2.7 for modified lines
- Loading indicator in status bar ("Computing diff...") while the background thread works; UI
  remains fully navigable during computation

**Exit criteria:** Open a real repository with 100+ changed files and a 5000+ line diff. Confirm
the UI loads the first screenful in under 100ms with subsequent lines streaming in. Navigate with
`j/k/Ctrl-d/Ctrl-u` at 60fps (no frame drops). Switch between all four diff modes. Verify via
`rustc` that no `git2::Repository` value crosses a thread boundary (the compiler enforces this;
confirm the compiler accepts the code without `unsafe`).

---

### Phase 4: Persistence Layer
**Goal:** Every session and comment survives restart, the multi-round thread schema is in place,
and the DB is exercised enough to validate the write paths before the MCP server is built on top.

**Delivers:**
- DB schema (in `airev-core`) finalized for multi-round threads: `sessions` table (id, repo_path,
  created_at, last_opened_at), `comments` table (id, session_id, thread_id, file_path, hunk_id,
  line_number, comment_type, severity, body, created_at, resolved_at), `threads` table (id,
  session_id, status enum: open/addressed/resolved, round_number); foreign keys enforced
- `db/mod.rs` in `airev-tui`: `tokio-rusqlite` `Connection::call()` for all reads/writes; results
  returned as `AppEvent::DbResult` variants on the unified channel; no blocking DB calls on the
  async event loop
- Session lifecycle: create a new session on first launch in a repo, auto-resume the most recent
  open session on subsequent launches, display session metadata in the status bar
- `mark_file_reviewed` write path: toggle per file, persisted to `sessions` table, visible as a
  checkmark in the file list panel
- `airev-core` DB schema versioning: migrations table, `schema_version` checked on open, forward
  migrations applied automatically; no manual `sqlite3` intervention ever required

**Exit criteria:** Start a session, navigate the diff, mark two files as reviewed, quit. Reopen
the tool in the same repo and confirm the session resumes with the reviewed state intact. Inspect
the database directly with `sqlite3 .airev/reviews.db "SELECT * FROM threads;"` and confirm the
schema matches the multi-round design. Run concurrent writes from two processes and confirm no
`SQLITE_BUSY` errors appear in stderr.

---

### Phase 5: Comment UI
**Goal:** The full review loop is closed in static mode: a user can navigate a diff, attach a
typed-and-categorized comment to any hunk, and export the complete review as structured Markdown.

**Delivers:**
- Comment entry mode: `c` on a focused hunk opens `INSERT` mode in the comments panel; input
  handled by `tui-textarea` 0.7 (vim keybindings, multi-line, `Esc` to cancel, `Enter`+modifier
  or `:wq` to submit); `AppState.mode` transitions drive rendering (no overlapping modal states)
- Mandatory taxonomy selection before submit: Tab-cycles through the 6 comment types
  (question/concern/til/suggestion/praise/nitpick); severity selection (critical/major/minor/info)
  via separate binding; neither can be skipped — form does not submit without both
- Comments panel: renders all comments for the current file in thread order; type and severity
  displayed as colored badges (e.g., `[concern:critical]`); open/addressed/resolved thread status
  shown per thread
- Filter keybindings: filter comments panel by severity or type without leaving normal mode
- Structured Markdown export: `y` yanks the comment under cursor to OSC 52 clipboard;
  `:clip` exports the full session review in a format that includes file path, line number,
  severity, type, thread history (all rounds), and a code context snippet from the hunk; output
  designed to be pasted directly into a Claude Code conversation for one-pass resolution
- `.aireignore` support: gitignore-pattern file at repo root; matched files excluded from the
  file list and diff view; common noise excluded by default (lock files, `dist/`, `.next/`, etc.)

**Exit criteria:** Open a real diff. Navigate to a hunk. Press `c`, write a comment, select type
`concern` and severity `critical`, submit. Confirm the comment appears in the comments panel with
correct badges. Press `y` on the comment, open a text editor, paste — confirm the Markdown
includes the hunk context and taxonomy metadata. Filter to `critical` only and confirm only that
comment shows. Add a `.aireignore` entry for a file in the diff and confirm it disappears from
the file list on restart.

---

### Phase 6: Live File Watcher
**Goal:** The diff view auto-refreshes as Claude Code writes files to disk — the tool's primary
differentiator — working correctly on both macOS (FSEvents) and Linux (inotify).

**Delivers:**
- `watch/mod.rs`: `notify-debouncer-full` 0.7 watcher with a 300ms debounce window; watcher
  initialized in a `std::thread::spawn` thread; events bridged to the unified async channel via
  `tx.blocking_send(AppEvent::FileChanged(event))`; app registers for `EventKind::Any` (not
  `EventKind::Modify` only — required for Linux rename-based writes from Claude Code)
- After debounce fires, the watcher does NOT trust event file paths: it runs `git status --porcelain`
  via the existing `AsyncGit` facade to get the canonical changed-file list, then re-requests diffs
  only for changed paths
- Visual status indicator in the status bar: shows "Watching" when watcher is active, "Modified"
  for 2 seconds after a change is detected and diff is refreshed
- `.aireignore` integration: watcher skips debounce for ignored paths before triggering git status
- `PollWatcher` fallback for network filesystems and WSL paths where `RecommendedWatcher` fails
- Cross-platform verification: watcher behavior must be confirmed on both macOS and Linux
  (Docker on macOS is acceptable for Linux verification) before phase is declared complete

**Exit criteria:** Run the tool in watch mode. Edit a tracked file with vim (rename-based write
on Linux) and confirm the diff panel updates within 400ms. Edit three files in rapid succession
and confirm only one git-status call fires (debounce working). Verify on Linux via Docker that
the `Create` rename event is caught (not just `Modify`). Add a file matching `.aireignore` and
confirm watcher changes to it do not trigger a refresh.

---

### Phase 7: MCP Server
**Goal:** Claude Code can query the live review state and post responses to open comments via
the `airev-mcp` binary, with the full multi-round thread lifecycle exercised end-to-end.

**Delivers:**
- `airev-mcp` binary: `rmcp` 0.16 `stdio()` transport; `#[tool]` proc macro for all tool
  definitions; synchronous `rusqlite` (no async needed in the MCP server); opens the shared
  `.airev/reviews.db` with WAL mode + `busy_timeout` configured on every connection open
- Six MCP tools exposed:
  - `list_sessions` — returns all sessions for the current repo path
  - `get_session` — returns session metadata + file reviewed status
  - `list_comments` — returns all comments for a session, optionally filtered by file, severity,
    or type; includes thread status and round number
  - `get_hunk_context` — returns the diff hunk text associated with a comment (for Claude Code
    to read the code being discussed)
  - `add_annotation` — Claude Code posts a response to an open comment thread; increments
    `round_number`, sets thread status to `addressed`
  - `resolve_comment` — marks a thread as `resolved`; callable by either the TUI (human confirms)
    or Claude Code (agent self-resolves trivial items)
- Claude Code `mcpServers` configuration entry documented: `{ "command": "airev-mcp", "args":
  ["--db", "~/.local/share/airev/sessions.db"] }`
- Smoke test: configure the MCP server in a local Claude Code instance and call `list_comments`
  against a real session; confirm the response structure matches what the TUI renders

**Exit criteria:** With the TUI running and two open comment threads visible, configure
`airev-mcp` in Claude Code's MCP settings. From Claude Code, call `list_comments` and confirm
the response matches what the TUI shows. Call `add_annotation` and confirm the thread status
updates to `addressed` in the TUI on the next refresh. Call `resolve_comment` and confirm the
thread is marked resolved in both the TUI and via a direct `sqlite3` query. Run TUI writes and
MCP reads concurrently for 60 seconds and confirm zero `SQLITE_BUSY` errors.

---

### Phase 8: Polish and Compatibility
**Goal:** Every item on the "looks done but isn't" checklist passes, and the tool is correct on
SSH, at narrow terminal widths, with color-limited terminals, and under the jj VCS workflow.

**Delivers:**
- SSH clipboard: `:clip` and `y` use OSC 52 escape sequence when `$SSH_TTY` is set; graceful
  failure message in the status bar ("Clipboard unavailable — paste manually") when OSC 52
  is not supported; never silent failure
- Color capability detection: reads `$COLORTERM` environment variable; falls back to 256-color
  ANSI palette when `truecolor`/`24bit` not detected; all diff and badge colors verified to
  remain distinguishable in 256-color mode
- Color theme system: TOML config at `$XDG_CONFIG_HOME/airev/config.toml`; four built-in themes
  (catppuccin-mocha, gruvbox-dark, dark, light); theme key applied at startup, no live-reload
  required for v1
- SIGTERM handling: verified to restore terminal cleanly (not just SIGINT); `signal-hook`
  handler confirmed in integration test that kills the process with SIGTERM and asserts terminal
  state with `stty`
- Vim colon command line: `:` enters command mode (distinct input state from `/` search and `i`
  insert — state machine disambiguates explicitly); built-in commands: `:w` save session,
  `:q` quit, `:clip` export review, `:filter <severity>` filter comments panel,
  `:round <n>` jump to review round `n`
- Jujutsu (jj) VCS support: detect jj repo at startup (`.jj/` directory present); route diff
  operations through `jj diff` command output parsed into the same `DiffHunk` structs the git
  layer produces; jj repos are git-backed so most paths reuse the existing git layer
- "Looks done but isn't" checklist from PITFALLS.md verified line by line before phase closes

**Exit criteria:** Run the tool over `ssh localhost` and confirm `:clip` either copies via OSC 52
or shows the fallback message — not silent failure. Set a theme in `config.toml` and confirm it
loads. Open in an `xterm` (256-color only) and confirm diff colors remain visually distinct.
Send SIGTERM and confirm the terminal is restored (verified with `stty -a | grep echo`). Open
a jj repository and confirm the diff view populates. Run all items in the PITFALLS.md
"Looks Done But Isn't" checklist manually and confirm each passes.

---

## Key Decisions Locked

The following architectural decisions are committed in Phase 1 and must not change in any
subsequent phase. Changing them after Phase 1 requires a full architectural rewrite.

| Decision | What Is Locked | Why It Cannot Change Later |
|----------|---------------|---------------------------|
| Two-binary architecture | `airev` TUI and `airev-mcp` MCP server are separate binaries in the same Cargo workspace | MCP stdio transport owns stdin/stdout exclusively; TUI owns the terminal. They cannot share a process. Discovered late = full I/O layer rewrite. |
| TUI renders to stderr | `CrosstermBackend::new(BufWriter::new(std::io::stderr()))` is the terminal backend | Allows MCP server to own stdout/stdin if ever colocated; also the correct default given the two-binary design. Changing backends requires replacing all terminal initialization code. |
| WAL mode on all SQLite connections | `PRAGMA journal_mode=WAL` + `busy_timeout` + `BEGIN IMMEDIATE` on every write transaction | WAL enables concurrent TUI + MCP access. Retrofitting WAL after data is written requires a careful migration with potential for data loss under concurrent access. |
| Multi-round thread schema from day one | `comments`, `threads`, and `sessions` tables with `thread_id`, `round_number`, `resolution_status` from Phase 4 | A flat comment table cannot be migrated to a threaded model without losing comment-to-thread associations already stored. MCP server (Phase 7) is built against this schema. |
| git2 on a dedicated std::thread::spawn | `git2::Repository` owned by a single background thread for its lifetime; all git calls via channel | `git2::Repository` is not `Send`. The compiler enforces this. Any attempt to use `spawn_blocking` with `Repository` fails at compile time. This pattern is the only workable design. |
| Single terminal.draw() per event loop iteration | All widgets, overlays, and modals rendered inside one `terminal.draw()` closure | Ratatui's double-buffer model erases the working buffer at the start of each `draw()` call. A second `draw()` call wipes the first frame. Changing this requires restructuring all render code. |
