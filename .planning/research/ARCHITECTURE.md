# Architecture Research

**Domain:** Rust/ratatui terminal TUI with live file watching, git operations, SQLite persistence, and MCP server
**Researched:** 2026-02-17
**Confidence:** HIGH (primary sources: ratatui official docs, verified async patterns, real-world reference implementations)

---

## Standard Architecture

### System Overview

```
┌─────────────────────────────────────────────────────────────────────────┐
│                         Process: airev (TUI)                             │
│                                                                          │
│  ┌──────────────────────────────────────────────────────────────────┐   │
│  │                     Tokio Async Runtime                          │   │
│  │                                                                  │   │
│  │  ┌─────────────┐  ┌─────────────┐  ┌──────────────────────────┐ │   │
│  │  │ Event Loop  │  │ File Watch  │  │  Git Ops (asyncgit)      │ │   │
│  │  │ (crossterm  │  │ Task        │  │  Thread pool + crossbeam │ │   │
│  │  │  EventStream│  │ (notify +   │  │  channels for git2 calls │ │   │
│  │  │  + tick +   │  │  mpsc chan) │  │                          │ │   │
│  │  │  render)    │  └──────┬──────┘  └────────────┬─────────────┘ │   │
│  │  └──────┬──────┘         │                      │               │   │
│  │         │                └──────────┬───────────┘               │   │
│  │         ▼                           ▼                            │   │
│  │  ┌─────────────────────────────────────────────────────────┐    │   │
│  │  │         Unified Event Channel (tokio::mpsc)             │    │   │
│  │  │   AppEvent { Key, Mouse, Resize, Tick, Render,          │    │   │
│  │  │              FileChanged, GitResult, DbResult }         │    │   │
│  │  └──────────────────────┬──────────────────────────────────┘    │   │
│  │                         │                                        │   │
│  │                         ▼                                        │   │
│  │  ┌─────────────────────────────────────────────────────────┐    │   │
│  │  │               Main Event Dispatch Loop                  │    │   │
│  │  │   match event { ... } → update AppState → schedule draw │    │   │
│  │  └──────────┬──────────────────────────────────────────────┘    │   │
│  │             │                                                    │   │
│  │     ┌───────┴────────────────────┐                              │   │
│  │     ▼                            ▼                              │   │
│  │  ┌──────────────────┐  ┌────────────────────────────────────┐   │   │
│  │  │  Render Layer    │  │  SQLite Layer (tokio-rusqlite)     │   │   │
│  │  │  ratatui draw()  │  │  Background thread, oneshot/mpsc   │   │   │
│  │  │  3-panel layout  │  │  for async DB reads and writes     │   │   │
│  │  │  edtui widget    │  └────────────────────────────────────┘   │   │
│  │  └──────────────────┘                                            │   │
│  └──────────────────────────────────────────────────────────────────┘   │
└─────────────────────────────────────────────────────────────────────────┘

┌─────────────────────────────────────────────────────────────────────────┐
│                     Process: airev-mcp (MCP Server)                      │
│                                                                          │
│  stdin/stdout: JSON-RPC 2.0 (MCP protocol)                              │
│                                                                          │
│  ┌──────────────────────────────────────────────────────────────────┐   │
│  │  rmcp / rust-mcp-sdk (stdio transport)                           │   │
│  │       ↓                                                          │   │
│  │  Tool Handlers: list_comments, add_annotation, get_session, ...  │   │
│  │       ↓                                                          │   │
│  │  SQLite (rusqlite, read/write to shared .db file)                │   │
│  └──────────────────────────────────────────────────────────────────┘   │
└─────────────────────────────────────────────────────────────────────────┘

             Claude Code (MCP Client)
                     │
                     │  spawns as subprocess (stdio)
                     ▼
             airev-mcp process
                     │
                     │  reads/writes
                     ▼
             ~/.local/share/airev/sessions.db
                     ▲
                     │  reads/writes
             airev TUI process (via tokio-rusqlite)
```

---

## Component Responsibilities

| Component | Responsibility | Typical Implementation |
|-----------|----------------|------------------------|
| Event Router | Receive all events from all sources; dispatch to state | `tokio::mpsc` channel, `tokio::select!` in main loop |
| TUI Renderer | Immediate-mode rendering of 3-panel layout each frame | `ratatui::Terminal::draw()`, `crossterm` backend |
| Terminal Manager (`tui.rs`) | Terminal lifecycle: raw mode, alternate screen, cleanup | `Tui` struct with `Drop` impl restoring terminal |
| File Watcher Task | Watch working tree for changes; debounce events | `notify` crate + `tx.blocking_send()` → async channel |
| Git Layer (`asyncgit` pattern) | Offload `git2` calls to thread pool; signal completion | `crossbeam-channel` for notifications, thread pool |
| SQLite Layer | Persist sessions, comments, threads; async bridge | `tokio-rusqlite` `Connection::call()` pattern |
| App State | Single source of truth; mutated only in main loop | Plain Rust struct, no `Mutex` needed in main loop |
| Editor Widget | Inline vim-mode comment editing | `edtui` widget embedded in render cycle |
| MCP Server | Expose review data to Claude Code via JSON-RPC | Separate binary using `rmcp` crate, stdio transport |

---

## Recommended Project Structure

```
airev/
├── Cargo.toml                  # workspace root
├── Cargo.lock
├── crates/
│   ├── airev-tui/              # main TUI binary
│   │   ├── Cargo.toml
│   │   └── src/
│   │       ├── main.rs         # tokio::main, wires all layers
│   │       ├── app.rs          # AppState struct + transitions
│   │       ├── tui.rs          # Terminal lifecycle, Tui struct
│   │       ├── event.rs        # AppEvent enum + EventRouter
│   │       ├── action.rs       # Action enum (command pattern)
│   │       ├── ui/
│   │       │   ├── mod.rs      # top-level draw() call
│   │       │   ├── layout.rs   # 3-panel constraint layout
│   │       │   ├── file_tree.rs
│   │       │   ├── diff_view.rs
│   │       │   └── comment_panel.rs
│   │       ├── git/
│   │       │   ├── mod.rs      # AsyncGit facade
│   │       │   ├── diff.rs     # hunk parsing, diff navigation
│   │       │   └── commits.rs  # commit listing, navigation
│   │       ├── watch/
│   │       │   └── mod.rs      # notify watcher task
│   │       └── db/
│   │           └── mod.rs      # tokio-rusqlite queries
│   │
│   ├── airev-mcp/              # MCP server binary
│   │   ├── Cargo.toml
│   │   └── src/
│   │       ├── main.rs         # rmcp stdio server entry point
│   │       ├── tools/
│   │       │   ├── comments.rs # list_comments, add_comment
│   │       │   ├── sessions.rs # get_session, list_sessions
│   │       │   └── hunks.rs    # get_hunk_context
│   │       └── db.rs           # rusqlite (sync ok in MCP server)
│   │
│   └── airev-core/             # shared types (optional)
│       ├── Cargo.toml
│       └── src/
│           ├── lib.rs
│           ├── models.rs       # Session, Comment, Hunk types
│           └── db_schema.rs    # schema migrations
```

### Structure Rationale

- **Workspace with separate crates for TUI and MCP:** The MCP server must own stdio exclusively; the TUI owns the terminal. They cannot share a process. A workspace keeps types shared and build tooling unified.
- **`event.rs` centralizes all event types:** File watch events, git completion events, DB results, and terminal events all flow through one `AppEvent` enum. This is the only queue the main loop reads.
- **`git/` submodule mirrors `asyncgit` pattern:** All blocking `git2` calls are dispatched to a thread pool. Results come back as `AppEvent::GitResult` variants, never as direct return values in the event handler.
- **`db/` uses `tokio-rusqlite`:** SQLite is synchronous; `tokio-rusqlite` bridges it with a background thread + `oneshot` channels, keeping the main loop non-blocking.
- **`ui/` is pure render code:** No I/O, no state mutation. Takes `&AppState`, returns nothing. This makes it testable and keeps render fast.

---

## Architectural Patterns

### Pattern 1: Unified Event Channel with `tokio::select!`

**What:** All concurrent inputs (terminal events, file changes, git results, DB results, timer ticks) are funneled into a single `tokio::mpsc::UnboundedReceiver<AppEvent>` that the main loop reads sequentially.

**When to use:** Always — this is the canonical ratatui async pattern. Without it, you block the render loop waiting on one input source.

**Trade-offs:** Simple, predictable sequencing. Back-pressure is limited with unbounded channels; bounded channels add complexity but are safer under load (not needed for a TUI tool).

**Example:**
```rust
pub enum AppEvent {
    Key(KeyEvent),
    Mouse(MouseEvent),
    Resize(u16, u16),
    Tick,
    Render,
    FileChanged(notify::Event),
    GitResult(GitOutput),
    DbResult(DbResponse),
    Quit,
}

// In main loop:
loop {
    match tui.next_event().await {
        AppEvent::Render => { tui.draw(|f| ui::draw(f, &app))?; }
        AppEvent::Key(key) => { app.handle_key(key, &action_tx); }
        AppEvent::FileChanged(ev) => { app.on_file_change(ev); }
        AppEvent::GitResult(result) => { app.on_git_result(result); }
        AppEvent::Quit => break,
        _ => {}
    }
}
```

---

### Pattern 2: Separate Tick and Render Events (ratatui official pattern)

**What:** The event task produces both `Event::Tick` (for animation/polling state) and `Event::Render` (for redraw) at independently configurable rates. Rendering only happens on `Render` events, not every event.

**When to use:** Always for production ratatui apps. Prevents rendering on every keypress (unnecessary CPU) while keeping animation smooth.

**Trade-offs:** Adds two timers but trivially so via `tokio::time::interval`.

**Example:**
```rust
// In event task spawned with tokio::spawn:
tokio::select! {
    _ = tick_interval.tick() => { tx.send(AppEvent::Tick).ok(); }
    _ = render_interval.tick() => { tx.send(AppEvent::Render).ok(); }
    Some(ev) = event_stream.next() => {
        match ev? {
            CEvent::Key(k) if k.kind == KeyEventKind::Press =>
                tx.send(AppEvent::Key(k)).ok(),
            CEvent::Resize(w, h) => tx.send(AppEvent::Resize(w, h)).ok(),
            // ...
        }
    }
}
```

---

### Pattern 3: Async Git Layer (asyncgit model)

**What:** All `git2` calls are dispatched to a thread pool (via `tokio::task::spawn_blocking` or `crossbeam-channel` workers). Completion is signaled back as an `AppEvent::GitResult`.

**When to use:** Any git operation that could take >1ms (diff computation, commit log parsing, hunk staging). All of them qualify.

**Trade-offs:** Adds indirection — you request an operation and receive the result on the next event loop iteration. This is correct behavior; git can be slow on large repos.

**Example:**
```rust
// Dispatch from event handler:
let tx = app.event_tx.clone();
tokio::task::spawn_blocking(move || {
    let diff = git_compute_diff(&repo_path, &commit_oid);
    tx.blocking_send(AppEvent::GitResult(GitOutput::Diff(diff))).ok();
});

// Receive in main loop:
AppEvent::GitResult(GitOutput::Diff(diff)) => {
    app.diff_state.load(diff);
    // render will be triggered by next Render tick
}
```

---

### Pattern 4: File Watching via notify + blocking_send Bridge

**What:** The `notify` crate uses synchronous callbacks. Bridge to async via `tx.blocking_send()` into the unified event channel.

**When to use:** Required when `notify` watcher is used inside a Tokio runtime (which is always the case here).

**Trade-offs:** `blocking_send` parks a synchronous thread until the async receiver drains; this is fine since the event channel is unbounded and fast to drain.

**Example:**
```rust
let (tx, rx) = tokio::sync::mpsc::unbounded_channel::<AppEvent>();
let mut watcher = notify::recommended_watcher(move |res: notify::Result<notify::Event>| {
    if let Ok(event) = res {
        tx.blocking_send(AppEvent::FileChanged(event)).ok();
    }
})?;
watcher.watch(&repo_path, RecursiveMode::Recursive)?;
// rx is merged into the main event channel via a tokio task:
tokio::spawn(async move {
    while let Some(ev) = rx.recv().await {
        main_tx.send(ev).ok();
    }
});
```

Debounce rapid events (e.g., editor saves many files at once) with a 100ms debounce before sending:
```rust
// Use notify-debouncer-mini or notify-debouncer-full crates
// They wrap notify with built-in coalescing
```

---

### Pattern 5: SQLite via tokio-rusqlite (async bridge)

**What:** `rusqlite` is synchronous. `tokio-rusqlite` runs it in a background thread and exposes `Connection::call()` which returns a `Future`.

**When to use:** Any DB read or write from the TUI event handler or from the MCP server.

**Trade-offs:** Adds one thread per open connection. The `Connection` is cheaply cloneable so one connection per app is sufficient.

**Example:**
```rust
// TUI side (async):
let conn = tokio_rusqlite::Connection::open(&db_path).await?;
let session = conn.call(|c| {
    let mut stmt = c.prepare("SELECT * FROM sessions WHERE id = ?1")?;
    Ok(stmt.query_row([id], |row| Session::from_row(row))?)
}).await?;
app.event_tx.send(AppEvent::DbResult(DbResponse::Session(session))).ok();

// MCP server side (sync is fine — MCP server is not a TUI):
let conn = rusqlite::Connection::open(&db_path)?;
let comments = conn.prepare("SELECT * FROM comments WHERE session_id = ?1")?
    .query_map([session_id], |r| Comment::from_row(r))?
    .collect::<Result<Vec<_>>>()?;
```

---

## Critical: MCP Server + TUI Coexistence

### The Problem

A stdio MCP server owns `stdin`/`stdout` exclusively — JSON-RPC messages are sent and received on these streams. A ratatui TUI also requires exclusive terminal control (typically `stdout` or `stderr`). **They cannot coexist in the same process.**

This is not a workaround or edge case — it is a hard constraint of the MCP stdio transport specification: "The server MUST NOT write anything to stdout that is not a valid MCP message."

### The Solution: Two Separate Binaries, Shared Database

```
Claude Code (MCP client)
    │
    │  spawns via stdio transport
    ▼
airev-mcp (MCP server binary)
    │  rusqlite (sync, direct)
    ▼
~/.local/share/airev/sessions.db   ←── shared SQLite file
    ▲
    │  tokio-rusqlite (async)
airev (TUI binary, user runs this)
```

**How it works:**
1. The user runs `airev` — the TUI starts, owns the terminal, reads/writes the SQLite DB.
2. Claude Code is configured with the MCP server entry: `{ "command": "airev-mcp", "args": ["--db", "~/.local/share/airev/sessions.db"] }`.
3. When Claude Code invokes an MCP tool (e.g., `get_session`), it spawns `airev-mcp` as a subprocess. The MCP server reads from the DB and responds via stdout.
4. The TUI and MCP server access the same SQLite file. SQLite's WAL (Write-Ahead Log) mode handles concurrent readers + one writer safely.

**SQLite concurrency configuration required:**
```sql
PRAGMA journal_mode=WAL;
PRAGMA busy_timeout=5000;
```
With WAL mode, the TUI can write while the MCP server reads (and vice versa) without blocking. `busy_timeout` prevents immediate errors on lock contention.

**Why not SSE/HTTP transport instead?** SSE transport would require the MCP server to run as a persistent background service, adding a daemon management problem. stdio transport is zero-infrastructure — Claude Code spawns the process on demand. For a local developer tool, this is correct.

**Why not threads instead of processes?** The MCP server spawned by Claude Code must communicate on the exact file descriptors Claude Code opens. There is no way to share a thread's stdin/stdout with a parent process's pipe; the only clean model is a separate executable.

---

## Data Flow

### User Navigates the Diff View

```
User presses 'j' (next hunk)
    │
    ▼
crossterm EventStream
    │  AppEvent::Key(KeyEvent { code: Char('j'), .. })
    ▼
Main event dispatch loop
    │  app.handle_key(key) → dispatches Action::NextHunk
    ▼
Action handler
    │  if diff already loaded in AppState → update cursor position in AppState
    │  if diff not loaded → spawn_blocking git2 call
    ▼
Next Render tick (16ms at 60fps)
    │
    ▼
ratatui terminal.draw(|f| ui::draw(f, &app))
    │  diff_view widget reads AppState.current_hunk
    ▼
Double-buffer diff applied to terminal
```

### File Change Detected

```
Editor saves file in working tree
    │
    ▼
OS FSEvents/inotify
    │
    ▼
notify::recommended_watcher callback (sync thread)
    │  tx.blocking_send(AppEvent::FileChanged(event))
    ▼
watch task relay → main event channel
    │
    ▼
Main loop: AppEvent::FileChanged
    │  debounce check (ignore if < 100ms since last)
    │  spawn_blocking: git2 re-read diff for changed path
    ▼
AppEvent::GitResult(GitOutput::Diff(new_diff))
    │
    ▼
AppState updated, status indicator shows "modified"
    │
    ▼
Next Render tick redraws diff panel
```

### Claude Code Calls MCP Tool

```
Claude Code (MCP client) needs review context
    │
    │  spawns: airev-mcp --db ~/.local/share/airev/sessions.db
    ▼
airev-mcp reads JSON-RPC from stdin
    │  { "method": "tools/call", "params": { "name": "get_session", ... } }
    ▼
Tool handler: rusqlite query on sessions.db
    │  SQLite WAL mode allows concurrent read alongside TUI writes
    ▼
Response written to stdout as JSON-RPC result
    │
    │  process exits (or stays alive for session — depends on MCP SDK)
    ▼
Claude Code receives session context, proceeds with review
```

### User Adds a Comment

```
User presses 'c' on a hunk (enter comment mode)
    │
    ▼
AppState.mode = Mode::CommentEdit
Next render tick draws edtui widget in comment panel
    │
User types comment in edtui (vim insert mode)
    │  AppEvent::Key events handled by edtui's EditorState
    │
User presses Escape then :wq or custom binding to submit
    │
    ▼
Action::SubmitComment { hunk_id, text }
    │
    ▼
tokio-rusqlite Connection::call():
    INSERT INTO comments (session_id, hunk_id, text, created_at) VALUES (...)
    │
    ▼
AppEvent::DbResult(DbResponse::CommentSaved)
    │
    ▼
AppState updated, comment count indicator refreshed, mode returns to Normal
```

---

## Async vs Sync Tradeoffs

| Concern | Approach | Rationale |
|---------|----------|-----------|
| TUI rendering | Synchronous (inside `draw` callback) | ratatui requires synchronous rendering; the `draw()` callback must be sync. This is a hard constraint. |
| Terminal event reading | Async (`crossterm::event::EventStream`) | Prevents 250ms blocking on `event::read()` that freezes the render loop. |
| File watching | Sync callback → async channel bridge | `notify` callbacks are synchronous; `blocking_send` bridges to the async world. |
| Git operations | `spawn_blocking` (thread pool) | `git2` is a blocking C library. Never call it directly in an async context; it blocks the Tokio executor thread. |
| SQLite (TUI) | `tokio-rusqlite` async bridge | Same reason as git2 — rusqlite is synchronous. |
| SQLite (MCP server) | Synchronous `rusqlite` directly | MCP server is not a TUI; its main function can block freely. No need for async bridging. |
| MCP server transport | Separate process, stdio | Hard requirement: stdio MCP server owns stdin/stdout exclusively. |

---

## Build Order (Phase Dependencies)

These components must exist before others can be built. Follow this order:

**Phase 1 — Foundation (must be first):**
- `airev-core`: shared types (`Session`, `Comment`, `Hunk`, DB schema)
- `tui.rs`: Terminal lifecycle (`Tui` struct, raw mode, alternate screen, `Drop` cleanup)
- `event.rs`: `AppEvent` enum + event router task (tokio::mpsc + crossterm EventStream)
- Basic `AppState` struct (empty fields, no logic)
- Wire main loop: receive events, call `terminal.draw()` with placeholder UI

*Nothing else works without a rendering main loop that doesn't hang.*

**Phase 2 — Rendering skeleton (required before features):**
- `ui/layout.rs`: 3-panel Constraint layout (file list | diff | comments)
- Placeholder widgets for each panel
- Verify responsive width behavior at various terminal widths

*All feature work requires a rendered UI to see results.*

**Phase 3 — Git layer (required for any review functionality):**
- `git/diff.rs`: `git2` diff reading, hunk parsing, `spawn_blocking` dispatch
- `git/commits.rs`: commit navigation
- Wire `AppEvent::GitResult` into AppState
- Basic hunk navigation in diff panel

**Phase 4 — Persistence (required for sessions/comments):**
- DB schema migrations in `airev-core`
- `db/mod.rs`: `tokio-rusqlite` queries (sessions CRUD, comments CRUD)
- Session lifecycle: create on launch, resume on reopen

**Phase 5 — File watching:**
- `watch/mod.rs`: `notify` watcher + debouncer
- Wire `AppEvent::FileChanged` → re-read git diff

*File watching depends on the git layer being present to re-compute diffs.*

**Phase 6 — Editor widget:**
- `edtui` integration in comment panel
- Submit comment → DB write

**Phase 7 — MCP server:**
- `airev-mcp` binary: `rmcp` / `rust-mcp-sdk`, stdio transport
- Tool implementations: `get_session`, `list_comments`, `add_annotation`, `get_hunk_context`
- SQLite WAL mode configuration
- Test concurrent access: MCP server reads while TUI writes

*MCP server depends on the DB schema being stable (Phase 4 must be complete).*

---

## Anti-Patterns

### Anti-Pattern 1: Blocking in the Async Event Handler

**What people do:** Call `git2::Repository::diff_index_to_workdir()` directly inside the `match event` arm.

**Why it's wrong:** `git2` is a C library with no async support. Calling it inside a Tokio async context blocks the executor thread, freezing the TUI for the duration of the git call. On large repos this is hundreds of milliseconds.

**Do this instead:** Always use `tokio::task::spawn_blocking` for git2 calls and receive results as `AppEvent::GitResult` on the next event loop iteration.

---

### Anti-Pattern 2: Mutex-Guarding App State

**What people do:** Wrap `AppState` in `Arc<Mutex<AppState>>` to share between the render loop and background tasks.

**Why it's wrong:** Acquiring a Mutex in the background task while the render loop holds it (or vice versa) causes rendering to stall. The `spotify-tui` team explicitly documents this pitfall: acquire the lock *after* async operations complete.

**Do this instead:** App state lives only in the main loop (single ownership). Background tasks communicate results back via the event channel. The main loop applies results to app state before the next render. No mutex needed anywhere in the critical path.

---

### Anti-Pattern 3: Running the MCP Server in the TUI Process

**What people do:** Spawn the MCP stdio server as a thread inside the TUI binary to avoid a second binary.

**Why it's wrong:** The MCP spec requires the server to write *only* valid JSON-RPC messages to stdout. The TUI also writes to the terminal (stdout/stderr). Even with redirects, the model (Claude Code) expects to read the MCP server's stdout directly — this is only possible if it spawned the process itself on dedicated file descriptors.

**Do this instead:** Build `airev-mcp` as a separate binary. Claude Code spawns it; the TUI never knows it exists. They share state via SQLite.

---

### Anti-Pattern 4: Using `std::sync::mpsc` Instead of `tokio::sync::mpsc`

**What people do:** Use standard library channels for the event loop, mixing sync and async code.

**Why it's wrong:** `std::sync::mpsc::Receiver::recv()` blocks the thread. Calling it inside an async context blocks the Tokio executor. The entire point of the async event loop is non-blocking multiplexing.

**Do this instead:** Use `tokio::sync::mpsc` for all async event routing. Use `crossbeam-channel` only inside thread-pool workers (like the asyncgit pattern), and funnel their results back via `tokio::sync::mpsc`.

---

### Anti-Pattern 5: Single Monolithic `app.rs` with All Logic

**What people do:** Put git operations, DB queries, UI state, and event handling all in one `App` struct with hundreds of methods.

**Why it's wrong:** Makes testing impossible, causes massive recompiles on any change, and makes async boundary decisions unclear.

**Do this instead:** Follow the layer separation in the project structure above. `AppState` holds data. `git/`, `db/`, `watch/` modules handle their respective I/O. `ui/` is pure render. `event.rs` wires them together.

---

## Integration Points

### External Services

| Service | Integration Pattern | Notes |
|---------|---------------------|-------|
| Claude Code (MCP client) | Spawns `airev-mcp` as a subprocess via stdio | Claude Code manages the process lifecycle; `airev-mcp` must be on PATH or configured with full path |
| Git repository | `git2` crate (libgit2 bindings) via `spawn_blocking` | Never call git2 synchronously in async context |
| OS file system (FSEvents / inotify) | `notify` crate `recommended_watcher` | Uses platform-native watching: FSEvents on macOS, inotify on Linux |
| SQLite | `tokio-rusqlite` (TUI) / `rusqlite` (MCP server) | WAL mode required for concurrent access |

### Internal Boundaries

| Boundary | Communication | Notes |
|----------|---------------|-------|
| Event task → Main loop | `tokio::mpsc::UnboundedSender<AppEvent>` | Event task is the only writer; main loop is the only reader |
| Git layer → Main loop | `AppEvent::GitResult` via same event channel | `spawn_blocking` completes → sends to channel |
| DB layer → Main loop | `AppEvent::DbResult` via same event channel | `tokio-rusqlite` future resolves → caller sends to channel |
| File watcher → Main loop | `AppEvent::FileChanged` via same event channel | `blocking_send` in notify callback |
| TUI process → MCP process | SQLite WAL shared file | No direct IPC; SQLite is the shared state store |
| edtui widget ↔ AppState | `EditorState` owned in `AppState`, rendered by reference | edtui handles its own key events when mode is `CommentEdit` |

---

## Scaling Considerations

This is a single-developer local tool; traditional scaling metrics do not apply. The relevant scaling axis is **repository size**:

| Repo Scale | Architecture Adjustments |
|------------|--------------------------|
| Small repos (<10k commits, <1k files) | Default approach works; no optimization needed |
| Medium repos (10k-100k commits) | Paginate commit log; lazy-load diffs on scroll; implement virtual scrolling in list widgets |
| Large repos (>100k commits, Linux kernel scale) | Stream diff output instead of loading full diff; add LRU cache for recently viewed diffs; consider separate process for git operations with shared memory |

### Scaling Priorities

1. **First bottleneck:** Git diff computation for large files. Fix: incremental diff loading, only parse hunks in the visible viewport.
2. **Second bottleneck:** SQLite write contention if MCP server and TUI write simultaneously at high frequency. Fix: WAL mode + `busy_timeout=5000` is sufficient for a single-developer tool. If needed, add a write queue in the TUI layer.

---

## Sources

- **Ratatui async tutorial (official):** https://ratatui.rs/tutorials/counter-async-app/full-async-events/ — HIGH confidence
- **Ratatui async template structure:** https://ratatui.github.io/async-template/02-structure.html — HIGH confidence
- **Ratatui terminal + event handler recipe:** https://ratatui.rs/recipes/apps/terminal-and-event-handler/ — HIGH confidence
- **asyncgit README (gitui architecture):** https://github.com/gitui-org/gitui/blob/master/asyncgit/README.md — HIGH confidence
- **tokio-rusqlite crate:** https://crates.io/crates/tokio-rusqlite — HIGH confidence
- **notify crate + tokio bridge pattern:** https://app.studyraid.com/en/read/10838/332168/file-watching-and-events — MEDIUM confidence (verified against tokio docs)
- **MCP stdio transport constraints:** https://github.com/microsoft/mcp-for-beginners/blob/main/03-GettingStarted/05-stdio-server/README.md — HIGH confidence
- **rmcp / rust-mcp-sdk:** https://github.com/rust-mcp-stack/rust-mcp-sdk — MEDIUM confidence (crate is active but relatively new)
- **edtui widget:** https://github.com/preiter93/edtui — HIGH confidence (official GitHub, well-maintained)
- **Ratatui layout system:** https://ratatui.rs/concepts/layout/ — HIGH confidence
- **mcp-probe TUI + MCP architecture reference:** https://github.com/conikeec/mcp-probe — MEDIUM confidence (real-world example of TUI + MCP coexistence)
- **spotify-tui async lessons:** https://keliris.dev/articles/improving-spotify-tui — MEDIUM confidence (historical but widely cited as definitive Rust TUI async case study)

---

*Architecture research for: airev — Rust/ratatui TUI code review tool with MCP server*
*Researched: 2026-02-17*
