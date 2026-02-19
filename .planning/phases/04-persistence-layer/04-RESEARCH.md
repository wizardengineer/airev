# Phase 4: Persistence Layer - Research

**Researched:** 2026-02-19
**Domain:** Rust async SQLite persistence with tokio-rusqlite, schema versioning, session lifecycle, AppEvent DB integration
**Confidence:** HIGH

<phase_requirements>
## Phase Requirements

| ID | Description | Research Support |
|----|-------------|-----------------|
| SQLite Persistence | WAL mode, BEGIN IMMEDIATE, busy_timeout, schema_version table, schema v1 tables (sessions, comments, file_review_state) | tokio-rusqlite 0.7 Connection::call() API verified; WAL + BEGIN IMMEDIATE pattern confirmed; existing open_db() foundation to build on |
| Session lifecycle | Create new session on first launch, auto-resume most recent open session, display session metadata in status bar | AppState extension pattern established; DbResult AppEvent variant already declared; session detection query documented |
| mark_file_reviewed | Toggle per-file reviewed state, persist to file_review_state table, checkmark in file list panel | file_review_state table in schema; write path via Connection::call() with BEGIN IMMEDIATE documented |
| DB schema versioning | migrations table, schema_version checked on open, forward migrations applied automatically | user_version PRAGMA approach (rusqlite_migration) vs manual schema_version table; requirements lock to manual schema_version table approach |
</phase_requirements>

---

## Summary

Phase 4 wires the persistence layer between the already-working `open_db()` foundation in `airev-core`
and the TUI's `AppState` + event loop. The `airev-core/src/db.rs` already opens the connection in WAL
mode and applies `SCHEMA_SQL` — but the schema itself must be replaced: the current schema uses
`AUTOINCREMENT INTEGER` primary keys, while requirements mandate UUID text IDs and different table
names (`file_review_state` instead of what Phase 3 added). The `airev-core/src/schema.rs` must be
rewritten to match the locked v1 schema exactly, and a `schema_version` table must be introduced so
that future migrations can be detected and applied on startup.

The TUI's event loop already has `AppEvent::DbResult` as a unit variant. This phase expands it into a
typed result carrier (analogous to how Phase 3 expanded `GitResult` from a unit variant to
`GitResult(Box<GitResultPayload>)`). DB operations are spawned as tokio tasks that call
`Connection::call()` and then send an `AppEvent::DbResult(DbResultPayload)` back over the unified
channel. The `Connection` struct in tokio-rusqlite 0.7 is `Clone + Send`, so it can be stored in
`AppState` or passed to spawned tasks without wrapping in `Arc<Mutex<_>>`.

The session lifecycle adds two startup paths to `main.rs`: detect if a session exists for the current
repo+mode+refs tuple (a simple `SELECT ... WHERE repo_path = ?` query), and either create a new
session or load the existing one. Both paths complete before the first frame renders — the
requirements state "no loading spinner; all reads complete before the first frame". This means the DB
reads in `main.rs` must be driven with `.await` directly in `main()` before the event loop starts,
using the same established pattern as the existing `open_db()` call.

**Primary recommendation:** Rewrite `airev-core/src/schema.rs` to the locked v1 schema with a
`schema_version` table, keep all DB logic in `airev-core/src/db.rs` functions (not inline in `main.rs`),
store the `tokio_rusqlite::Connection` in `AppState`, expand `AppEvent::DbResult` to carry typed
payloads, and use direct `.await` for startup reads before entering the event loop.

---

## Standard Stack

### Core

| Library | Version | Purpose | Why Standard |
|---------|---------|---------|--------------|
| `rusqlite` | 0.37.0 | Synchronous SQLite bindings, used inside Connection::call() closures | Locked to 0.37 in workspace Cargo.lock; tokio-rusqlite 0.7 requires exactly ^0.37 |
| `tokio-rusqlite` | 0.7.0 | Async wrapper; spawns one background thread per Connection; call() bridges tokio and rusqlite | Locked decision; already in workspace deps and Cargo.lock |
| `tokio` | 1.49 (full) | Async runtime; required by tokio-rusqlite and the main event loop | Locked decision; already in workspace |

### Supporting

| Library | Version | Purpose | When to Use |
|---------|---------|---------|-------------|
| `uuid` crate (optional) | not yet in workspace | Generate UUID v4 for session/comment IDs | Only add if requirements mandate UUID format in IDs — current schema.rs uses INTEGER; requirements say UUID text IDs. See Open Questions. |
| `std::time::SystemTime` | std | Get Unix timestamps for created_at, updated_at, reviewed_at | Use UNIX_EPOCH math: `SystemTime::now().duration_since(UNIX_EPOCH).unwrap().as_secs() as i64` |

### Alternatives Considered

| Instead of | Could Use | Tradeoff |
|------------|-----------|----------|
| Manual schema_version table | `rusqlite_migration` 2.4.0 crate | rusqlite_migration uses SQLite `user_version` PRAGMA (not a table), applies migrations atomically, has `.validate()` for testing. Requirements locked to a `schema_version` table — do not use rusqlite_migration |
| Manual schema_version table | `refinery` crate | refinery creates a migrations table but uses hash-based versioning; more complex than needed. Requirements locked to simple integer schema_version — do not use refinery |
| tokio-rusqlite for airev-mcp | synchronous `rusqlite` | Requirements specify `airev-mcp` uses synchronous rusqlite (no tokio-rusqlite) with same WAL + BEGIN IMMEDIATE config. Only the TUI process uses tokio-rusqlite |

**Installation (no new crates required for the TUI persistence layer):**

```bash
# tokio-rusqlite and rusqlite are already in workspace.dependencies
# If UUID text IDs are needed (see Open Questions), add:
cargo add uuid --features v4 -p airev-core
```

---

## Architecture Patterns

### Recommended Project Structure

```
airev-core/src/
├── db.rs           # open_db(), create_session(), resume_session(), load_session(),
│                   # mark_file_reviewed(), load_file_review_state(), save_comment()
├── schema.rs       # SCHEMA_V1_SQL const + migrate() function using schema_version table
├── types.rs        # Session, Comment, FileReviewState, DbResultPayload structs
└── lib.rs          # pub mod db; pub mod schema; pub mod types;

airev/src/
├── main.rs         # startup: open_db, detect/create session, store conn in AppState, load state
├── app.rs          # AppState: add db_conn, session, file_review_states, comments fields
└── event.rs        # AppEvent::DbResult(Box<DbResultPayload>) — typed, not unit variant
```

### Pattern 1: Connection::call() — the only rusqlite access pattern

**What:** All reads and writes go through `conn.call(|db| { ... })`. The closure receives
`&mut rusqlite::Connection` and runs synchronously in tokio-rusqlite's background thread.
Results are `.await`ed in the async context.

**When to use:** Every time you touch SQLite. Never call rusqlite methods directly on any
tokio task or the main event loop thread.

**Verified API (docs.rs/tokio-rusqlite/0.7.0):**

```rust
// Source: docs.rs/tokio-rusqlite/0.7.0/tokio_rusqlite/struct.Connection.html
pub async fn call<F, R, E>(&self, function: F) -> Result<R, Error<E>>
where
    F: FnOnce(&mut Connection) -> Result<R, E> + 'static + Send,
    R: Send + 'static,
    E: Send + 'static,
```

**Error wrapping for main() compatibility (locked decision):**

```rust
// Source: requirements + tokio-rusqlite Error<E> implements std::error::Error
conn.call(|db| { /* ... */ Ok(()) })
    .await
    .map_err(|e| std::io::Error::new(std::io::ErrorKind::Other, e))?;
```

### Pattern 2: Write Transactions with BEGIN IMMEDIATE (locked decision)

**What:** Every write transaction must use `TransactionBehavior::Immediate` to prevent
lock-upgrade races when multiple processes share the WAL file.

**Why it matters:** `BEGIN DEFERRED` (rusqlite's default) starts a read lock; if a write is
needed later in the same transaction, SQLite must upgrade to a write lock. In WAL mode with
concurrent writers (TUI + MCP server), this upgrade can fail with `SQLITE_BUSY` even when
`busy_timeout` is set. `BEGIN IMMEDIATE` acquires the write lock upfront.

**Verified source:** sqlite.org/wal.html + berthug.eu/articles on SQLITE_BUSY despite timeout

```rust
// Source: rusqlite docs - transaction_with_behavior signature verified
// Source: requirements spec — "All write transactions use BEGIN IMMEDIATE"
conn.call(|db| {
    let tx = db.transaction_with_behavior(
        rusqlite::TransactionBehavior::Immediate,
    )?;
    tx.execute("INSERT INTO sessions ...", rusqlite::params![...])?;
    tx.commit()?;
    Ok(())
}).await.map_err(|e| std::io::Error::new(std::io::ErrorKind::Other, e))?;
```

### Pattern 3: Schema Versioning with schema_version Table

**What:** On every `open_db()` call, check `schema_version` table for current version integer.
If the table does not exist (version 0), run the full v1 DDL inside a `BEGIN IMMEDIATE` transaction,
then insert `version = 1`. If the table exists and `version >= 1`, skip.

**Requirements lock this to a manual table** (not `user_version` PRAGMA or `rusqlite_migration` crate).

```rust
// Schema versioning DDL — run inside BEGIN IMMEDIATE before applying any schema
const SCHEMA_VERSION_DDL: &str = "
    CREATE TABLE IF NOT EXISTS schema_version (
        version INTEGER NOT NULL
    ) STRICT;
";

// In migrate() function:
fn migrate(db: &rusqlite::Connection) -> rusqlite::Result<()> {
    db.execute_batch(SCHEMA_VERSION_DDL)?;
    let version: i64 = db
        .query_row(
            "SELECT COALESCE(MAX(version), 0) FROM schema_version",
            [],
            |r| r.get(0),
        )
        .unwrap_or(0);
    if version < 1 {
        let tx = db.transaction_with_behavior(
            rusqlite::TransactionBehavior::Immediate,
        )?;
        tx.execute_batch(SCHEMA_V1_SQL)?;
        tx.execute("INSERT INTO schema_version (version) VALUES (1)", [])?;
        tx.commit()?;
    }
    Ok(())
}
```

### Pattern 4: Expanding AppEvent::DbResult from Unit to Typed Carrier

**What:** `AppEvent::DbResult` is currently a unit variant. Expand it to
`DbResult(Box<DbResultPayload>)` following the exact same pattern used in Phase 3 for
`AppEvent::GitResult(Box<GitResultPayload>)`.

**Why `Box<>`:** Keeps the AppEvent enum size small (one pointer width) regardless of payload size.

**`#[non_exhaustive]` protection:** The existing `_ => {}` match arms in `main.rs` already
handle unknown variants, so adding data to DbResult only requires adding one new match arm,
not updating all existing match arms.

```rust
// event.rs additions
#[derive(Debug)]
pub enum DbResultPayload {
    SessionLoaded(airev_core::types::Session),
    SessionCreated(airev_core::types::Session),
    FileReviewStateLoaded(Vec<(String, bool)>),  // (file_path, reviewed)
    ReviewToggled { file_path: String, reviewed: bool },
    CommentSaved(airev_core::types::Comment),
    // Additional variants added as needed by future phase tasks
}

// In AppEvent enum (event.rs):
DbResult(Box<DbResultPayload>),  // was: DbResult (unit)
```

### Pattern 5: Startup Reads Before First Frame (no loading spinner)

**What:** Requirements: "no loading spinner; all reads complete before the first frame."
Drive all session detect/resume reads synchronously via `.await` in `main()` before entering
the event loop, following the existing pattern for `open_db()`.

```rust
// main.rs startup sequence (after existing open_db call):
let conn = airev_core::db::open_db(".airev/reviews.db")
    .await
    .map_err(|e| std::io::Error::new(std::io::ErrorKind::Other, e))?;

// Detect or create session before first draw
let session = airev_core::db::detect_or_create_session(
    &conn, repo_path, diff_mode_str, diff_args_str,
)
.await
.map_err(|e| std::io::Error::new(std::io::ErrorKind::Other, e))?;

// Load file review state for this session
let review_states = airev_core::db::load_file_review_state(&conn, session.id)
    .await
    .map_err(|e| std::io::Error::new(std::io::ErrorKind::Other, e))?;

// Store conn and session in AppState BEFORE entering the event loop
state.db_conn = Some(conn);
state.session = Some(session);
state.file_review_states = review_states;

// Now enter the event loop — first Render tick sees complete state
```

### Pattern 6: Async DB Write from Keybinding (mark_file_reviewed)

**What:** When the user presses `r` on a file, the keybinding handler cannot `await` (it runs
in the synchronous keybinding dispatch path). Spawn a tokio task that calls `conn.call()` and
sends the result back over the event channel.

**Key insight:** `tokio_rusqlite::Connection` implements `Clone + Send`. Store it in `AppState`
and clone it into the spawned task. The spawned task returns `AppEvent::DbResult(...)` back
over the existing `tx` channel.

```rust
// In keybindings.rs handle_key(), KeyAction::ToggleReview arm:
if let (Some(conn), Some(session)) = (&state.db_conn, &state.session) {
    let conn = conn.clone();  // cheap: just clones the Arc<_> handle inside
    let session_id = session.id;
    let file_path = state.current_file_path().to_owned();
    let tx = state.event_tx.clone();  // AppState needs event_tx field added
    tokio::spawn(async move {
        let result = airev_core::db::toggle_file_reviewed(&conn, session_id, &file_path)
            .await;
        match result {
            Ok(reviewed) => {
                let _ = tx.send(AppEvent::DbResult(Box::new(
                    DbResultPayload::ReviewToggled { file_path, reviewed },
                )));
            }
            Err(e) => eprintln!("DB error toggling review: {e}"),
        }
    });
}
```

### Anti-Patterns to Avoid

- **Never call rusqlite directly from the async event loop:** rusqlite's `Connection` is not
  `Send`; the tokio-rusqlite `Connection` must be used instead, and only via `call()`.
- **Never use `BEGIN DEFERRED` for writes:** rusqlite's `conn.transaction()` default is
  `DEFERRED`. Always use `transaction_with_behavior(Immediate)` for writes. Forgetting this
  causes intermittent `SQLITE_BUSY` when the MCP server writes concurrently.
- **Never set `busy_timeout` via PRAGMA string:** Requirements and existing `open_db()` both
  specify `db.busy_timeout(Duration::from_secs(5))` — use the rusqlite method, not the pragma,
  to ensure the setting applies correctly regardless of pragma caching.
- **Never block tokio tasks inside `conn.call()` with `std::thread::sleep`:** `conn.call()`
  runs in a background thread, but sleeping in that thread blocks the rusqlite thread pool
  indefinitely for that connection.
- **Never reuse `AppEvent::DbResult` as a unit variant:** The existing `_ => {}` arm in the
  main event loop will silently drop unit variants. Expand it to carry the payload before wiring
  any DB operations.

---

## Don't Hand-Roll

| Problem | Don't Build | Use Instead | Why |
|---------|-------------|-------------|-----|
| Async SQLite access | Custom thread + mpsc channels to rusqlite | `tokio-rusqlite` Connection::call() | Already in workspace; handles thread lifetime, oneshot channels, error propagation correctly |
| Schema migration tracking | Custom hash-based migration files | Manual `schema_version` INTEGER table (as locked by requirements) | Requirements explicitly specify schema_version table; no external crate needed for one migration |
| Concurrent write safety | Custom locking / Mutex around Connection | WAL mode + `BEGIN IMMEDIATE` (already in open_db) | SQLite WAL + IMMEDIATE is the standard pattern; external locking would serialize reads unnecessarily |
| UUID generation | Custom random ID | `uuid` crate v4 feature | If text UUIDs are needed, uuid 1.x has `Uuid::new_v4().to_string()`. Do not hand-roll random hex strings |

**Key insight:** The tokio-rusqlite `Connection::call()` pattern is the only safe way to bridge
synchronous rusqlite with the async tokio event loop. Any attempt to "simplify" by calling rusqlite
directly from an async context will either fail to compile (`!Send`) or block the executor thread.

---

## Common Pitfalls

### Pitfall 1: Schema Mismatch — Current schema.rs vs Requirements

**What goes wrong:** The existing `airev-core/src/schema.rs` defines `sessions` with
`INTEGER PRIMARY KEY AUTOINCREMENT`, `threads`, and `comments` tables. Requirements mandate
different tables: `sessions` (UUID text id), `comments` (different columns), and
`file_review_state` (new table; no `threads` in v1 — threads are for Phase 7 MCP).

**Why it happens:** Phase 1/2 scaffolded a placeholder schema; Phase 4 finalizes it.

**How to avoid:** Completely replace `SCHEMA_SQL` in `schema.rs` with the v1 schema per
requirements. The `db.rs` `open_db()` function applies `SCHEMA_SQL` inside `BEGIN IMMEDIATE` —
this is the correct integration point. Also ensure `migrate()` is called from `open_db()` so
future schema bumps are applied automatically.

**Warning signs:** `rusqlite::Error::SqliteFailure` with "table already exists" (if schema_version
not checked before applying DDL) or missing `file_review_state` table errors at runtime.

**Note on threads table:** The phase description mentions `threads` table as a deliverable, but
the v1 requirements schema for the SQLite Persistence requirement does NOT include `threads`. The
`threads` table is described as "needed for MCP" (Phase 7). Research recommendation: add `threads`
DDL to `SCHEMA_V1_SQL` to satisfy the phase exit criterion ("inspect DB with sqlite3 and confirm
multi-round design") but make it optional/dormant in the TUI (no write path touches it in Phase 4).

### Pitfall 2: AppEvent::DbResult Unit Variant — Silent Drop

**What goes wrong:** `main.rs` event loop has `_ => {}` as a catch-all. If `DbResult` remains
a unit variant and DB tasks send it, the event loop silently ignores all DB results.

**Why it happens:** Phase 1 scaffolded `DbResult` as a placeholder unit variant. Phase 4
must promote it to `DbResult(Box<DbResultPayload>)` before wiring any DB result paths.

**How to avoid:** Expand the variant first (Task 1), then add the match arm in `main.rs`,
then wire DB operations. Do not wire DB operations before the match arm exists.

**Warning signs:** DB writes succeed but AppState never updates; reviewed files don't show
checkmarks even though `sqlite3` confirms the write happened.

### Pitfall 3: busy_timeout Not Applied to New Connections

**What goes wrong:** `busy_timeout` is a per-connection setting. If any code path opens a new
`tokio_rusqlite::Connection` without calling `db.busy_timeout()` inside `call()`, concurrent
writes from the MCP server will immediately return `SQLITE_BUSY` with no retry.

**Why it happens:** The existing `open_db()` sets `busy_timeout` correctly. New callers that
open connections themselves (e.g., in tests, or if airev-mcp calls open directly) may forget.

**How to avoid:** All connection-open code must go through `airev_core::db::open_db()`. Never
open a connection directly with `tokio_rusqlite::Connection::open()` without subsequently
calling `db.busy_timeout()` inside a `call()` closure.

**Warning signs:** `SQLITE_BUSY` errors in stderr during concurrent test runs.

### Pitfall 4: Storing Connection in AppState with Borrow Issues

**What goes wrong:** Attempting to pass `state.db_conn` into a `tokio::spawn` closure fails
because `AppState` is owned by the event loop and cannot be moved into multiple spawned tasks.

**Why it happens:** `tokio::spawn` requires `'static` bounds; borrowing from `AppState`
doesn't satisfy this.

**How to avoid:** `tokio_rusqlite::Connection` is `Clone + Send`. Store `Option<Connection>`
in `AppState` and clone it before spawning:
```rust
let conn = state.db_conn.as_ref().unwrap().clone();
tokio::spawn(async move { conn.call(|db| { ... }).await });
```
Each clone is just an `Arc` increment — it points to the same background thread.

**Warning signs:** Compiler error "cannot move out of `state.db_conn` which is behind a mutable reference."

### Pitfall 5: wal_checkpoint Called on MCP Connection

**What goes wrong:** `PRAGMA wal_checkpoint(TRUNCATE)` is meant to be called once per
startup by the TUI process. If `airev-mcp` also calls it at startup, and both processes race
on checkpoint, the TUI may see an inconsistent WAL state.

**Why it happens:** `open_db()` in `airev-core` is shared by both binaries. If airev-mcp
uses the same `open_db()`, it will also run the checkpoint.

**How to avoid:** Pass a flag to `open_db()` (or provide a separate `open_db_readonly()`) for
the MCP process. Requirements state: "`PRAGMA wal_checkpoint(TRUNCATE)` called once on startup
from the TUI process" — the MCP process must not call it.

**Warning signs:** "database disk image is malformed" or data inconsistencies after concurrent
TUI + MCP startup.

### Pitfall 6: Session ID Type — Integer vs UUID Text

**What goes wrong:** The current `types.rs` defines `Session.id: i64`, matching the current
`AUTOINCREMENT INTEGER` schema. Requirements mandate `id (UUID)`. If left as INTEGER, the MCP
server (Phase 7) will receive numeric session IDs, breaking the UUID contract.

**Why it happens:** Phase 1 scaffolded types with integers; requirements updated to UUIDs.

**How to avoid:** Change schema to `id TEXT PRIMARY KEY` (UUID stored as text), update `Session`
struct to `id: String`, update all queries. Add `uuid` crate with `v4` feature to `airev-core`.

**Warning signs:** MCP tool `get_session` receives `"42"` instead of a UUID string.

---

## Code Examples

Verified patterns from official sources:

### Full open_db() with Schema Versioning

```rust
// Source: docs.rs/tokio-rusqlite/0.7.0 + rusqlite docs + requirements spec
// airev-core/src/db.rs

use std::time::Duration;
use tokio_rusqlite::Connection;
use crate::schema::{SCHEMA_VERSION_DDL, SCHEMA_V1_SQL};

pub async fn open_db(path: &str) -> Result<Connection, tokio_rusqlite::Error<rusqlite::Error>> {
    let conn = Connection::open(path).await?;

    conn.call(|db| {
        // WAL pragmas — applied on every open (connection-level settings)
        db.execute_batch(
            "PRAGMA journal_mode=WAL;
             PRAGMA synchronous=NORMAL;
             PRAGMA foreign_keys=ON;",
        )?;
        // busy_timeout via Connection method, not PRAGMA string (locked requirement)
        db.busy_timeout(Duration::from_secs(5))?;
        Ok(())
    })
    .await?;

    // wal_checkpoint once on startup (TUI process only — not MCP process)
    conn.call(|db| {
        db.execute_batch("PRAGMA wal_checkpoint(TRUNCATE);")?;
        Ok(())
    })
    .await?;

    // Apply schema migrations inside BEGIN IMMEDIATE
    conn.call(|db| {
        crate::schema::migrate(db)
    })
    .await?;

    Ok(conn)
}
```

### schema.rs — v1 Schema DDL

```rust
// Source: requirements spec v1 schema (SQLite Persistence requirement)
// airev-core/src/schema.rs

pub const SCHEMA_VERSION_DDL: &str = "
    CREATE TABLE IF NOT EXISTS schema_version (
        version INTEGER NOT NULL
    ) STRICT;
";

// Note: threads table included for Phase 7 MCP compatibility, but not written to in Phase 4
pub const SCHEMA_V1_SQL: &str = "
    CREATE TABLE IF NOT EXISTS sessions (
        id          TEXT    PRIMARY KEY,       -- UUID v4
        repo_path   TEXT    NOT NULL,
        diff_mode   TEXT    NOT NULL,
        diff_args   TEXT    NOT NULL DEFAULT '',
        created_at  INTEGER NOT NULL,          -- Unix timestamp seconds
        updated_at  INTEGER NOT NULL
    ) STRICT;

    CREATE TABLE IF NOT EXISTS comments (
        id          TEXT    PRIMARY KEY,       -- UUID v4
        session_id  TEXT    NOT NULL REFERENCES sessions(id) ON DELETE CASCADE,
        file_path   TEXT    NOT NULL,
        line_number INTEGER,
        hunk_offset INTEGER,
        comment_type TEXT   NOT NULL
                            CHECK(comment_type IN
                                  ('question','concern','til','suggestion','praise','nitpick')),
        severity    TEXT    NOT NULL
                            CHECK(severity IN ('critical','major','minor','info')),
        body        TEXT    NOT NULL,
        created_at  INTEGER NOT NULL,
        resolved_at INTEGER,
        thread_id   TEXT    REFERENCES threads(id) ON DELETE SET NULL
    ) STRICT;

    CREATE TABLE IF NOT EXISTS file_review_state (
        session_id  TEXT    NOT NULL REFERENCES sessions(id) ON DELETE CASCADE,
        file_path   TEXT    NOT NULL,
        reviewed    INTEGER NOT NULL DEFAULT 0,  -- BOOLEAN stored as 0/1
        reviewed_at INTEGER,
        PRIMARY KEY (session_id, file_path)
    ) STRICT;

    CREATE TABLE IF NOT EXISTS threads (
        id          TEXT    PRIMARY KEY,       -- UUID v4
        session_id  TEXT    NOT NULL REFERENCES sessions(id) ON DELETE CASCADE,
        status      TEXT    NOT NULL DEFAULT 'open'
                            CHECK(status IN ('open', 'addressed', 'resolved')),
        round_number INTEGER NOT NULL DEFAULT 1
    ) STRICT;
";

pub fn migrate(db: &rusqlite::Connection) -> rusqlite::Result<()> {
    db.execute_batch(SCHEMA_VERSION_DDL)?;
    let version: i64 = db
        .query_row(
            "SELECT COALESCE(MAX(version), 0) FROM schema_version",
            [],
            |r| r.get(0),
        )
        .unwrap_or(0);
    if version < 1 {
        let tx = db.transaction_with_behavior(
            rusqlite::TransactionBehavior::Immediate,
        )?;
        tx.execute_batch(SCHEMA_V1_SQL)?;
        tx.execute("INSERT INTO schema_version (version) VALUES (1)", [])?;
        tx.commit()?;
    }
    Ok(())
}
```

### detect_or_create_session()

```rust
// Source: requirements spec + verified tokio-rusqlite call() pattern
// airev-core/src/db.rs

pub async fn detect_or_create_session(
    conn: &Connection,
    repo_path: &str,
    diff_mode: &str,
    diff_args: &str,
) -> Result<crate::types::Session, tokio_rusqlite::Error<rusqlite::Error>> {
    let repo_path = repo_path.to_owned();
    let diff_mode = diff_mode.to_owned();
    let diff_args = diff_args.to_owned();

    conn.call(move |db| {
        // Try to find the most recent session for this repo+mode+args
        let existing: Option<crate::types::Session> = db
            .query_row(
                "SELECT id, repo_path, diff_mode, diff_args, created_at, updated_at
                 FROM sessions
                 WHERE repo_path = ?1 AND diff_mode = ?2 AND diff_args = ?3
                 ORDER BY updated_at DESC
                 LIMIT 1",
                rusqlite::params![&repo_path, &diff_mode, &diff_args],
                |r| {
                    Ok(crate::types::Session {
                        id: r.get(0)?,
                        repo_path: r.get(1)?,
                        diff_mode: r.get(2)?,
                        diff_args: r.get(3)?,
                        created_at: r.get(4)?,
                        updated_at: r.get(5)?,
                    })
                },
            )
            .optional()?;

        if let Some(session) = existing {
            // Resume: update last_opened timestamp
            let now = std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap()
                .as_secs() as i64;
            let tx = db.transaction_with_behavior(
                rusqlite::TransactionBehavior::Immediate,
            )?;
            tx.execute(
                "UPDATE sessions SET updated_at = ?1 WHERE id = ?2",
                rusqlite::params![now, &session.id],
            )?;
            tx.commit()?;
            Ok(session)
        } else {
            // Create new session
            let id = uuid::Uuid::new_v4().to_string();
            let now = std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap()
                .as_secs() as i64;
            let tx = db.transaction_with_behavior(
                rusqlite::TransactionBehavior::Immediate,
            )?;
            tx.execute(
                "INSERT INTO sessions (id, repo_path, diff_mode, diff_args, created_at, updated_at)
                 VALUES (?1, ?2, ?3, ?4, ?5, ?5)",
                rusqlite::params![&id, &repo_path, &diff_mode, &diff_args, now],
            )?;
            tx.commit()?;
            Ok(crate::types::Session {
                id,
                repo_path,
                diff_mode,
                diff_args,
                created_at: now,
                updated_at: now,
            })
        }
    })
    .await
}
```

### toggle_file_reviewed() Write Path

```rust
// Source: requirements + rusqlite upsert pattern
// airev-core/src/db.rs

pub async fn toggle_file_reviewed(
    conn: &Connection,
    session_id: &str,
    file_path: &str,
) -> Result<bool, tokio_rusqlite::Error<rusqlite::Error>> {
    let session_id = session_id.to_owned();
    let file_path = file_path.to_owned();

    conn.call(move |db| {
        let now = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_secs() as i64;

        // Read current reviewed state (0 if no row)
        let current: bool = db
            .query_row(
                "SELECT reviewed FROM file_review_state
                 WHERE session_id = ?1 AND file_path = ?2",
                rusqlite::params![&session_id, &file_path],
                |r| r.get::<_, bool>(0),
            )
            .unwrap_or(false);

        let new_state = !current;
        let reviewed_at = if new_state { Some(now) } else { None };

        let tx = db.transaction_with_behavior(
            rusqlite::TransactionBehavior::Immediate,
        )?;
        tx.execute(
            "INSERT INTO file_review_state (session_id, file_path, reviewed, reviewed_at)
             VALUES (?1, ?2, ?3, ?4)
             ON CONFLICT(session_id, file_path)
             DO UPDATE SET reviewed = excluded.reviewed,
                           reviewed_at = excluded.reviewed_at",
            rusqlite::params![&session_id, &file_path, new_state, reviewed_at],
        )?;
        tx.commit()?;
        Ok(new_state)
    })
    .await
}
```

### Concurrent Write Test (exit criterion)

```rust
// Source: exit criterion — "concurrent writes from two processes, no SQLITE_BUSY in stderr"
// This is an integration test, not unit test

#[tokio::test]
async fn test_concurrent_writes_no_busy() {
    let dir = tempfile::tempdir().unwrap();
    let db_path = dir.path().join("reviews.db").to_str().unwrap().to_owned();

    // Open two connections simulating TUI + MCP
    let conn1 = airev_core::db::open_db(&db_path).await.unwrap();
    let conn2 = airev_core::db::open_db(&db_path).await.unwrap();

    // Spawn concurrent writes on both connections
    let h1 = tokio::spawn(async move {
        for _ in 0..10 {
            conn1.call(|db| {
                let tx = db.transaction_with_behavior(
                    rusqlite::TransactionBehavior::Immediate,
                )?;
                // ... insert session ...
                tx.commit()
            }).await.expect("conn1 write should not return SQLITE_BUSY");
        }
    });
    let h2 = tokio::spawn(async move {
        for _ in 0..10 {
            conn2.call(|db| {
                let tx = db.transaction_with_behavior(
                    rusqlite::TransactionBehavior::Immediate,
                )?;
                // ... insert session ...
                tx.commit()
            }).await.expect("conn2 write should not return SQLITE_BUSY");
        }
    });

    let (r1, r2) = tokio::join!(h1, h2);
    r1.unwrap();
    r2.unwrap();
}
```

---

## State of the Art

| Old Approach | Current Approach | When Changed | Impact |
|--------------|------------------|--------------|--------|
| Schema in `schema.rs` with INTEGER ids + AUTOINCREMENT | UUID text ids, `file_review_state` table, schema_version table | Phase 4 | Must rewrite `schema.rs` entirely; existing data (none in production yet) lost |
| `AppEvent::DbResult` unit variant | `AppEvent::DbResult(Box<DbResultPayload>)` typed | Phase 4 | Same pattern as Phase 3 GitResult expansion |
| `Connection` created and discarded in `main.rs` | `Connection` stored in `AppState`, cloned for async tasks | Phase 4 | Enables write paths from keybinding handlers |
| Session state not tracked | `AppState.session: Option<Session>` + startup detect/resume | Phase 4 | Enables status bar display, comment association |
| No file review state | `AppState.file_review_states: HashMap<String, bool>` | Phase 4 | Enables checkmark in file list panel |

**Deprecated/outdated:**
- Current `airev-core/src/schema.rs` `SCHEMA_SQL` const: replace entirely in Phase 4
- Current `airev-core/src/types.rs` `Session.id: i64`: change to `String` for UUID
- Current `airev-core/src/types.rs` `Comment.id: i64`: change to `String` for UUID

---

## Open Questions

1. **UUID vs INTEGER for session/comment IDs**
   - What we know: Requirements say `id (UUID)` for both `sessions` and `comments` tables. Current `types.rs` uses `i64`. The `uuid` crate is not yet in the workspace.
   - What's unclear: Whether the planner should add `uuid` crate to `airev-core` in a separate task, or inline it with the schema rewrite task.
   - Recommendation: Add `uuid = { version = "1", features = ["v4"] }` to `airev-core/Cargo.toml` in the schema rewrite task. No new workspace dep needed unless `airev` or `airev-mcp` also need UUID generation.

2. **threads table in Phase 4 vs Phase 7**
   - What we know: Phase 4 description says "threads table schema is in place." Requirements SQLite Persistence spec does NOT list `threads` in the v1 schema. The MCP section says threads are needed for Phase 7.
   - What's unclear: Should `threads` DDL be in `SCHEMA_V1_SQL` now, or deferred to a `SCHEMA_V2_SQL` migration in Phase 7?
   - Recommendation: Include `threads` DDL in `SCHEMA_V1_SQL` now. The exit criterion requires `sqlite3 .airev/reviews.db "SELECT * FROM threads;"` to succeed, which requires the table to exist. No write path for threads in Phase 4 — the table exists but is empty.

3. **comment_count on session resume prompt**
   - What we know: Requirements say "On launch, airev detects an existing session... offers to resume it (shows comment count)."
   - What's unclear: The resume prompt is a TUI modal or status bar message. No modal infrastructure has been built yet. Phase 4 may need a minimal "resume session?" prompt before the main event loop, or could defer the prompt UX to a later phase.
   - Recommendation: Implement the session resume as automatic (always resume most recent session) with status bar display of "Session resumed: N comments" rather than an interactive prompt. The interactive prompt is a UX refinement that can be added in Phase 5.

4. **event_tx in AppState for spawned DB tasks**
   - What we know: The `AppEvent::DbResult` approach requires spawned tokio tasks to send back over the channel. The `handler.tx` sender is currently local to `main.rs`.
   - What's unclear: Whether `AppState` should hold a clone of `event_tx` (for use in `handle_key`), or whether `handle_key` returns a `DbWriteRequest` enum that `main.rs` dispatches.
   - Recommendation: Add `event_tx: Option<tokio::sync::mpsc::UnboundedSender<AppEvent>>` to `AppState`. Store a clone of `handler.tx` there during startup. This follows the existing pattern where `AppState.git_tx` is used for git requests.

---

## Sources

### Primary (HIGH confidence)

- `docs.rs/tokio-rusqlite/0.7.0` — Connection::call() signature, Error<E> enum variants, Connection::open() signature (verified via WebFetch 2026-02-19)
- `docs.rs/rusqlite/0.37.0` — busy_timeout() method, transaction_with_behavior() method signature (verified via WebFetch 2026-02-19)
- `sqlite.org/wal.html` — WAL mode concurrency model, single-writer constraint, SQLITE_BUSY conditions (verified via WebFetch 2026-02-19)
- `/airev-core/src/db.rs` — existing open_db() implementation using correct patterns (read directly from codebase)
- `/airev-core/src/schema.rs` — existing SCHEMA_SQL showing what must be replaced (read directly from codebase)
- `/airev-core/src/types.rs` — existing type definitions showing integer IDs that must change (read directly from codebase)
- `/airev/src/event.rs` — `AppEvent::DbResult` as unit variant confirmed (read directly from codebase)
- `/airev/src/main.rs` — startup sequence, existing open_db() call pattern, event loop structure (read directly from codebase)

### Secondary (MEDIUM confidence)

- `berthug.eu/articles/posts/a-brief-post-on-sqlite3-database-locked-despite-timeout/` — SQLITE_BUSY despite timeout when upgrading from DEFERRED to write; BEGIN IMMEDIATE prevention (WebSearch 2026-02-19, cross-verified with sqlite.org WAL docs)
- `docs.rs/rusqlite_migration/latest` — rusqlite_migration uses user_version not a table; confirms manual schema_version table is a deliberate, valid alternative (WebSearch 2026-02-19)
- WebSearch multi-source consensus on BEGIN IMMEDIATE as correct pattern for WAL multi-writer (2026-02-19)

### Tertiary (LOW confidence)

- WebSearch on AppEvent + DbResult TUI pattern — general architectural description, not a specific authoritative source

---

## Metadata

**Confidence breakdown:**
- Standard stack: HIGH — rusqlite 0.37 and tokio-rusqlite 0.7 are in Cargo.lock; APIs verified via docs.rs
- Architecture: HIGH — Connection::call() pattern is verified; AppEvent expansion follows the established Phase 3 GitResult pattern already in the codebase
- Pitfalls: HIGH — schema mismatch and unit variant issues are directly visible from reading the codebase; SQLITE_BUSY analysis cross-verified with sqlite.org
- Schema versioning: MEDIUM — requirements lock to manual schema_version table (not rusqlite_migration); the specific SQL for migrate() is designed to spec but not battle-tested in this codebase

**Research date:** 2026-02-19
**Valid until:** 2026-04-19 (rusqlite and tokio-rusqlite are slow-moving stable crates)
