use std::time::Duration;
use tokio_rusqlite::Connection;
use crate::schema::SCHEMA_SQL;

/// Opens (or creates) the SQLite database at `path`, configures WAL mode,
/// and applies the schema migrations.
///
/// # Errors
/// Returns an error if the file cannot be opened, WAL cannot be enabled,
/// or schema DDL fails.
pub async fn open_db(path: &str) -> Result<Connection, tokio_rusqlite::Error> {
    let conn = Connection::open(path).await?;

    // Step 1: WAL pragmas — connection-level settings re-applied on every open.
    conn.call(|db| {
        db.execute_batch(
            "PRAGMA journal_mode=WAL;
             PRAGMA synchronous=NORMAL;
             PRAGMA foreign_keys=ON;",
        )?;
        // busy_timeout is set via the Connection method, not a PRAGMA string,
        // to ensure it applies regardless of pragma caching behavior.
        db.busy_timeout(Duration::from_secs(10))?;
        Ok(())
    })
    .await?;

    // Step 2: Checkpoint any leftover WAL from a previous run (maintenance,
    // not a data write — plain execute_batch is appropriate here).
    conn.call(|db| {
        db.execute_batch("PRAGMA wal_checkpoint(TRUNCATE);")?;
        Ok(())
    })
    .await?;

    // Step 3: Apply schema DDL inside a BEGIN IMMEDIATE transaction.
    // All write transactions use BEGIN IMMEDIATE per locked requirements.
    conn.call(|db| {
        let tx = db.transaction_with_behavior(
            rusqlite::TransactionBehavior::Immediate,
        )?;
        tx.execute_batch(SCHEMA_SQL)?;
        tx.commit()?;
        Ok(())
    })
    .await?;

    Ok(conn)
}
