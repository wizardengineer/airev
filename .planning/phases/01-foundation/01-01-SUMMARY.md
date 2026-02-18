---
phase: 01-foundation
plan: "01"
subsystem: infra
tags: [rust, cargo, workspace, sqlite, tokio-rusqlite, ratatui, rmcp]

# Dependency graph
requires: []
provides:
  - Cargo workspace with resolver=3 and three crates: airev, airev-mcp, airev-core
  - airev-core::types: Session, Comment, Hunk, DiffLine, DiffLineKind
  - airev-core::db: open_db() with WAL pragma sequence and schema DDL
  - airev-core::schema: SCHEMA_SQL DDL for sessions, threads, comments tables
  - Dependency isolation: airev has no rmcp; airev-mcp has no ratatui/crossterm
affects:
  - 01-02: Rendering skeleton (uses airev workspace structure)
  - 01-03: Git layer (uses airev-core types)
  - 01-04: Persistence layer (uses open_db and schema)

# Tech tracking
tech-stack:
  added:
    - rusqlite 0.37 (bundled SQLite)
    - tokio-rusqlite 0.7 (async wrapper)
    - tokio 1.49 (async runtime)
    - ratatui 0.30 with crossterm feature
    - crossterm 0.29 with event-stream
    - signal-hook 0.3
    - futures 0.3
    - rmcp 0.16 (MCP server SDK, airev-mcp only)
  patterns:
    - All rusqlite access via tokio_rusqlite::Connection::call() closures
    - All write transactions use BEGIN IMMEDIATE (TransactionBehavior::Immediate)
    - busy_timeout via Connection method (not PRAGMA string)
    - PRAGMA sequence: journal_mode=WAL, synchronous=NORMAL, foreign_keys=ON
    - wal_checkpoint(TRUNCATE) as maintenance batch after WAL config on open

key-files:
  created:
    - Cargo.toml (workspace root)
    - Cargo.lock
    - airev/Cargo.toml
    - airev/src/main.rs
    - airev-mcp/Cargo.toml
    - airev-mcp/src/main.rs
    - airev-core/Cargo.toml
    - airev-core/src/lib.rs
    - airev-core/src/types.rs
    - airev-core/src/schema.rs
    - airev-core/src/db.rs
  modified: []

key-decisions:
  - "rusqlite downgraded from 0.38 to 0.37 to match tokio-rusqlite 0.7 requirement (version conflict auto-fix)"
  - "busy_timeout set via Connection method, not PRAGMA string, per locked architectural requirement"
  - "Schema DDL applied inside BEGIN IMMEDIATE transaction per locked write transaction requirement"
  - "wal_checkpoint(TRUNCATE) run as plain maintenance batch (not inside a transaction)"

patterns-established:
  - "Pattern 1: All DB calls go through conn.call(|db| { ... }) closure — never call rusqlite directly outside call()"
  - "Pattern 2: All write transactions use db.transaction_with_behavior(TransactionBehavior::Immediate)"
  - "Pattern 3: WAL pragmas + checkpoint run on every open_db() call for connection correctness"

requirements-completed:
  - Binary Architecture constraints
  - Rust Stack exact versions
  - SQLite Configuration constraints

# Metrics
duration: 2min
completed: 2026-02-18
---

# Phase 01 Plan 01: Cargo Workspace and airev-core Foundation Summary

**Three-crate Cargo workspace (airev TUI, airev-mcp MCP, airev-core shared) with WAL-mode SQLite open_db(), Session/Comment/Hunk/DiffLine types, and multi-round thread schema**

## Performance

- **Duration:** 2 min
- **Started:** 2026-02-18T05:58:35Z
- **Completed:** 2026-02-18T06:01:34Z
- **Tasks:** 2
- **Files modified:** 11 created

## Accomplishments
- Cargo workspace with resolver=3, shared workspace.dependencies, and clean dependency isolation (airev has no rmcp; airev-mcp has no ratatui/crossterm)
- airev-core::types exposes Session, Comment, Hunk, DiffLine, DiffLineKind for use across all three crates
- open_db() implements the full locked WAL pragma sequence: journal_mode=WAL, synchronous=NORMAL, foreign_keys=ON, busy_timeout via method (not PRAGMA), wal_checkpoint(TRUNCATE) as maintenance batch, schema DDL in BEGIN IMMEDIATE transaction
- Multi-round thread schema in SQLite from day one: sessions, threads (round_number, status enum), comments all with STRICT and CHECK constraints

## Task Commits

Each task was committed atomically:

1. **Task 1: Cargo workspace root and three-crate skeleton** - `06fe9cf` (chore)
2. **Task 2: airev-core shared types and WAL database initialization** - `ab857a4` (feat)

**Plan metadata:** (docs commit follows)

## Files Created/Modified
- `Cargo.toml` - Workspace root with resolver=3, workspace.dependencies for all shared crates
- `airev/Cargo.toml` - TUI crate: ratatui, crossterm, tokio, signal-hook, futures, airev-core (no rmcp)
- `airev-mcp/Cargo.toml` - MCP crate: tokio, rusqlite, airev-core, rmcp (no ratatui/crossterm)
- `airev-core/Cargo.toml` - Shared crate: rusqlite, tokio-rusqlite, tokio
- `airev/src/main.rs` - Stub binary entry point
- `airev-mcp/src/main.rs` - Stub binary entry point
- `airev-core/src/lib.rs` - Module declarations: pub mod db, schema, types
- `airev-core/src/types.rs` - Session, Comment, Hunk, DiffLine, DiffLineKind structs/enum
- `airev-core/src/schema.rs` - SCHEMA_SQL DDL constant for sessions/threads/comments
- `airev-core/src/db.rs` - open_db() async function with full WAL pragma sequence
- `Cargo.lock` - Generated lockfile

## Decisions Made
- Downgraded rusqlite workspace dependency from 0.38 to 0.37 to resolve version conflict with tokio-rusqlite 0.7 (which pins rusqlite ^0.37). This is a dependency compatibility constraint, not a feature gap.
- Schema DDL applied inside BEGIN IMMEDIATE transaction as specified by locked architectural requirements.
- busy_timeout set via Connection method (not PRAGMA string) per locked architectural requirements.

## Deviations from Plan

### Auto-fixed Issues

**1. [Rule 1 - Bug] Fixed rusqlite version conflict blocking workspace build**
- **Found during:** Task 1 (Cargo workspace root and three-crate skeleton)
- **Issue:** Plan specified `rusqlite = "0.38"` in workspace, but `tokio-rusqlite 0.7` internally requires `rusqlite = "^0.37"`. The `links = "sqlite3"` constraint in Cargo prevents two different versions of `libsqlite3-sys` in the same build graph, making `cargo build --workspace` fail immediately.
- **Fix:** Changed workspace `rusqlite` version from `"0.38"` to `"0.37"` to match tokio-rusqlite 0.7's constraint.
- **Files modified:** Cargo.toml
- **Verification:** `cargo build --workspace` exits 0 after the fix.
- **Committed in:** 06fe9cf (Task 1 commit)

---

**Total deviations:** 1 auto-fixed (1 Rule 1 version conflict bug)
**Impact on plan:** Auto-fix was required for correctness. Using 0.37 vs 0.38 has no behavioral difference for the features used (WAL, transactions, busy_timeout). No scope creep.

## Issues Encountered
None beyond the rusqlite version conflict documented above.

## User Setup Required
None - no external service configuration required. All dependencies are bundled (SQLite via rusqlite bundled feature).

## Next Phase Readiness
- Workspace compiles: `cargo build --workspace` exits 0 with zero errors
- Dependency isolation verified: rmcp absent from airev tree, ratatui absent from airev-mcp tree
- airev-core exports all types and open_db() needed by subsequent plans
- Plan 01-02 (Rendering Skeleton) can proceed using the airev workspace structure
- Plan 01-03 (Git Layer) can proceed using airev-core types
- Plan 01-04 (Persistence Layer) can proceed using open_db() and the schema

---
*Phase: 01-foundation*
*Completed: 2026-02-18*
