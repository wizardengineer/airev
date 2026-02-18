# Pitfalls Research

**Domain:** Rust terminal TUI — ratatui + file watching + git + SQLite + MCP server
**Researched:** 2026-02-17
**Confidence:** HIGH (critical pitfalls verified against official docs, GitHub issues, and ratatui.rs)

---

## Critical Pitfalls

These mistakes have caused rewrites or complete project abandonment in the Rust TUI ecosystem.
Prioritized by severity and likelihood for this specific project.

---

### Pitfall 1: Raw Mode Left Active on Crash

**What goes wrong:**
The terminal enters raw mode and the alternate screen when the TUI starts. If the process crashes,
panics, or receives SIGTERM without executing cleanup code, the terminal is left in raw mode and
possibly on the alternate screen. The shell becomes unusable — no line editing, no echo, characters
appear as garbage. The user must type `reset` blindly to recover.

**Why it happens:**
Developers call `enable_raw_mode()` and then rely on Rust's normal exit path for cleanup. But
panics and signals bypass the normal return path. `Drop` implementations run on normal exit and
stack unwinds, but SIGKILL and some panics do not unwind at all.

A second cause: using `Terminal::new()` directly instead of `ratatui::init()`. As of ratatui 0.28.1,
`ratatui::init()` automatically installs a panic hook that calls `ratatui::restore()`. `Terminal::new()`
does NOT do this — callers must install the hook manually.

**How to avoid:**
1. Use `ratatui::init()` (not `Terminal::new()`) — it installs the panic hook automatically.
2. Still install a SIGTERM handler using `signal-hook` or `ctrlc` crate that sets an `AtomicBool`
   and causes the event loop to exit cleanly, triggering `ratatui::restore()`.
3. For signals that cannot be caught (SIGKILL), accept that no cleanup is possible — the terminal
   handles this gracefully on process death by restoring state. Only SIGTERM needs a handler.
4. If using `Terminal::new()` for any reason, add this boilerplate before `init_tui()`:

```rust
pub fn init_panic_hook() {
    let original_hook = std::panic::take_hook();
    std::panic::set_hook(Box::new(move |panic_info| {
        let _ = ratatui::restore(); // intentionally ignore errors — already panicking
        original_hook(panic_info);
    }));
}
```

**Warning signs:**
- Shell becomes unresponsive after a test run
- Characters don't echo when typing after Ctrl+C
- `stty -a` shows `(-echo)` after the process exits

**Phase to address:** Phase 1 — Terminal Foundation. Wire this before any other feature.

---

### Pitfall 2: MCP stdio Server Corrupting the TUI Terminal

**What goes wrong:**
An MCP server using stdio transport reads from stdin and writes to stdout. Ratatui, by default,
also renders to stdout and reads keyboard events from stdin. If both run in the same process using
the same file descriptors, MCP JSON-RPC messages appear as garbage on the terminal, and keyboard
events get consumed by the MCP reader instead of the TUI. The result is a completely unusable
application.

**Why it happens:**
The MCP stdio specification requires the server to own stdin/stdout entirely for JSON-RPC framing.
Ratatui's `DefaultTerminal` also owns stdin/stdout. These two ownership models are fundamentally
incompatible in the same process on the same fds.

**How to avoid:**
Choose exactly one of the following architectures — do not try to combine them without choosing:

**Option A (Recommended for airev): Render TUI to stderr, MCP owns stdout/stdin.**
```rust
// MCP server reads stdin / writes stdout as normal
// TUI renders to stderr:
let backend = CrosstermBackend::new(std::io::stderr());
let terminal = Terminal::new(backend)?;
```
Downside: stderr is unbuffered, so rendering is ~2x slower (~12 FPS vs ~24 FPS). For a code
review tool this is acceptable — wrap stderr in a `BufWriter`.

**Option B: Run MCP server as a subprocess, communicate over pipes.**
The TUI process owns stdin/stdout normally. The MCP server is a separate process whose stdin/stdout
are piped via channels. The TUI parent forwards MCP traffic. More complex but no performance hit.

**Option C: Use a non-stdio MCP transport (HTTP/SSE).**
Host the MCP server on a local port. Claude Code connects via HTTP. TUI owns the terminal. No
conflict. Adds network complexity but is the cleanest separation.

**Warning signs:**
- Terminal shows raw JSON on screen during testing
- Key events cause the MCP server to emit errors about malformed JSON
- Application works when MCP is disabled but breaks when enabled

**Phase to address:** Phase 1 — Architecture design decision. This must be resolved before
implementing either the TUI or the MCP server, as it determines the entire I/O architecture.

---

### Pitfall 3: Blocking Git Operations on the UI Thread

**What goes wrong:**
Calling `git2::Repository::diff_*` or spawning `git diff` on the main thread freezes the TUI for
the duration of the operation. On a large repository like LLVM (hundreds of thousands of files),
a cold diff can take 5–30 seconds. The terminal becomes completely unresponsive — no cursor
movement, no key handling, no rendering. Users assume the application crashed.

**Why it happens:**
libgit2 (and thus the `git2` Rust crate) performs diff computations synchronously. A known GitHub
issue (libgit2/libgit2 #3920) documents that libgit2 diff is nearly 10x slower than native `git`
for large repositories. Developers write `repo.diff_tree_to_workdir()` directly in the render loop
or event handler without recognizing it as a blocking call.

An additional complication: `git2::Repository` does not implement `Send` in the current crate
version (see git2-rs issue #194). You cannot move a `Repository` into `tokio::task::spawn_blocking`
directly — the compiler will reject it. The workaround is to open a new `Repository` inside the
background task closure, or to dedicate a single `std::thread::spawn` thread to all git operations
and communicate with it via channels.

**How to avoid:**
1. All git operations go on a dedicated background thread (`std::thread::spawn`), not the Tokio
   runtime thread pool. This thread owns the `Repository` and receives requests via `mpsc::channel`.
2. Return computed diffs as owned `Vec<DiffHunk>` structs (not `git2::Diff` which borrows the
   `Repository`) so results can be sent safely across thread boundaries.
3. Show a loading indicator in the TUI while git is computing. The event loop continues running.
4. Implement pagination: load only the first N files of a diff immediately; load remaining files
   on demand as the user scrolls.
5. Consider caching: store the last known diff in the SQLite database so the UI can display stale
   data instantly while a fresh computation runs in background.

**Warning signs:**
- Application freezes when switching to a commit with many changed files
- The TUI frame rate drops to 0 during git operations
- `tokio::task::spawn_blocking` compiler error mentioning `git2::Repository: not Send`

**Phase to address:** Phase 2 — Git Integration. Design the background-thread architecture before
writing any git code.

---

### Pitfall 4: SQLite Deadlocks Between TUI and MCP Server

**What goes wrong:**
Both the TUI process and the MCP server process (or threads) open connections to the same
`.airev/reviews.db` file. Without WAL mode, SQLite uses exclusive write locks — any write from one
process blocks all readers and writers in the other. With WAL mode enabled but `busy_timeout` not
set, a write from the MCP server while the TUI is reading returns `SQLITE_BUSY` immediately with no
retry, causing the TUI to silently lose review state or crash.

A subtler version: a `DEFERRED` transaction that starts with a read and later upgrades to a write
will return `SQLITE_BUSY` immediately (not after the timeout) when another writer has committed
between the read and the upgrade. The `busy_timeout` pragma has NO effect on this upgrade failure.

**Why it happens:**
Developers enable WAL mode (which is correct) but assume it solves all concurrency. SQLite's WAL
mode allows one writer and multiple readers simultaneously, but write serialization still applies.
The `DEFERRED` transaction upgrade bug is non-obvious and the SQLite documentation buries this
behavior.

**How to avoid:**
1. Enable WAL mode on every connection, immediately after opening:
   ```rust
   conn.pragma_update(None, "journal_mode", "WAL")?;
   conn.pragma_update(None, "synchronous", "NORMAL")?;
   conn.busy_timeout(Duration::from_secs(10))?;
   ```
2. For any transaction that will write, start it with `BEGIN IMMEDIATE` (not `BEGIN` or
   `BEGIN DEFERRED`). In rusqlite: `conn.execute("BEGIN IMMEDIATE", [])?;`
3. Keep write transactions small and short — one logical operation per transaction.
4. If using separate TUI and MCP server processes, consider routing all writes through a single
   owner (the TUI) and using MCP tool calls to request writes rather than writing directly.
5. Set `PRAGMA wal_checkpoint(TRUNCATE)` on startup to prevent unbounded WAL file growth.

**Warning signs:**
- `rusqlite::Error::SqliteFailure` with error code 5 (SQLITE_BUSY) in logs
- Review state disappears after MCP tool calls
- WAL file (`.airev/reviews.db-wal`) grows without bound
- Intermittent "database is locked" errors that are hard to reproduce

**Phase to address:** Phase 1 (database schema) and Phase 3 (MCP integration). Set up WAL + BEGIN
IMMEDIATE in the initial schema migration code, before any concurrent access is introduced.

---

### Pitfall 5: notify File Watcher Missing Claude Code's Write Pattern

**What goes wrong:**
Claude Code writes files by creating a temporary file and then atomically renaming it to the
target path (the same pattern used by most editors). The `notify` crate's raw watcher on Linux
emits a `Create` event (from the rename) rather than a `Modify` event. A file watcher that listens
only for `EventKind::Modify` will miss all of Claude Code's writes on Linux, while correctly
detecting them on macOS (where FSEvents aggregates rename-based writes as `Modify`).

Additionally, Claude Code typically writes multiple files in rapid succession (modifying a function,
its test file, and related modules in one operation). Without debouncing, each file write triggers a
separate git diff computation, causing the UI to thrash.

**Why it happens:**
FSEvents (macOS) and inotify (Linux) have fundamentally different event models. FSEvents coalesces
events at the OS level; inotify is fine-grained and precise. The `notify` crate exposes these
differences rather than fully abstracting them. Developers test on macOS and ship to Linux without
realizing the event types differ.

The default debounce timeout in `notify-debouncer-full` is configurable but not set by default.
Without it, 10 rapid writes produce 10 separate callbacks.

**How to avoid:**
1. Use `notify-debouncer-full` (not the raw `notify` crate) with a 300–500ms debounce window.
2. Listen for ALL event kinds (`EventKind::Any`) rather than filtering to `Modify`. After debounce,
   determine if a file actually changed by comparing git status, not by trusting event types.
3. Test the watcher explicitly on both macOS and Linux in CI. Use Docker for Linux testing on a Mac.
4. For network filesystems or WSL paths, fall back to `PollWatcher` automatically:
   ```rust
   let watcher = match RecommendedWatcher::new(tx, config) {
       Ok(w) => w,
       Err(_) => PollWatcher::new(tx, config)?, // fallback
   };
   ```
5. After debounce fires, run `git status --porcelain` (or equivalent via git2) to get the
   canonical list of changed files rather than trusting the watcher's file list.

**Warning signs:**
- File changes detected on macOS but missed on Linux in the same test
- Rapid writes trigger many git diff computations (visible as CPU spikes)
- `notify` errors with "no space left on device" on Linux (inotify watch limit hit)
- File watcher stops working after editing files in Docker on macOS M1

**Phase to address:** Phase 2 — File Watching. Test on both platforms before declaring done.

---

## Moderate Pitfalls

---

### Pitfall 6: Calling terminal.draw() More Than Once Per Loop Iteration

**What goes wrong:**
Ratatui uses a double buffer: each call to `terminal.draw()` wipes the working buffer clean before
rendering widgets into it. If you call `terminal.draw()` twice in one loop iteration (e.g., once for
the main view and once for a modal overlay), only the second call's content appears on screen. The
first draw is completely erased. This manifests as widgets flickering in and out or never appearing.

**Why it happens:**
Developers coming from immediate-mode GUI frameworks (Dear ImGui, egui) are accustomed to calling
draw functions in sequence and seeing cumulative results. Ratatui's buffer model is different: the
entire frame must be composed in a single `draw()` closure.

**How to avoid:**
All widgets — including overlays, modals, and popups — must be rendered inside a single
`terminal.draw(|frame| { ... })` call. Use layout constraints and `Clear` widget to handle overlays:
```rust
terminal.draw(|frame| {
    render_main_view(frame, area);
    if state.show_modal {
        render_modal(frame, centered_area); // rendered on top, in same closure
    }
})?;
```

**Warning signs:**
- A widget renders on the first frame then disappears
- Modal dialogs don't appear
- Calling `draw()` seems to clear existing content

**Phase to address:** Phase 1 — TUI skeleton. Learn the rendering model before building any widgets.

---

### Pitfall 7: Vim Keybinding State Machine Complexity

**What goes wrong:**
Rolling a custom Vim-style modal editor (normal/insert/visual modes) from scratch balloons in
complexity. Handling `ci"`, `dap`, `gg`, `10j`, counts, marks, and registers requires thousands of
lines of state machine code. The first version seems to work fine, but edge cases (operator-pending
mode, numeric prefixes, text objects) accumulate until the code is unmaintainable. Meanwhile, the
`edtui` and `modalkit-ratatui` crates already implement this.

**Why it happens:**
The initial implementation of `i`, `j`, `k`, `l`, and `Esc` takes about 50 lines and feels easy.
Developers don't realize the full scope of Vim's grammar until they're 2000 lines deep.

**How to avoid:**
Do not write a modal editor from scratch. Use existing crates:
- `edtui` — Vim-inspired editor widget for ratatui, handles the state machine internally. Best
  for inline text editing within the TUI (review comments, search boxes).
- `modalkit-ratatui` — Full Vim keybinding machine with `default_vim_keys()`. Most complete.
- `ratatui-textarea` — Simpler, includes a Vim emulation example using an enum-based state machine.

For the diff viewer (read-only navigation), normal-mode navigation keys (`j/k/d/u/g/G`) are a
small, bounded set. Build only this subset custom; do not generalize.

**Warning signs:**
- The keybinding handler file exceeds 300 lines
- `v` (visual mode) is deferred "for later" and never gets done
- Users report that `ci"` doesn't work as expected

**Phase to address:** Phase 2 — UI interactions. Decide on edtui vs. custom before writing any
keybinding code.

---

### Pitfall 8: Large Diff Rendering Without Virtualization

**What goes wrong:**
Passing thousands of diff lines directly to a `ratatui::widgets::List` or `Paragraph` widget causes
noticeable frame drops. Ratatui's `List` widget does not virtualize — it processes all items to
compute layout, even items that are off-screen. For a diff with 10,000 lines (common in LLVM-scale
changes), this produces hundreds of milliseconds of work per frame.

**Why it happens:**
Ratatui's immediate mode model encourages passing the full dataset to widgets. For small datasets
this is fine. The performance cliff is invisible during development with small test diffs and only
appears in production with real LLVM-sized commits.

**How to avoid:**
Implement manual virtualization using `ListState` scroll offset tracking:
1. Keep the full diff in memory as a `Vec<Line>`.
2. Compute `visible_start = state.scroll_offset` and `visible_end = visible_start + terminal_height`.
3. Pass only `&lines[visible_start..visible_end]` to the widget each frame.
4. For syntax highlighting, pre-compute `Vec<Spans>` in the background thread, not in the render
   closure.

For LLVM-scale diffs (100K+ lines), also implement lazy loading: parse only the portion of the diff
that has been scrolled to, loading more on demand.

**Warning signs:**
- Frame rate drops when opening a large diff
- The application feels sluggish only on real project diffs (not toy examples)
- Profiling shows the render closure taking >16ms per frame

**Phase to address:** Phase 3 — Diff viewer. Build with virtualization from the start; retrofitting
it is painful.

---

### Pitfall 9: Terminal Resize Events Not Triggering Relayout

**What goes wrong:**
When the user resizes their terminal window, crossterm emits `Event::Resize(width, height)`. If the
event loop does not explicitly handle this event and trigger a redraw, the TUI continues rendering
at the old dimensions. Content outside the new window bounds is clipped or padded incorrectly.
The layout appears broken until the next key press that happens to trigger a redraw.

Additionally, on Windows, ALL keyboard events are sent twice (once for `Press`, once for `Release`).
Not filtering for `KeyEventKind::Press` causes every action to execute twice — double navigation,
double commits, double saves.

**How to avoid:**
```rust
match event {
    Event::Resize(_, _) => { /* force full redraw next iteration */ }
    Event::Key(key) if key.kind == KeyEventKind::Press => { handle_key(key) }
    _ => {}
}
```
Always filter `KeyEventKind::Press`. Always handle `Event::Resize`.

**Warning signs:**
- Layout breaks after resizing the terminal window
- On Windows, every key action fires twice
- Scrolling jumps two positions per keypress on Windows

**Phase to address:** Phase 1 — Event loop setup.

---

### Pitfall 10: SSH Sessions — Clipboard and Color Support

**What goes wrong:**
When airev runs over SSH, the system clipboard (`arboard`, `clipboard-rs`) is unavailable — there
is no X11 or Wayland session to connect to. Calling clipboard APIs silently fails or panics.
Additionally, some SSH terminal emulators don't support 256 colors or true color — rendering
carefully chosen review diff colors as undifferentiated noise.

**Why it happens:**
Developers test locally with a desktop terminal and never try the tool over SSH. The clipboard API
returns `Ok(())` in some implementations even when it silently fails, making the failure invisible.
Color fallback requires explicit capability detection that most projects skip.

**How to avoid:**
1. Use OSC 52 escape sequences for clipboard access over SSH. OSC 52 is supported by iTerm2,
   most modern terminal emulators, and native SSH. It is NOT supported by macOS Terminal.app or
   Mosh. Check `$SSH_TTY` to detect SSH sessions.
2. Gate all clipboard operations behind a result check and surface a status bar message on failure
   rather than silently failing.
3. Use `$COLORTERM` environment variable (`truecolor` or `24bit`) to detect true color support.
   Fall back to 256-color ANSI for unsupported terminals.
4. Test explicitly with `ssh localhost` and a plain `xterm` to verify graceful degradation.

**Warning signs:**
- "Copy to clipboard" does nothing over SSH
- Colors look wrong in a non-iTerm2 terminal
- `$SSH_TTY` is set but clipboard still tries to use the system clipboard

**Phase to address:** Phase 4 — Polish and compatibility.

---

## Technical Debt Patterns

Shortcuts that seem reasonable in early phases but create compounding problems.

| Shortcut | Immediate Benefit | Long-term Cost | When Acceptable |
|----------|-------------------|----------------|-----------------|
| Open `git2::Repository` on every git call | Simple, no thread management | CPU spike per call; `not Send` forces awkward workarounds | Never — open once per background thread |
| Default SQLite journal mode (DELETE) | Zero setup | `SQLITE_BUSY` errors as soon as MCP server runs | Never — WAL mode on from day one |
| `terminal.draw()` without a panic hook | Fewer lines of code | Terminal left in raw mode on any crash | Never — ratatui::init() is 1 line |
| Pass full diff `Vec<Line>` to `List` widget | Works fine for small diffs | Frame drops on real projects | MVP only — replace before beta |
| Roll custom Vim keybindings | Full control | 2000+ lines of unmaintainable state machine | Never — use edtui |
| Synchronous git diff in event handler | Simple flow | Application freezes on large diffs | Never — background thread from day one |
| Render TUI to stdout when MCP is enabled | Simpler setup | JSON-RPC output corrupts the terminal | Never — pick stderr or separate process |

---

## Integration Gotchas

| Integration | Common Mistake | Correct Approach |
|-------------|----------------|------------------|
| MCP stdio transport | Both TUI and MCP read/write stdin/stdout | Render TUI to stderr; MCP owns stdout/stdin |
| `git2` crate | Moving `Repository` into `spawn_blocking` | Dedicated `std::thread::spawn` thread owns the `Repository` |
| `notify` file watcher | Listening only for `Modify` events | Listen for any event; verify changes via `git status` |
| rusqlite WAL mode | Enable WAL but use `BEGIN DEFERRED` for writes | Always `BEGIN IMMEDIATE` for write transactions |
| crossterm on Windows | Not filtering `KeyEventKind` | Always check `key.kind == KeyEventKind::Press` |
| ratatui::init() | Using `Terminal::new()` without installing panic hook | Use `ratatui::init()` which installs the hook automatically |
| SIGTERM handling | Relying on `Drop` for terminal cleanup | Register `signal-hook` handler that sets an exit flag |

---

## Performance Traps

| Trap | Symptoms | Prevention | When It Breaks |
|------|----------|------------|----------------|
| Unvirtualized diff rendering | Frame drops on large diffs | Pass only visible slice to `List` widget | Diffs with 500+ lines |
| Git diff on UI thread | UI freezes during git operations | Background thread with channel messaging | Any repository, any size |
| Multiple `terminal.draw()` per loop | Widgets disappear or flicker | Single `draw()` call per frame | Immediately on first use |
| Synchronous `notify` callback | Event handler blocks the watcher thread | Debounce + send to event channel; don't compute in callback | First rapid write sequence |
| Unbounded WAL file | Disk fills up; checkpoint takes seconds | `PRAGMA wal_checkpoint(TRUNCATE)` on startup | After ~1000 write transactions |
| Syntax highlighting in render closure | Render closure exceeds 16ms | Pre-compute spans in background thread | First file with >200 lines |

---

## Security Mistakes

Domain-specific security concerns for a code review tool that executes git and spawns processes.

| Mistake | Risk | Prevention |
|---------|------|------------|
| Passing unsanitized file paths from watcher events to shell commands | Path traversal / command injection | Use `git2` API (not `git` subprocess) for operations where possible; validate paths against repo root |
| MCP tool arguments passed directly to git subprocess | Command injection | Validate all MCP tool arguments as structured types, never interpolate into shell strings |
| Storing full diff content in SQLite without size limit | Disk exhaustion | Cap stored diff size; store only metadata + reference to git object hash |
| Logging review content to files readable by other users | Data leak | Ensure `.airev/` is created with `0700` permissions; log only metadata |

---

## UX Pitfalls

| Pitfall | User Impact | Better Approach |
|---------|-------------|-----------------|
| No loading indicator during git diff computation | User thinks app is frozen | Show spinner/status bar "Computing diff..." while background thread works |
| Silent clipboard failure over SSH | User loses copied content with no feedback | Show "Copied to clipboard" / "Clipboard unavailable (SSH)" in status bar |
| Terminal left in raw mode after crash | Shell becomes unusable | ratatui::init() panic hook — non-negotiable |
| No visual indication of modal state | User types text into wrong mode | Always render mode indicator (NORMAL / INSERT) in status bar |
| Resize not triggering relayout | Broken UI after window resize | Handle `Event::Resize` explicitly in event loop |
| Rendering ALL diff lines on first open | 1-2 second blank screen on LLVM-scale diff | Virtualize list, show first screenful immediately |

---

## "Looks Done But Isn't" Checklist

- [ ] **Terminal cleanup:** Verified that crash (panic), Ctrl+C (SIGINT), and SIGTERM all restore
      the terminal correctly — not just normal exit.
- [ ] **MCP + TUI coexistence:** Verified that MCP JSON-RPC messages do not appear as garbage on
      the terminal when both are active simultaneously.
- [ ] **WAL mode:** Verified with `PRAGMA journal_mode;` that the database is actually in WAL mode
      (some pragmas silently fail if the database is locked).
- [ ] **File watcher cross-platform:** Verified that file changes are detected on both macOS
      (FSEvents) and Linux (inotify), not just the development platform.
- [ ] **Large diff performance:** Tested with a real LLVM-scale diff (100+ files, 5000+ lines).
      Not just with toy examples.
- [ ] **Resize handling:** Resized the terminal window during a running session and verified the
      layout recomputed correctly.
- [ ] **SSH clipboard:** Ran the tool over `ssh localhost` and verified clipboard failure is
      surfaced gracefully rather than silently.
- [ ] **Windows key duplication:** If Windows support is planned, verified single-fire per keypress.
- [ ] **git2 thread safety:** Verified there are no `git2::Repository` values crossing thread
      boundaries (compiler check + runtime test under heavy load).

---

## Recovery Strategies

| Pitfall | Recovery Cost | Recovery Steps |
|---------|---------------|----------------|
| Terminal left in raw mode | LOW | User types `reset` — recoverable, but terrible UX |
| MCP + TUI stdout conflict discovered late | HIGH | Architectural refactor of entire I/O layer; affects both MCP and TUI code |
| git diff blocking discovered late | MEDIUM | Refactor event handler to use channels; moderate scope |
| SQLite WAL not enabled, concurrent corruption | HIGH | Data may be corrupted; must migrate database, add WAL, audit all write paths |
| No virtualization, performance unusable | MEDIUM | Refactor list rendering; data pipeline stays the same |
| Custom Vim keybindings, unmaintainable | HIGH | Near-full rewrite of input handling; switch to edtui |

---

## Pitfall-to-Phase Mapping

| Pitfall | Prevention Phase | Verification |
|---------|------------------|--------------|
| Raw mode not restored on crash/signal | Phase 1 — TUI Foundation | Intentionally panic the app; verify terminal recovers |
| MCP + TUI stdio conflict | Phase 1 — Architecture Decision | Enable both and verify no JSON on screen |
| Git diff blocking UI thread | Phase 2 — Git Integration | Open a 500-file diff; verify UI remains responsive |
| SQLite concurrency / SQLITE_BUSY | Phase 1 — Database Setup | Run concurrent TUI + MCP writes; check for errors |
| File watcher missing rename-based writes | Phase 2 — File Watching | Save a file with vim/Claude Code; verify detection on Linux |
| Multiple draw() calls per frame | Phase 1 — TUI Skeleton | Code review; confirm single draw() per loop |
| Custom Vim state machine complexity | Phase 2 — UI Interactions | Adopt edtui before writing any keybinding code |
| Unvirtualized large diff rendering | Phase 3 — Diff Viewer | Test with real LLVM diff before calling phase complete |
| Resize not handled | Phase 1 — Event Loop | Resize during smoke test |
| SSH clipboard failure | Phase 4 — Compatibility | Smoke test over `ssh localhost` |

---

## Sources

- [Setup Panic Hooks — Ratatui](https://ratatui.rs/recipes/apps/panic-hooks/) — HIGH confidence
- [Ratatui FAQ](https://ratatui.rs/faq/) — HIGH confidence (official docs)
- [Ratatui rendering under the hood](https://ratatui.rs/concepts/rendering/under-the-hood/) — HIGH confidence
- [Ratatui Async Event Stream tutorial](https://ratatui.rs/tutorials/counter-async-app/async-event-stream/) — HIGH confidence
- [Ratatui Backends — stdout vs stderr](https://ratatui.rs/concepts/backends/) — HIGH confidence
- [ratatui issue #1005 — Termion panic hook doesn't exit raw mode correctly](https://github.com/ratatui/ratatui/issues/1005) — HIGH confidence
- [ratatui discussion #579 — Rendering best practices](https://github.com/ratatui/ratatui/discussions/579) — MEDIUM confidence
- [libgit2 issue #3920 — Diff via libgit2 nearly 10x slower than git](https://github.com/libgit2/libgit2/issues/3920) — HIGH confidence (official issue)
- [git2-rs issue #194 — Implement Send for libgit2 structs](https://github.com/rust-lang/git2-rs/issues/194) — HIGH confidence
- [notify-debouncer-full docs](https://docs.rs/notify-debouncer-full) — HIGH confidence (official docs)
- [notify crate — Rust](https://docs.rs/notify/) — HIGH confidence
- [parcel/watcher issue #171 — FSEvents behaves differently from inotify](https://github.com/parcel-bundler/watcher/issues/171) — MEDIUM confidence
- [rusqlite Connection docs](https://docs.rs/rusqlite/latest/rusqlite/struct.Connection.html) — HIGH confidence
- [SQLite concurrent writes and "database is locked" errors](https://tenthousandmeters.com/blog/sqlite-concurrent-writes-and-database-is-locked-errors/) — MEDIUM confidence (verified against SQLite official pragma docs)
- [SQLite PRAGMA documentation](https://sqlite.org/pragma.html) — HIGH confidence (official SQLite docs)
- [edtui — vim-inspired editor widget for ratatui](https://github.com/preiter93/edtui) — HIGH confidence
- [modalkit-ratatui docs](https://docs.rs/modalkit-ratatui/latest/modalkit_ratatui/) — HIGH confidence
- [ratatui-textarea — multi-line text editor widget](https://github.com/rhysd/tui-textarea) — HIGH confidence
- [crossterm raw mode docs](https://docs.rs/crossterm/latest/crossterm/terminal/fn.enable_raw_mode.html) — HIGH confidence
- [Why stdout is faster than stderr — Orhun's Blog](https://blog.orhun.dev/stdout-vs-stderr/) — MEDIUM confidence (benchmark article)
- [mcp-probe — MCP TUI client in Rust](https://github.com/conikeec/mcp-probe) — MEDIUM confidence
- [MCP transports — official specification](https://modelcontextprotocol.io/specification/2025-11-25/basic/transports) — HIGH confidence (official MCP spec)

---

*Pitfalls research for: airev — Rust TUI code review tool (ratatui + git + SQLite + MCP)*
*Researched: 2026-02-17*
