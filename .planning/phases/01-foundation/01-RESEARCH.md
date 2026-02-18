# Phase 1: Foundation - Research

**Researched:** 2026-02-18
**Domain:** Rust TUI (ratatui/crossterm/tokio), SQLite WAL (rusqlite/tokio-rusqlite), Cargo workspace, signal handling
**Confidence:** HIGH (core APIs verified via official docs and docs.rs)

---

## Summary

Phase 1 establishes the structural skeleton for the entire application: a Cargo workspace with two binaries (`airev`, `airev-mcp`) and a shared types crate (`airev-core`), a terminal lifecycle that survives panics and SIGTERM signals, a tokio async event loop with separated tick and render intervals, and a SQLite database opened in WAL mode with the correct pragma sequence.

The ratatui 0.30 `init()` function returns a `DefaultTerminal`, installs a panic hook automatically, and writes to **stdout by default**. Because the project requires the TUI to render to stderr (freeing stdout for MCP's stdio protocol), `ratatui::init()` cannot be used directly; instead `Terminal::new(CrosstermBackend::new(BufWriter::new(std::io::stderr())))` must be used, with a manual panic hook and explicit `ratatui::restore()` call. The signal-hook crate (version 0.3.18) provides `signal_hook::flag::register()` for registering SIGTERM against an `Arc<AtomicBool>`, which the tokio event loop polls on every iteration.

The tokio-rusqlite pattern wraps rusqlite behind an async `call()` closure, with all WAL pragmas executed via `execute_batch()` immediately after `Connection::open()`. The `BEGIN IMMEDIATE` requirement for all write transactions must be enforced explicitly, as rusqlite's default `BEGIN` defers the write lock.

**Primary recommendation:** Use `Terminal::new(CrosstermBackend::new(BufWriter::new(std::io::stderr())))` with a manual panic hook + explicit `ratatui::restore()`, NOT `ratatui::init()`, because the project requires stderr rendering.

---

## Standard Stack

### Core
| Library | Version | Purpose | Why Standard |
|---------|---------|---------|--------------|
| ratatui | 0.30 | TUI rendering, terminal management | Official TUI framework, handles diff rendering, `DefaultTerminal`, panic hooks |
| crossterm | 0.29 | Cross-platform terminal I/O, `EventStream` | ratatui's default backend; `event-stream` feature required for async |
| tokio | 1.49 (full) | Async runtime, `mpsc`, `select!`, `time::interval` | Required for concurrent event handling, background DB threads |
| rusqlite | 0.38 (bundled) | SQLite bindings | Ergonomic SQLite API, `bundled` embeds SQLite 3.51.1 (no system dep) |
| tokio-rusqlite | 0.7 | Async wrapper around rusqlite | rusqlite::Connection is `!Send`; tokio-rusqlite pins it to a background thread |
| signal-hook | 0.3.18 | SIGTERM via `AtomicBool` | Safe multi-handler signal registration; `flag::register` is the canonical pattern |

### Supporting
| Library | Version | Purpose | When to Use |
|---------|---------|---------|-------------|
| futures | 0.3 | `FutureExt::fuse()` for `EventStream` | Required by crossterm `EventStream::next().fuse()` in `tokio::select!` |

### Alternatives Considered
| Instead of | Could Use | Tradeoff |
|------------|-----------|----------|
| `signal-hook` flag | `tokio::signal::unix::SignalKind::terminate()` | tokio native is cleaner for pure-tokio apps, but the spec locks in `signal-hook` |
| `tokio-rusqlite` | `sqlx` (SQLite) | sqlx compiles SQL at build time (nicer), but tokio-rusqlite matches the spec exactly |
| `ratatui::init()` (stdout) | manual `Terminal::new()` (stderr) | `init()` is simpler but hardcoded to stdout; stderr is required for MCP compatibility |

**Installation:**
```bash
# Root workspace — no direct deps
# airev/Cargo.toml
[dependencies]
ratatui = { version = "0.30", features = ["crossterm"] }
crossterm = { version = "0.29", features = ["event-stream"] }
tokio = { version = "1.49", features = ["full"] }
signal-hook = "0.3"
futures = "0.3"
airev-core = { path = "../airev-core" }

# airev-core/Cargo.toml
[dependencies]
rusqlite = { version = "0.38", features = ["bundled"] }
tokio-rusqlite = "0.7"
tokio = { version = "1.49", features = ["full"] }
```

---

## Architecture Patterns

### Recommended Project Structure
```
diff-grief/
├── Cargo.toml                   # workspace root
├── Cargo.lock
├── airev/
│   ├── Cargo.toml
│   └── src/
│       ├── main.rs              # tokio::main, event loop
│       ├── tui.rs               # Tui struct, Drop impl, panic hook
│       ├── event.rs             # AppEvent enum, EventHandler
│       └── ui.rs                # placeholder 3-panel render
├── airev-mcp/
│   ├── Cargo.toml
│   └── src/
│       └── main.rs              # MCP stdio server (rmcp)
└── airev-core/
    ├── Cargo.toml
    └── src/
        ├── lib.rs               # pub re-exports
        ├── db.rs                # DB init, WAL pragma sequence
        └── schema.rs            # sessions, comments, threads tables
```

### Pattern 1: Workspace Root `Cargo.toml`
**What:** Declares workspace members; shared dependency versions via `[workspace.dependencies]`
**When to use:** Always in a multi-crate project

```toml
# Source: https://doc.rust-lang.org/book/ch14-03-cargo-workspaces.html
[workspace]
resolver = "3"
members = ["airev", "airev-mcp", "airev-core"]

[workspace.dependencies]
tokio = { version = "1.49", features = ["full"] }
rusqlite = { version = "0.38", features = ["bundled"] }
tokio-rusqlite = "0.7"
airev-core = { path = "airev-core" }
```

---

### Pattern 2: `AppEvent` Enum and Unbounded MPSC Channel
**What:** Single event bus; all events (key, tick, render, signals, async results) flow through one `tokio::sync::mpsc::UnboundedSender<AppEvent>`
**When to use:** This is the canonical ratatui async pattern; avoids multiple receivers

```rust
// Source: https://ratatui.rs/tutorials/counter-async-app/full-async-events/
use crossterm::event::{KeyEvent, MouseEvent};
use tokio::sync::mpsc;

#[derive(Debug, Clone)]
pub enum AppEvent {
    Key(KeyEvent),
    Resize(u16, u16),
    Tick,
    Render,
    FileChanged,
    GitResult,
    DbResult,
    Quit,
}

pub struct EventHandler {
    pub tx: mpsc::UnboundedSender<AppEvent>,
    rx: mpsc::UnboundedReceiver<AppEvent>,
}
```

---

### Pattern 3: `tokio::select!` with Separate Tick and Render Intervals
**What:** Single spawned task drives the event bus; `tokio::time::interval` for tick and render run independently; `EventStream` for crossterm input
**When to use:** Required per spec; separating tick and render rates is the official ratatui pattern

```rust
// Source: https://ratatui.rs/tutorials/counter-async-app/full-async-events/
use crossterm::event::{Event, EventStream, KeyEventKind};
use futures::{FutureExt, StreamExt};
use std::time::Duration;
use tokio::time::interval;

pub fn spawn_event_task(tx: mpsc::UnboundedSender<AppEvent>) {
    tokio::spawn(async move {
        let tick_delay = Duration::from_millis(250);   // 4 Hz logic
        let render_delay = Duration::from_millis(33);  // ~30 FPS
        let mut tick_interval = interval(tick_delay);
        let mut render_interval = interval(render_delay);
        let mut reader = EventStream::new();

        loop {
            let tick_tick = tick_interval.tick();
            let render_tick = render_interval.tick();
            let crossterm_event = reader.next().fuse();

            tokio::select! {
                _ = tick_tick => { let _ = tx.send(AppEvent::Tick); }
                _ = render_tick => { let _ = tx.send(AppEvent::Render); }
                maybe_event = crossterm_event => {
                    match maybe_event {
                        Some(Ok(Event::Key(key))) => {
                            // Filter: macOS/Linux only emit Press; Windows emits Press+Release
                            if key.kind == KeyEventKind::Press {
                                let _ = tx.send(AppEvent::Key(key));
                            }
                        }
                        Some(Ok(Event::Resize(w, h))) => {
                            let _ = tx.send(AppEvent::Resize(w, h));
                        }
                        _ => {}
                    }
                }
            }
        }
    });
}
```

---

### Pattern 4: `Tui` Struct — stderr Backend, Panic Hook, SIGTERM, Drop
**What:** Wraps `Terminal<CrosstermBackend<BufWriter<Stderr>>>` with lifecycle management
**When to use:** Required pattern; stderr backend necessary for MCP stdout compatibility

```rust
// Source: https://ratatui.rs/recipes/apps/terminal-and-event-handler/
// Source: https://ratatui.rs/recipes/apps/panic-hooks/
use std::io::{BufWriter, Stderr, stderr};
use std::panic;
use std::sync::{Arc, atomic::{AtomicBool, Ordering}};
use ratatui::{Terminal, backend::CrosstermBackend};
use crossterm::terminal::{
    disable_raw_mode, enable_raw_mode,
    EnterAlternateScreen, LeaveAlternateScreen,
};
use crossterm::execute;

pub type Tui = Terminal<CrosstermBackend<BufWriter<Stderr>>>;

pub fn init_tui() -> std::io::Result<Tui> {
    let mut stderr = BufWriter::new(stderr());
    enable_raw_mode()?;
    execute!(stderr, EnterAlternateScreen)?;
    Terminal::new(CrosstermBackend::new(stderr))
}

pub fn restore_tui() -> std::io::Result<()> {
    disable_raw_mode()?;
    execute!(stderr(), LeaveAlternateScreen)?;
    Ok(())
}

/// MUST be called after any other panic hook installs (e.g., color-eyre).
/// ratatui::init() installs its own hook, but we cannot use ratatui::init()
/// because it hardcodes stdout. We replicate the pattern manually.
pub fn install_panic_hook() {
    let original_hook = panic::take_hook();
    panic::set_hook(Box::new(move |panic_info| {
        let _ = restore_tui();   // restore terminal before printing panic
        original_hook(panic_info);
    }));
}
```

**Key note:** `ratatui::init()` installs a panic hook that calls `ratatui::restore()`, which undoes `ratatui::init()`'s own setup (stdout). For stderr rendering, replicate this pattern manually as shown above. The ordering rule still applies: install other hooks (e.g., `color_eyre::install()`) BEFORE calling `install_panic_hook()`, so terminal restoration fires before formatted error output.

---

### Pattern 5: SIGTERM via `signal_hook::flag::register`
**What:** Registers SIGTERM to set an `Arc<AtomicBool>`; the event loop polls the flag and injects `AppEvent::Quit`
**When to use:** Required by spec; simpler than `signal-hook-tokio` for a single flag

```rust
// Source: https://docs.rs/signal-hook/latest/signal_hook/flag/index.html
use std::sync::{Arc, atomic::{AtomicBool, Ordering}};
use signal_hook::{consts::SIGTERM, flag::register};

pub fn register_sigterm() -> Arc<AtomicBool> {
    let term = Arc::new(AtomicBool::new(false));
    // Safety: register() is safe as long as the handler only sets an AtomicBool
    register(SIGTERM, Arc::clone(&term)).expect("failed to register SIGTERM handler");
    term
}

// In the event loop:
// if term_flag.load(Ordering::Relaxed) { tx.send(AppEvent::Quit).ok(); break; }
```

Check the `AtomicBool` on every `tokio::select!` iteration. Because `signal_hook::flag::register` works by setting the flag from the signal handler (not a tokio task), it does not need a separate async adapter; polling in the select loop is sufficient.

---

### Pattern 6: SQLite WAL Initialization via `tokio-rusqlite`
**What:** Opens connection in tokio-rusqlite, runs the pragma sequence in a `call()` closure, then runs schema migration
**When to use:** All DB work; tokio-rusqlite pins the non-Send `rusqlite::Connection` to a dedicated thread

```rust
// Source: https://docs.rs/tokio-rusqlite/latest/tokio_rusqlite/
// Source: https://highperformancesqlite.com/articles/sqlite-recommended-pragmas
use tokio_rusqlite::Connection;

pub async fn open_db(path: &str) -> Result<Connection, tokio_rusqlite::Error> {
    let conn = Connection::open(path).await?;

    conn.call(|db| {
        db.execute_batch("
            PRAGMA journal_mode=WAL;
            PRAGMA synchronous=NORMAL;
            PRAGMA foreign_keys=ON;
        ")?;
        // busy_timeout is set via the rusqlite Connection method, not PRAGMA,
        // to guarantee it applies (tokio-rusqlite defaults to 5000ms anyway)
        db.busy_timeout(std::time::Duration::from_secs(10))?;
        Ok(())
    }).await?;

    // Run schema migration
    conn.call(|db| {
        db.execute_batch("
            PRAGMA wal_checkpoint(TRUNCATE);
            CREATE TABLE IF NOT EXISTS sessions ( ... );
            CREATE TABLE IF NOT EXISTS comments ( ... );
            CREATE TABLE IF NOT EXISTS threads ( ... );
        ")?;
        Ok(())
    }).await?;

    Ok(conn)
}
```

**Write transactions must use `BEGIN IMMEDIATE`:**
```rust
conn.call(|db| {
    db.execute_batch("BEGIN IMMEDIATE")?;
    // ... writes ...
    db.execute_batch("COMMIT")?;
    Ok(())
}).await?;
```

---

### Pattern 7: Main Event Loop
**What:** `tokio::main`, creates Tui and event handler, receives `AppEvent` from channel, renders only on `AppEvent::Render`
**When to use:** Single `terminal.draw()` call per `Render` event; never call `draw()` twice per frame

```rust
// Source: https://ratatui.rs/tutorials/counter-async-app/full-async-events/
#[tokio::main]
async fn main() -> std::io::Result<()> {
    // 1. Install panic hook FIRST (before init_tui which enters alternate screen)
    install_panic_hook();

    // 2. Register SIGTERM flag
    let term_flag = register_sigterm();

    // 3. Initialize terminal (stderr backend)
    let mut terminal = init_tui()?;

    // 4. Set up event channel and spawn event task
    let (tx, mut rx) = tokio::sync::mpsc::unbounded_channel::<AppEvent>();
    spawn_event_task(tx.clone());

    // 5. Open DB (before first frame — no loading spinner per perf requirements)
    let db = open_db(".airev/reviews.db").await?;

    // 6. Event loop
    loop {
        // Check SIGTERM on every iteration
        if term_flag.load(std::sync::atomic::Ordering::Relaxed) {
            break;
        }

        match rx.recv().await {
            Some(AppEvent::Render) => {
                terminal.draw(|frame| ui::render(frame, &state))?; // single draw() per Render
            }
            Some(AppEvent::Key(key)) => { /* handle key */ }
            Some(AppEvent::Resize(w, h)) => { /* force relayout */ }
            Some(AppEvent::Quit) => break,
            _ => {}
        }
    }

    // 7. Restore terminal on clean exit
    restore_tui()?;
    Ok(())
}
```

---

### Anti-Patterns to Avoid
- **Calling `ratatui::init()` for a stderr backend:** `init()` hardcodes stdout; using it with stderr rendering will leave the terminal in a broken state on panic because the panic hook calls the wrong `restore()`.
- **Calling `terminal.draw()` more than once per `Render` event:** The spec and ratatui docs are explicit — one `draw()` per frame cycle.
- **Using `BEGIN` (deferred) for writes in WAL mode:** Start write transactions with `BEGIN IMMEDIATE` to acquire the write lock upfront and avoid `SQLITE_BUSY` errors.
- **Not filtering `KeyEventKind::Press`:** On Windows, both `Press` and `Release` events fire; failing to filter causes every key to be processed twice.
- **Running schema migrations outside a `call()` closure:** rusqlite `Connection` is `!Send`; all access must go through `tokio_rusqlite::Connection::call()`.
- **Mixing `read()`/`poll()` with `EventStream`:** crossterm documents that combining blocking `read`/`poll` with `EventStream` on different threads is not allowed.

---

## Don't Hand-Roll

| Problem | Don't Build | Use Instead | Why |
|---------|-------------|-------------|-----|
| Terminal enter/exit, raw mode | Custom ANSI escape sequences | `crossterm::terminal::{enable_raw_mode, EnterAlternateScreen}` | Platform differences (Windows ConPTY, macOS, Linux) |
| Panic hook terminal restore | Ad-hoc cleanup in `main` | Manual `panic::set_hook` wrapping `restore_tui()` | `?` early returns bypass `main` cleanup |
| Non-blocking signal delivery | `unsafe` signal handler writing to channels | `signal_hook::flag::register` with `AtomicBool` | Only signal-safe primitives are legal in signal handlers |
| Async SQLite access | Spawning `tokio::task::spawn_blocking` manually | `tokio-rusqlite` | Handles the thread lifecycle, error mapping, and `!Send` constraint |
| MPSC event channel | Custom channel implementation | `tokio::sync::mpsc::unbounded_channel` | Battle-tested; handles wakeup correctly across threads |
| Per-cell diff in rendering | Tracking changed cells manually | ratatui's internal diffing (built-in) | ratatui only re-renders changed cells automatically |

**Key insight:** The most dangerous area is signal handling — the only operations safe inside a POSIX signal handler are async-signal-safe (writing to an `AtomicBool`, `write()` to a pipe). `signal_hook::flag` is explicitly designed for this constraint.

---

## Common Pitfalls

### Pitfall 1: Using `ratatui::init()` with stderr backend
**What goes wrong:** `ratatui::init()` initializes stdout as the backend and installs a panic hook that calls `ratatui::restore()`. If your terminal is actually on stderr, the panic hook restores the wrong stream, leaving the terminal corrupted.
**Why it happens:** The convenience function assumes stdout.
**How to avoid:** Use `Terminal::new(CrosstermBackend::new(BufWriter::new(stderr())))` + manual panic hook.
**Warning signs:** After a panic, the terminal is garbled even though you called `init()`.

### Pitfall 2: `Drop` does NOT automatically call `restore()`
**What goes wrong:** As of ratatui 0.30, `DefaultTerminal` does not implement `Drop` to call `restore()`. On normal exit, if `restore()` is not called explicitly, the terminal is left in raw mode / alternate screen.
**Why it happens:** The `Drop` impl was requested (GitHub issue #2087) but not yet merged as of this research date.
**How to avoid:** Always call `restore_tui()` (or `ratatui::restore()`) at every exit path — normal return, `?` propagation via separate function, and panic hook.
**Warning signs:** Terminal stays in alternate screen after application exits; cursor remains hidden.

### Pitfall 3: `BEGIN` vs `BEGIN IMMEDIATE` in WAL mode
**What goes wrong:** A `BEGIN` transaction in SQLite defers lock acquisition until the first write. In WAL mode with concurrent readers, a write that finds the DB busy will return `SQLITE_BUSY` immediately, even with `busy_timeout` set, if another write transaction is already active.
**Why it happens:** SQLite WAL only allows one writer at a time; `BEGIN` does not pre-acquire the write lock.
**How to avoid:** All write transactions use `BEGIN IMMEDIATE` to acquire the write lock upfront; `busy_timeout(10s)` handles retry.
**Warning signs:** Intermittent `SQLITE_BUSY` / "database is locked" errors during writes.

### Pitfall 4: `execute_batch` for `journal_mode` pragma — ordering matters
**What goes wrong:** `PRAGMA journal_mode=WAL` returns a result row (`"wal"`). In old rusqlite versions this was problematic; in 0.38, `execute_batch()` handles this correctly. However, the WAL pragma is persistent (file-level) while `synchronous` and `busy_timeout` are connection-level and must be re-set on every connection open.
**Why it happens:** Developers set WAL once and assume `synchronous=NORMAL` persists across reconnects.
**How to avoid:** Run the full pragma sequence in `open_db()` every time a connection is opened.
**Warning signs:** Writes become slower than expected after reconnect (reverted to `FULL` sync).

### Pitfall 5: `crossterm::EventStream` + `FutureExt::fuse()` required
**What goes wrong:** Using `reader.next()` directly in `tokio::select!` without `.fuse()` can cause the future to be polled after completion in some configurations.
**Why it happens:** `select!` requires futures to be fused when used in a loop to avoid polling a completed future.
**How to avoid:** Always write `reader.next().fuse()` and import `futures::FutureExt`.
**Warning signs:** Occasional panics from polling a completed stream.

### Pitfall 6: panic hook ordering with other error libraries
**What goes wrong:** If `color_eyre::install()` is called AFTER `install_panic_hook()`, the color-eyre hook fires before the terminal is restored, printing its output to the alternate screen.
**Why it happens:** Panic hooks are chained in reverse installation order — the last-installed hook fires first.
**How to avoid:** Install other panic hooks (color-eyre, etc.) BEFORE `install_panic_hook()`. The terminal-restoring hook must be the innermost (last installed, first to fire).
**Warning signs:** Panic output appears on alternate screen, or terminal is left in alternate screen mode after panic.

### Pitfall 7: SIGTERM on macOS requires `Ordering::Relaxed` to be checked in the loop
**What goes wrong:** The `AtomicBool` is set by the signal handler asynchronously. If the event loop is blocked on `rx.recv().await` and no events arrive, the SIGTERM flag is never checked.
**Why it happens:** `rx.recv()` blocks until an event arrives; SIGTERM does not itself inject an event.
**How to avoid:** Add a short timeout or ensure the tick interval drives the loop frequently enough. Alternatively add a `tokio::select!` branch polling the flag via `tokio::time::sleep(Duration::from_millis(100))`.
**Warning signs:** Process does not terminate when SIGTERM is sent; must use SIGKILL.

---

## Code Examples

Verified patterns from official sources:

### Cargo Workspace Root
```toml
# Source: https://doc.rust-lang.org/book/ch14-03-cargo-workspaces.html
[workspace]
resolver = "3"
members = ["airev", "airev-mcp", "airev-core"]

[workspace.dependencies]
ratatui = { version = "0.30", features = ["crossterm"] }
crossterm = { version = "0.29", features = ["event-stream"] }
tokio = { version = "1.49", features = ["full"] }
rusqlite = { version = "0.38", features = ["bundled"] }
tokio-rusqlite = "0.7"
signal-hook = "0.3"
futures = "0.3"
```

### crossterm EventStream with KeyEventKind::Press filter
```rust
// Source: https://ratatui.rs/tutorials/counter-async-app/async-event-stream/
use crossterm::event::{Event, EventStream, KeyEventKind};
use futures::{FutureExt, StreamExt};

let mut reader = EventStream::new();
let crossterm_event = reader.next().fuse();
tokio::select! {
    maybe_event = crossterm_event => {
        if let Some(Ok(Event::Key(key))) = maybe_event {
            if key.kind == KeyEventKind::Press {
                tx.send(AppEvent::Key(key)).ok();
            }
        }
    }
}
```

### rusqlite WAL pragma sequence
```rust
// Source: https://docs.rs/rusqlite/latest/rusqlite/struct.Connection.html
// Source: https://highperformancesqlite.com/articles/sqlite-recommended-pragmas
conn.call(|db| {
    db.execute_batch("
        PRAGMA journal_mode=WAL;
        PRAGMA synchronous=NORMAL;
        PRAGMA foreign_keys=ON;
    ")?;
    db.busy_timeout(std::time::Duration::from_secs(10))?;
    Ok(())
}).await?;
```

### signal-hook SIGTERM registration
```rust
// Source: https://docs.rs/signal-hook/latest/signal_hook/flag/index.html
use std::sync::{Arc, atomic::{AtomicBool, Ordering}};
use signal_hook::{consts::SIGTERM, flag::register};

let term = Arc::new(AtomicBool::new(false));
register(SIGTERM, Arc::clone(&term)).expect("SIGTERM registration failed");
// Check: term.load(Ordering::Relaxed)
```

### Manual panic hook (replicating ratatui::init() but for stderr)
```rust
// Source: https://ratatui.rs/recipes/apps/panic-hooks/
use std::panic;

let original_hook = panic::take_hook();
panic::set_hook(Box::new(move |panic_info| {
    let _ = restore_tui();   // restore stderr terminal first
    original_hook(panic_info);
}));
```

### `BEGIN IMMEDIATE` write transaction pattern
```rust
// Source: https://highperformancesqlite.com/articles/sqlite-recommended-pragmas
conn.call(|db| {
    let tx = db.transaction_with_behavior(
        rusqlite::TransactionBehavior::Immediate
    )?;
    tx.execute("INSERT INTO comments ...", params![...])?;
    tx.commit()?;
    Ok(())
}).await?;
```

---

## State of the Art

| Old Approach | Current Approach | When Changed | Impact |
|--------------|------------------|--------------|--------|
| `Terminal::new(CrosstermBackend::new(stdout()))` manually | `ratatui::init()` (stdout only) | ratatui 0.28.1 | Simplifies stdout apps; stderr apps must still use `Terminal::new()` |
| Manual panic hook in every app | `ratatui::init()` installs hook automatically | ratatui 0.28.1 | Reduces boilerplate; but not available for stderr backends |
| `ratatui::init()` + explicit `restore()` | `ratatui::run(closure)` | ratatui 0.30.0 | All-in-one; still stdout only |
| `resolver = "2"` in workspace | `resolver = "3"` (Rust 2024 edition) | Rust 1.84 (Feb 2025) | Better feature unification across workspace |
| Separate `Cargo.lock` per crate | Single shared `Cargo.lock` at workspace root | Always been workspace behavior | Prevents version divergence |

**Deprecated/outdated:**
- `Terminal::with_options()`: Still valid but `ratatui::init()` is preferred for stdout apps
- `crossterm::event::read()` / `poll()` in blocking mode: Valid for simple apps; EventStream is preferred for tokio async apps
- `signal_hook::iterator::Signals`: Valid but heavier than `flag::register` for a simple terminate flag

---

## Open Questions

1. **tokio-rusqlite 0.7 exact `open_with_flags` support**
   - What we know: `Connection::open(path)` exists; `call()` exposes raw rusqlite `Connection`
   - What's unclear: Whether tokio-rusqlite 0.7 exposes `open_with_flags` or if custom flags require calling into rusqlite via `call()` after open
   - Recommendation: Use `Connection::open(path)` and set all options via `call()` + `execute_batch()` + `busy_timeout()`; this is verified to work

2. **SIGTERM check latency when `rx.recv()` blocks**
   - What we know: `signal_hook::flag::register` sets the flag from the signal handler; it does not wake a tokio task
   - What's unclear: Whether the tick interval (250ms) is a sufficient check interval for the SIGTERM flag, or if a dedicated `tokio::signal::unix` waker should be added
   - Recommendation: Add a `tokio::select!` arm with a short `tokio::time::sleep(Duration::from_millis(50))` to ensure SIGTERM is detected within 50ms even if no events arrive

3. **`wal_checkpoint(TRUNCATE)` on open — contention risk**
   - What we know: The spec requires `PRAGMA wal_checkpoint(TRUNCATE)` on open
   - What's unclear: Whether TRUNCATE will block on open if another connection holds a read lock (it degrades to PASSIVE silently)
   - Recommendation: Accept the PASSIVE degradation; the WAL file size is bounded by auto-checkpoint at 1000 pages

---

## Sources

### Primary (HIGH confidence)
- [docs.rs/ratatui/0.30 — fn.init](https://docs.rs/ratatui/latest/ratatui/fn.init.html) — init(), restore(), DefaultTerminal, panic hook
- [ratatui.rs/highlights/v030](https://ratatui.rs/highlights/v030/) — ratatui 0.30 new features (run(), DefaultTerminal)
- [ratatui.rs/recipes/apps/panic-hooks](https://ratatui.rs/recipes/apps/panic-hooks/) — manual panic hook pattern
- [ratatui.rs/recipes/apps/terminal-and-event-handler](https://ratatui.rs/recipes/apps/terminal-and-event-handler/) — Tui struct with Drop and event handler
- [ratatui.rs/tutorials/counter-async-app/async-event-stream](https://ratatui.rs/tutorials/counter-async-app/async-event-stream/) — EventStream + separate tick/render intervals
- [ratatui.rs/tutorials/counter-async-app/full-async-events](https://ratatui.rs/tutorials/counter-async-app/full-async-events/) — full async event loop with AppEvent enum
- [docs.rs/crossterm/latest — EventStream](https://docs.rs/crossterm/latest/crossterm/event/struct.EventStream.html) — EventStream, event-stream feature
- [docs.rs/signal-hook/latest — flag](https://docs.rs/signal-hook/latest/signal_hook/flag/index.html) — register(), AtomicBool pattern
- [docs.rs/rusqlite/latest — Connection](https://docs.rs/rusqlite/latest/rusqlite/struct.Connection.html) — open(), execute_batch(), busy_timeout()
- [docs.rs/tokio-rusqlite/latest — Connection](https://docs.rs/tokio-rusqlite/latest/tokio_rusqlite/struct.Connection.html) — open(), call()
- [doc.rust-lang.org — Cargo Workspaces](https://doc.rust-lang.org/book/ch14-03-cargo-workspaces.html) — workspace structure, resolver, shared deps

### Secondary (MEDIUM confidence)
- [ratatui.rs/faq](https://ratatui.rs/faq/) — stderr vs stdout backend guidance (confirmed: stderr is the pattern for MCP/piped stdout architectures)
- [highperformancesqlite.com — recommended pragmas](https://highperformancesqlite.com/articles/sqlite-recommended-pragmas) — WAL + NORMAL + busy_timeout best practices
- [GitHub ratatui issue #2087](https://github.com/ratatui/ratatui/issues/2087) — confirms Drop does NOT auto-call restore() as of 2025
- [blog.orhun.dev — stdout vs stderr](https://blog.orhun.dev/stdout-vs-stderr/) — buffering differences; confirms stderr needs BufWriter

### Tertiary (LOW confidence)
- [ratatui async-template tui.rs](https://ratatui.github.io/async-template/02-structure.html) — Community template using `Backend<Stderr>` (unverified template status)

---

## Metadata

**Confidence breakdown:**
- Standard stack: HIGH — all versions verified on docs.rs and crates.io
- Architecture: HIGH — patterns taken from official ratatui docs and tutorials
- Pitfalls: HIGH — Drop/restore gap confirmed via GitHub issue; pragma ordering from SQLite official docs
- SIGTERM latency: MEDIUM — behavior of AtomicBool + rx.recv() interaction inferred from tokio semantics

**Research date:** 2026-02-18
**Valid until:** 2026-03-20 (30 days — ratatui and rusqlite are stable; check ratatui changelog if version bumps occur)
