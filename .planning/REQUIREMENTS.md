# Requirements

## V1 (Current Milestone)

V1 ships a usable, daily-driver TUI for reviewing AI-generated code changes. A Neovim user who runs
Claude Code daily can replace `tuicr` with `airev` and get a richer review experience: 3-panel layout,
a more expressive comment taxonomy, SQLite-backed persistence, and a live diff view that updates as
Claude Code writes files. The MCP server completes the loop — Claude Code can read open comments and
respond to them programmatically.

### Core Features

#### Navigation and Keybindings

- `j`/`k` scroll within the focused panel (one line per keypress)
- `h`/`l` or `Ctrl-h`/`Ctrl-l` switch focus between panels (file list → diff → comments)
- `g` jumps to the top of the current panel; `G` jumps to the bottom
- `Ctrl-d`/`Ctrl-u` scroll half a page within the focused panel
- `Ctrl-f`/`Ctrl-b` scroll a full page within the focused panel
- `{`/`}` jump to the previous/next file in the file list
- `[h`/`]h` navigate to the previous/next hunk within the diff view
- `q` quits the application; prompts for confirmation only when there are unsaved comments
- `Esc` dismisses any modal (help overlay, comment entry, mode picker) and returns to normal mode
- `?` opens a contextual help overlay showing available keys for the current panel and mode
- All keybindings are consistent from first launch — no option to reconfigure in v1

#### Layout

- 3-panel layout: file list (left) | diff view (center) | comments panel (right)
- Default column widths: file list 20%, diff 55%, comments 25%
- `<` and `>` resize the diff panel by 5% per keypress, redistributing between file list and comments
- File list and comments panels are individually collapsible when terminal width is below 120 columns
- Minimum functional width: 80 columns (diff panel only, both side panels collapsed)
- Layout redraws correctly on terminal resize without requiring restart
- Single `terminal.draw()` call per render loop iteration — all overlays rendered inside one closure

#### Diff View

- Displays unified diff format with `+`/`-` line prefixes and `@@` hunk headers
- Syntax highlighting via `syntect` 5.3 + `syntect-tui` 3.0.6 using the `default-fancy` feature set
- Within-line (word-level) diff highlighting: changed words highlighted within a modified line, not just the full line
- Hunk context lines (unchanged lines surrounding changes) shown in muted color, distinct from additions and deletions
- Expandable context between hunks: pressing `Enter` on a `... N lines ...` separator expands 10 lines of context
- Diff panel is scrollable beyond terminal height using `tui-scrollview` 0.6.2; rendering is virtual (off-screen lines are not rendered)
- A diff with 1,000+ changed lines scrolls without frame drops at 60fps

#### File List Panel

- Lists all files in the current diff, sorted by: modified, added, deleted, renamed
- Each entry shows: status badge (M/A/D/R), filename, changed line count (+N/-N)
- Files matching `.aireignore` patterns are excluded from the list entirely
- `r` marks the current file as reviewed; marking is toggled, persisted in SQLite, and shown as a checkmark in the file list
- A status line below the file list shows reviewed count: `3/12 reviewed`
- Pressing `Enter` on a file in the file list moves focus to the diff panel at the top of that file's diff

#### Comments Panel

- Lists all comments for the currently focused file, in line-number order
- Each comment entry shows: line number, comment type badge (colored), severity badge (colored), first line of comment text
- Comment type badge colors: question=blue, concern=orange, til=green, suggestion=cyan, praise=yellow, nitpick=gray
- Severity badge colors: critical=red, major=orange, minor=yellow, info=gray
- `j`/`k` navigate between comments when focus is in the comments panel
- `Enter` on a comment in the comments panel scrolls the diff view to the corresponding line

#### Inline Commenting

- `c` in the diff view opens comment entry mode anchored to the line under the cursor
- Comment entry mode presents: a type picker, a severity picker, and a text input area
- Type picker cycles through all 6 types on `Tab`; selected type is displayed prominently
- Severity picker cycles through 4 severity levels on `Shift-Tab`; selected severity is displayed prominently
- Both type and severity are mandatory — comment cannot be saved without selecting both (no default)
- Text input uses `tui-textarea` 0.7 with vim insert-mode keybindings: `i` to enter insert mode, `Esc` to return to normal mode within the textarea, standard vim cursor movement in normal mode
- `Ctrl-s` or `:w` saves the comment and closes comment entry mode
- `Esc` in normal mode within comment entry discards the comment after confirmation prompt
- Saved comments are immediately written to SQLite and appear in the comments panel without restart

#### Comment Taxonomy

- 6 comment types: `question`, `concern`, `til`, `suggestion`, `praise`, `nitpick`
- 4 severity levels: `critical`, `major`, `minor`, `info`
- Type and severity are orthogonal (any combination is valid — e.g., a `question` can be `critical`)
- Comments are stored with: file path, line number, hunk offset, type, severity, text, session ID, created timestamp

#### Git Diff Modes

- `airev` with no arguments opens in staged mode (equivalent to `git diff --staged`)
- `airev --unstaged` shows unstaged working tree changes (equivalent to `git diff`)
- `airev --range <ref1>..<ref2>` shows diff between two git refs (commits, tags, branches)
- `airev --branch` shows diff between current branch and `main` (equivalent to `git diff main..HEAD`)
- Mode is shown in the status bar at the bottom of the screen
- Switching diff modes during a session is not supported in v1 — start a new invocation

#### .aireignore

- `airev` reads `.aireignore` in the repository root at startup
- File follows gitignore pattern syntax; patterns match relative to repository root
- Files matching any pattern are excluded from the file list and diff view
- Default exclusions when no `.aireignore` exists: `package-lock.json`, `pnpm-lock.yaml`, `yarn.lock`, `Cargo.lock`, `*.lock`, `dist/`, `.next/`, `__pycache__/`, `*.pyc`, `node_modules/`
- Patterns in `.aireignore` replace (not extend) the default exclusions

#### SQLite Persistence

- All review state is stored in `.airev/reviews.db` relative to the repository root
- SQLite is opened in WAL mode on every connection: `PRAGMA journal_mode=WAL; PRAGMA busy_timeout=5000;`
- All write transactions use `BEGIN IMMEDIATE` (not `BEGIN DEFERRED`)
- `PRAGMA wal_checkpoint(TRUNCATE)` is called once on startup
- Schema is versioned via a `schema_version` table; migrations run automatically on startup
- Database schema (v1):
  - `sessions`: id (UUID), repo_path, diff_mode, diff_args, created_at, updated_at
  - `comments`: id (UUID), session_id (FK), file_path, line_number, hunk_offset, comment_type, severity, body, created_at
  - `file_review_state`: session_id (FK), file_path, reviewed (BOOLEAN), reviewed_at
- A session corresponds to one `airev` invocation with one diff mode and ref set
- On launch, `airev` detects an existing session for the same repo+mode+refs and offers to resume it (shows comment count)
- Resuming a session restores: comment list, file reviewed state, last focused file

#### Structured Markdown Export

- `y` in normal mode copies the currently focused comment to the clipboard via OSC 52
- `:clip` in command-line mode exports the full review for the current session to the clipboard via OSC 52
- OSC 52 is used exclusively for clipboard — no `pbcopy`, `xclip`, or `wl-copy` dependency
- Export format for a single comment:
  ```
  **[CONCERN / MAJOR]** `src/parser.rs:42`
  > Changed line text from the diff

  Comment body text here.
  ```
- Export format for full review (`:clip`):
  ```markdown
  # Review: <diff mode and refs>
  **Session:** <date>  **Files reviewed:** 3/12

  ## src/parser.rs

  ### Line 42 — CONCERN / MAJOR
  > Changed line text

  Comment body.

  ### Line 87 — QUESTION / INFO
  > Changed line text

  Comment body.

  ## src/main.rs
  ...
  ```
- Export includes all comments across all files in line-number order, grouped by file

#### Live File Watcher

- `airev --watch` enables live file watching mode (watching is opt-in, not always-on)
- When active, `airev` monitors the repository root recursively for file changes using `notify-debouncer-full` 0.7
- File changes are debounced at 200ms: the diff is recomputed 200ms after the last detected change, not on every event
- The watcher listens for `EventKind::Any` (not just `Modify`) to catch rename-based writes (Claude Code's write pattern on Linux)
- After debounce, diff is recomputed by re-running the configured diff mode (not by trusting event file paths)
- When the diff updates: the file list refreshes, the diff view scrolls to the first changed hunk, a one-line status message shows `[updated N files]` in the status bar for 3 seconds
- Comments anchored to lines that no longer exist after a file update are preserved with a `[stale]` badge; they are not deleted
- `.aireignore` patterns are applied to watcher events — changes to ignored files do not trigger a diff recompute
- The status bar shows a `[watching]` indicator when watch mode is active

#### MCP Server

- `airev-mcp` is a separate binary (not a flag on `airev`) sharing the same `.airev/reviews.db` file
- Configured in Claude Code's `mcpServers` as `command: "airev-mcp"` with `args: ["--db", "<path>"]`
- Uses `rmcp` 0.16 with `stdio()` transport — no network port, no environment variables required beyond the db path
- All MCP tools are read-write against the shared SQLite file using the same WAL + `BEGIN IMMEDIATE` config
- Exposed tools:

  **`list_sessions`**
  - Input: `{ repo_path?: string }`
  - Output: array of `{ id, repo_path, diff_mode, diff_args, created_at, comment_count }`

  **`get_session`**
  - Input: `{ session_id: string }`
  - Output: `{ id, repo_path, diff_mode, diff_args, created_at, comments: Comment[] }`

  **`list_comments`**
  - Input: `{ session_id: string, file_path?: string, severity?: string, comment_type?: string }`
  - Output: array of `{ id, file_path, line_number, comment_type, severity, body, created_at }`

  **`add_annotation`**
  - Input: `{ session_id: string, file_path: string, line_number: int, comment_type: string, severity: string, body: string }`
  - Output: `{ id, created_at }`
  - Validates `comment_type` against the 6-type enum; returns error on invalid type
  - Validates `severity` against the 4-level enum; returns error on invalid severity

  **`resolve_comment`**
  - Input: `{ comment_id: string, resolution_note?: string }`
  - Output: `{ id, resolved_at }`
  - Marks a comment as resolved; adds optional resolution note to the record
  - Resolved comments are shown with a `[resolved]` badge in the TUI comments panel

  **`get_hunk_context`**
  - Input: `{ session_id: string, file_path: string, line_number: int, context_lines?: int }`
  - Output: `{ hunk_header: string, lines: [{ number, content, change_type }] }`
  - Returns the diff hunk containing the given line, with `context_lines` (default 5) of surrounding context

- `airev-mcp` exits cleanly when stdin is closed (Claude Code terminating the MCP connection)

#### Color Themes

- 4 built-in themes: `dark`, `light`, `catppuccin-mocha`, `gruvbox-dark`
- Theme is set in `~/.config/airev/config.toml` under `[ui] theme = "catppuccin-mocha"`
- Default theme when no config file exists: `dark`
- Theme applies to: syntax highlighting colors, comment type badges, severity badges, panel borders, status bar, diff `+`/`-` line backgrounds
- No runtime theme switching in v1 (requires restart)

#### Help Overlay

- `?` opens a full-screen modal listing all keybindings grouped by context: Navigation, Diff View, File List, Comments, Comment Entry, Command Line
- The overlay is dismissed with `?`, `Esc`, or `q`
- Help overlay content is accurate — any keybinding listed must work as documented

#### Vim Command Line

- `:` in normal mode opens a command-line input at the bottom of the screen
- Supported commands:
  - `:w` — saves the current session (no-op if session is auto-saved, confirms to user)
  - `:q` — quits (prompts if unsaved comments exist)
  - `:wq` — save and quit
  - `:clip` — exports full review to clipboard via OSC 52
  - `:filter <severity>` — filters the comments panel to show only comments of the given severity; `:filter` with no argument clears the filter
- `Esc` cancels command-line input without executing
- Unknown commands show a one-line error in the status bar: `Unknown command: :foo`
- The `:` command-line input state is distinct from `/` search and `i` comment insert — the input state machine must not conflate them

#### Performance Requirements

- Startup to first rendered frame: under 100ms on a cold start (no warm cache assumed)
- Scrolling in the diff view: no frame drops at 60fps for diffs up to 2,000 changed lines
- SQLite reads on session resume: no loading spinner; all reads complete before the first frame
- Diff recompute on file change (watch mode): under 500ms from debounce completion to re-render
- Memory ceiling: under 100MB RSS for a diff with 500 changed files and 1,000 comments

#### Terminal Lifecycle and Safety

- Uses `ratatui::init()` (not `Terminal::new()`) so the panic hook is installed automatically
- On panic: raw mode is restored before the panic message is printed; terminal is left in a usable state
- `SIGTERM` handler: registered via `signal-hook`; restores raw mode and exits cleanly
- On clean exit (`q`, `:q`): raw mode is restored, alternate screen is exited, cursor is restored

### Nice-to-Have V1

These features are not blocking for v1 launch but would improve the experience. Implement if time allows, defer if not.

- **File search in file list:** `/` in the file list panel opens an inline filter-as-you-type input. Files not matching the query are hidden. `Esc` clears the filter. Does not depend on external `fzf`.

- **File-jump shortcut:** `gf` in the diff view opens the currently focused file in `$EDITOR` at the line under the cursor. Falls back to `vi` if `$EDITOR` is unset.

- **Session list view:** `:sessions` command opens a modal listing past sessions for the current repo (date, diff mode, comment count, reviewed count). `Enter` resumes a selected session.

- **Status bar diff stats:** Show total `+N/-N` changed lines in the status bar alongside the current diff mode.

- **`Ctrl-r` diff refresh:** Manually trigger a diff recompute in non-watch mode (useful when files have changed but watch mode is off).

- **Comment edit:** `e` on a selected comment in the comments panel opens the comment entry UI pre-populated with the existing comment for editing. Saves over the original record (does not create a new comment).

- **Comment delete:** `d` on a selected comment prompts for confirmation, then deletes the comment from SQLite.

---

## V2 (Future)

These features are explicitly deferred. Do not build them in v1.

- **Predict-before-reveal mode:** Hide the diff and ask the user to predict what changed before revealing. Builds on the comment system and session model but requires a separate UI mode and new data model fields. Deferred until the core review loop is proven daily.

- **SM-2 spaced repetition engine:** Track which code patterns the user found surprising or important (marked via `til` and `concern` comments) and resurface them on a spaced schedule. Requires a separate quiz UI, spaced repetition scheduling logic, and a new SQLite table. Deferred until v1 comment taxonomy is validated.

- **Neovim plugin (`airev.nvim`):** A Lua plugin that opens `airev` in a floating terminal window, passes the current buffer's git context, and reads comments back into the Neovim quickfix list. Deferred until the standalone TUI is proven. The lazygit model: ship the TUI first, build the editor integration after.

- **Tree-sitter syntax highlighting:** Replace `syntect` with tree-sitter for incremental, grammar-based highlighting. Higher quality on large files; handles embedded languages. Deferred because `syntect` is sufficient for v1 and tree-sitter adds significant build complexity.

- **Jujutsu (jj) VCS support:** Detect jj repos and use `jj diff` / `jj log` output. jj repos are git-backed so most operations work, but `jj` revset syntax for range selection differs from git. Deferred until the git path is stable.

- **Multi-commit review mode:** Browse and annotate multiple commits in sequence (like `git log --patch`). Adds a commit list panel and session-per-commit model. Deferred — the primary AI review workflow is reviewing uncommitted or latest changes, not historical commits.

- **Custom keybinding TOML config:** Allow users to remap any keybinding in `~/.config/airev/config.toml`. The TOML structure should be designed in v1 but all bindings should be hardcoded. Expose the config surface in v2 once the v1 keybinding design is validated.

---

## Out of Scope

These are explicit non-goals. Do not implement. Do not accept PRs for these.

- **GitHub/GitLab PR integration:** `airev` reviews code in flight before it is committed or pushed. It is not a PR review client. Use Markdown export to paste findings into a PR comment.
- **Mouse support:** The target audience disables mouse in their terminal. Mouse support breaks tmux scrolling and signals the wrong audience. Every action must be reachable with keyboard.
- **AI-generated review suggestions:** `airev` is the surface where a human reviews AI-generated code. Adding AI review suggestions inverts the responsibility. Tools like CodeRabbit solve a different problem.
- **Web preview or shareable URL:** `airev` is local-first. A web backend requires hosting, auth, and maintenance. Markdown export is the shareable artifact.
- **Real-time collaborative review:** The tool is designed for solo use. Multi-user sync requires CRDT or OT infrastructure that conflicts with the local SQLite model.
- **Built-in git operations (stage, commit, push):** `airev` is a review tool, not a git client. `lazygit` already exists. Use `:!git <command>` passthrough for power users.
- **PDF export:** Markdown export covers all real use cases. PDF is a novelty.
- **Desktop / OS notifications:** Fails in SSH headless contexts. Visual-only status updates in the TUI.
- **Plugin system:** Premature. TOML config covers 90% of customization. Add when a concrete need is identified.
- **Clipboard via pbcopy/xclip/wl-copy:** Fails over SSH. OSC 52 is the only supported clipboard mechanism.

---

## Technical Constraints

These are non-negotiable. Changing any of these requires a documented decision with rationale.

### Binary Architecture

- Two binaries in a Cargo workspace: `airev` (TUI) and `airev-mcp` (MCP server)
- A third crate `airev-core` contains shared types (`Session`, `Comment`, `Hunk`, `DiffLine`) and the SQLite schema/migrations
- `airev-mcp` does not link ratatui, crossterm, or any TUI dependency
- `airev` does not link rmcp or any MCP dependency
- The only IPC between the two processes is the shared `.airev/reviews.db` SQLite file

### Rust Stack (exact versions)

| Crate | Version | Notes |
|-------|---------|-------|
| `ratatui` | 0.30 | TUI framework — only viable option |
| `crossterm` | 0.29 | Must match ratatui 0.30 exactly |
| `tokio` | 1.49 (full) | Async runtime — required by crossterm EventStream and rmcp |
| `git2` | 0.20 | `features = ["vendored"]` for reproducible builds |
| `notify` | 8.2 | File watching — always bump with notify-debouncer-full |
| `notify-debouncer-full` | 0.7 | Always bump with notify |
| `rusqlite` | 0.38 | `features = ["bundled"]` — avoids system lib issues |
| `tokio-rusqlite` | 0.7 | Async SQLite bridge for the TUI process |
| `rmcp` | 0.16 | MCP SDK — requires `schemars` 1.0 (NOT 0.8) |
| `similar` | 2.7 | Within-line diff computation |
| `syntect` | 5.3 | Syntax highlighting |
| `syntect-tui` | 3.0.6 | syntect → ratatui color bridge |
| `tui-textarea` | 0.7 | Comment text input with vim keybindings |
| `tui-scrollview` | 0.6.2 | Scrollable diff panel |

### Event Architecture

- Single `tokio::mpsc::UnboundedChannel<AppEvent>` as the unified event queue
- Sources that send to the channel: crossterm keyboard events, notify file change events, git background thread results, tokio-rusqlite results, timer ticks
- App state lives in the main Tokio task with single ownership — no `Arc<Mutex<AppState>>` on the critical path
- All state mutation happens only in the main event loop, never inside background tasks

### Git Layer

- `git2::Repository` is not `Send` — it must be owned by a dedicated `std::thread::spawn` thread
- All git operations are dispatched to this thread via `crossbeam-channel` or `std::sync::mpsc`
- Results returned as owned `Vec<DiffHunk>` structs (not `git2::Diff` which borrows the repository)
- `spawn_blocking` must never be used for git2 operations

### SQLite Configuration

- WAL mode enabled on every connection open: `PRAGMA journal_mode=WAL`
- `PRAGMA busy_timeout=5000` on every connection open
- All write transactions use `BEGIN IMMEDIATE`
- `PRAGMA wal_checkpoint(TRUNCATE)` called once on startup from the TUI process
- The `airev-mcp` process uses synchronous `rusqlite` (no `tokio-rusqlite`) with the same WAL + `BEGIN IMMEDIATE` config

### MCP Transport

- `rmcp` 0.16 stdio transport: MCP server reads from stdin, writes to stdout
- `schemars` must be version 1.0 — version 0.8 silently fails at schema generation with rmcp 0.16
- `airev-mcp` exits when stdin is closed

### Rendering

- One `terminal.draw()` call per event loop iteration
- All overlays, modals, and panels rendered inside a single draw closure
- Diff panel uses virtual rendering: only lines within the visible viewport are rendered

---

## Success Criteria

V1 is done when all of the following are true:

1. `airev --staged` opens and displays the staged diff with syntax highlighting within 100ms on a repository with 50 changed files.

2. Pressing `c` on a diff line, selecting a type and severity, typing a comment, and pressing `Ctrl-s` saves the comment to `.airev/reviews.db` and displays it in the comments panel in the same session.

3. Closing `airev` and reopening it with the same diff mode and repo offers to resume the previous session; accepting the prompt restores all comments and file reviewed state.

4. `:clip` copies a formatted Markdown review to the clipboard (via OSC 52) that includes all comments with file, line, type, severity, and body — confirmed working over an SSH session.

5. `airev --watch` detects a file write (including rename-based writes from Claude Code) within 200ms and recomputes the diff, updating the file list and diff view without restarting.

6. `airev-mcp` is configured in Claude Code's `mcpServers`; `list_comments` returns the correct comments from the current session; `resolve_comment` marks a comment resolved and the TUI shows the `[resolved]` badge on next render.

7. The tool is used daily for at least one week of real Claude Code review sessions without data loss, terminal corruption on crash, or keybinding violations that break muscle memory.

8. A diff with 500 changed lines scrolls without frame drops and the process stays under 100MB RSS.
