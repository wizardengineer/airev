use std::time::{Duration, SystemTime, UNIX_EPOCH};

use rusqlite::OptionalExtension;
use tokio_rusqlite::Connection;

use crate::types::Session;

/// Opens (or creates) the SQLite database at `path`, configures WAL mode,
/// and applies schema migrations via the `schema_version` table.
///
/// This function is the single entry point for all database connections.
/// It sets `busy_timeout` via the `Connection` method (not a PRAGMA string) to
/// ensure the setting takes effect regardless of pragma caching.
///
/// # Errors
///
/// Returns `tokio_rusqlite::Error` if the file cannot be opened, WAL configuration
/// fails, or schema DDL fails.
pub async fn open_db(path: &str) -> Result<Connection, tokio_rusqlite::Error> {
    let conn = Connection::open(path).await?;

    // Step 1: WAL pragmas — connection-level settings re-applied on every open.
    conn.call(|db| {
        db.execute_batch(
            "PRAGMA journal_mode=WAL;
             PRAGMA synchronous=NORMAL;
             PRAGMA foreign_keys=ON;",
        )?;
        // busy_timeout via Connection method (not PRAGMA string) per locked decision.
        db.busy_timeout(Duration::from_secs(5))?;
        Ok(())
    })
    .await?;

    // Step 2: Checkpoint any leftover WAL from a previous run.
    // Called from the TUI process only — airev-mcp must not call this.
    conn.call(|db| {
        db.execute_batch("PRAGMA wal_checkpoint(TRUNCATE);")?;
        Ok(())
    })
    .await?;

    // Step 3: Apply schema migrations via schema_version versioning system.
    conn.call(|db| {
        crate::schema::migrate(db)?;
        Ok(())
    })
    .await?;

    Ok(conn)
}

/// Returns the current Unix timestamp in seconds.
fn now_secs() -> i64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default()
        .as_secs() as i64
}

/// Finds the most recent session for `repo_path + diff_mode + diff_args`, or creates one.
///
/// On resume: updates `updated_at` to the current time via `BEGIN IMMEDIATE`.
/// On create: generates a new UUID v4, inserts the session via `BEGIN IMMEDIATE`.
///
/// This function must be called before the first event-loop frame so session data
/// is available immediately (no loading spinner required).
///
/// # Errors
///
/// Returns `tokio_rusqlite::Error` if the query or write transaction fails.
pub async fn detect_or_create_session(
    conn: &Connection,
    repo_path: &str,
    diff_mode: &str,
    diff_args: &str,
) -> Result<Session, tokio_rusqlite::Error> {
    let repo_path = repo_path.to_owned();
    let diff_mode = diff_mode.to_owned();
    let diff_args = diff_args.to_owned();

    conn.call(move |db| {
        let existing: Option<Session> = db
            .query_row(
                "SELECT id, repo_path, diff_mode, diff_args, created_at, updated_at
                 FROM sessions
                 WHERE repo_path = ?1 AND diff_mode = ?2 AND diff_args = ?3
                 ORDER BY updated_at DESC
                 LIMIT 1",
                rusqlite::params![&repo_path, &diff_mode, &diff_args],
                |r| {
                    Ok(Session {
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
            let now = now_secs();
            let tx = db.transaction_with_behavior(rusqlite::TransactionBehavior::Immediate)?;
            tx.execute(
                "UPDATE sessions SET updated_at = ?1 WHERE id = ?2",
                rusqlite::params![now, &session.id],
            )?;
            tx.commit()?;
            Ok(session)
        } else {
            let id = uuid::Uuid::new_v4().to_string();
            let now = now_secs();
            let tx = db.transaction_with_behavior(rusqlite::TransactionBehavior::Immediate)?;
            tx.execute(
                "INSERT INTO sessions (id, repo_path, diff_mode, diff_args, created_at, updated_at)
                 VALUES (?1, ?2, ?3, ?4, ?5, ?5)",
                rusqlite::params![&id, &repo_path, &diff_mode, &diff_args, now],
            )?;
            tx.commit()?;
            Ok(Session {
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

/// Loads the reviewed state for all files within `session_id`.
///
/// Returns a `Vec` of `(file_path, reviewed)` pairs. Files with no row in the
/// `file_review_state` table are absent from the result (treated as unreviewed).
///
/// # Errors
///
/// Returns `tokio_rusqlite::Error` if the query fails.
pub async fn load_file_review_state(
    conn: &Connection,
    session_id: &str,
) -> Result<Vec<(String, bool)>, tokio_rusqlite::Error> {
    let session_id = session_id.to_owned();

    conn.call(move |db| {
        let mut stmt = db.prepare(
            "SELECT file_path, reviewed FROM file_review_state WHERE session_id = ?1",
        )?;
        let rows = stmt
            .query_map(rusqlite::params![&session_id], |r| {
                let file_path: String = r.get(0)?;
                let reviewed: bool = r.get(1)?;
                Ok((file_path, reviewed))
            })?
            .collect::<rusqlite::Result<Vec<_>>>()?;
        Ok(rows)
    })
    .await
}

/// Toggles the reviewed flag for `file_path` within `session_id`.
///
/// Uses an upsert (`INSERT ... ON CONFLICT DO UPDATE`) inside `BEGIN IMMEDIATE`.
/// Sets `reviewed_at` to now when transitioning to reviewed; clears it on untoggle.
///
/// Returns the new reviewed state (`true` = reviewed, `false` = unreviewed).
///
/// # Errors
///
/// Returns `tokio_rusqlite::Error` if the upsert transaction fails.
pub async fn toggle_file_reviewed(
    conn: &Connection,
    session_id: &str,
    file_path: &str,
) -> Result<bool, tokio_rusqlite::Error> {
    let session_id = session_id.to_owned();
    let file_path = file_path.to_owned();

    conn.call(move |db| {
        let now = now_secs();

        let current: bool = db
            .query_row(
                "SELECT reviewed FROM file_review_state
                 WHERE session_id = ?1 AND file_path = ?2",
                rusqlite::params![&session_id, &file_path],
                |r| r.get::<_, bool>(0),
            )
            .unwrap_or(false);

        let new_state = !current;
        let reviewed_at: Option<i64> = if new_state { Some(now) } else { None };

        let tx = db.transaction_with_behavior(rusqlite::TransactionBehavior::Immediate)?;
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

/// Updates the `updated_at` timestamp for `session_id` to the current time.
///
/// Called on quit or after significant user actions to keep the session's
/// `updated_at` field current, so `detect_or_create_session` can resume it.
///
/// # Errors
///
/// Returns `tokio_rusqlite::Error` if the `BEGIN IMMEDIATE` transaction fails.
pub async fn update_session_timestamp(
    conn: &Connection,
    session_id: &str,
) -> Result<(), tokio_rusqlite::Error> {
    let session_id = session_id.to_owned();

    conn.call(move |db| {
        let now = now_secs();
        let tx = db.transaction_with_behavior(rusqlite::TransactionBehavior::Immediate)?;
        tx.execute(
            "UPDATE sessions SET updated_at = ?1 WHERE id = ?2",
            rusqlite::params![now, &session_id],
        )?;
        tx.commit()?;
        Ok(())
    })
    .await
}
