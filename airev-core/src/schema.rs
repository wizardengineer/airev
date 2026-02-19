/// DDL to create the schema_version tracking table.
///
/// Applied unconditionally on every DB open (before checking the version),
/// using `IF NOT EXISTS` so it is safe to run multiple times.
pub const SCHEMA_VERSION_DDL: &str = "
    CREATE TABLE IF NOT EXISTS schema_version (
        version INTEGER NOT NULL
    ) STRICT;
";

/// DDL for the full v1 schema.
///
/// Contains four tables:
/// - `sessions`: one row per review session, keyed by UUID v4 text.
/// - `comments`: inline code comments attached to a session and optionally a thread.
/// - `file_review_state`: per-file reviewed flag within a session.
/// - `threads`: multi-round comment threads (written by Phase 7 MCP; schema present from day one).
///
/// All tables use `STRICT` mode for type enforcement.
/// Foreign keys use `ON DELETE CASCADE` so removing a session cleans up all child rows.
pub const SCHEMA_V1_SQL: &str = "
    CREATE TABLE IF NOT EXISTS sessions (
        id          TEXT    PRIMARY KEY,
        repo_path   TEXT    NOT NULL,
        diff_mode   TEXT    NOT NULL,
        diff_args   TEXT    NOT NULL DEFAULT '',
        created_at  INTEGER NOT NULL,
        updated_at  INTEGER NOT NULL
    ) STRICT;

    CREATE TABLE IF NOT EXISTS threads (
        id           TEXT    PRIMARY KEY,
        session_id   TEXT    NOT NULL REFERENCES sessions(id) ON DELETE CASCADE,
        status       TEXT    NOT NULL DEFAULT 'open'
                             CHECK(status IN ('open', 'addressed', 'resolved')),
        round_number INTEGER NOT NULL DEFAULT 1
    ) STRICT;

    CREATE TABLE IF NOT EXISTS comments (
        id           TEXT    PRIMARY KEY,
        session_id   TEXT    NOT NULL REFERENCES sessions(id) ON DELETE CASCADE,
        file_path    TEXT    NOT NULL,
        line_number  INTEGER,
        hunk_offset  INTEGER,
        comment_type TEXT    NOT NULL
                             CHECK(comment_type IN
                                   ('question','concern','til','suggestion','praise','nitpick')),
        severity     TEXT    NOT NULL
                             CHECK(severity IN ('critical','major','minor','info')),
        body         TEXT    NOT NULL,
        created_at   INTEGER NOT NULL,
        resolved_at  INTEGER,
        thread_id    TEXT    REFERENCES threads(id) ON DELETE SET NULL
    ) STRICT;

    CREATE TABLE IF NOT EXISTS file_review_state (
        session_id  TEXT    NOT NULL REFERENCES sessions(id) ON DELETE CASCADE,
        file_path   TEXT    NOT NULL,
        reviewed    INTEGER NOT NULL DEFAULT 0,
        reviewed_at INTEGER,
        PRIMARY KEY (session_id, file_path)
    ) STRICT;
";

/// Runs forward-only schema migration to migrate the DB to the latest version.
///
/// This function is idempotent: safe to call on every startup regardless of
/// whether the schema has already been applied.
///
/// # Process
///
/// 1. Creates the `schema_version` table if it does not exist.
/// 2. Reads the current version (`0` if the table is empty).
/// 3. If the version is below 1, applies `SCHEMA_V1_SQL` inside a
///    `BEGIN IMMEDIATE` transaction and records `version = 1`.
///
/// # Errors
///
/// Returns `rusqlite::Error` if the DDL fails or the version row cannot be read.
pub fn migrate(db: &mut rusqlite::Connection) -> rusqlite::Result<()> {
    db.execute_batch(SCHEMA_VERSION_DDL)?;

    let version: i64 = db
        .query_row(
            "SELECT COALESCE(MAX(version), 0) FROM schema_version",
            [],
            |r| r.get(0),
        )
        .unwrap_or(0);

    if version < 1 {
        let tx = db.transaction_with_behavior(rusqlite::TransactionBehavior::Immediate)?;
        tx.execute_batch(SCHEMA_V1_SQL)?;
        tx.execute("INSERT INTO schema_version (version) VALUES (1)", [])?;
        tx.commit()?;
    }

    Ok(())
}
