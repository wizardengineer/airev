//! Integration test for Phase 4 DB lifecycle.
//!
//! Exercises: open_db, migrate, detect_or_create_session,
//! load_file_review_state, toggle_file_reviewed, update_session_timestamp.

use airev_core::db;

fn temp_db_path() -> String {
    let dir = tempfile::TempDir::new().unwrap();
    let path = dir.keep().join("test.db");
    path.to_string_lossy().to_string()
}

#[tokio::test]
async fn full_session_lifecycle() {
    let path = temp_db_path();
    let conn = db::open_db(&path).await.unwrap();

    // Verify schema_version = 1
    let version: i64 = conn
        .call(|db| {
            Ok::<_, rusqlite::Error>(db.query_row(
                "SELECT MAX(version) FROM schema_version",
                [],
                |r| r.get(0),
            )?)
        })
        .await
        .unwrap();
    assert_eq!(version, 1, "schema_version should be 1");

    // Verify WAL mode
    let journal: String = conn
        .call(|db| {
            Ok::<_, rusqlite::Error>(
                db.query_row("PRAGMA journal_mode", [], |r| r.get(0))?,
            )
        })
        .await
        .unwrap();
    assert_eq!(journal, "wal", "journal_mode should be wal");

    // Verify threads table exists (empty)
    let thread_count: i64 = conn
        .call(|db| {
            Ok::<_, rusqlite::Error>(
                db.query_row("SELECT COUNT(*) FROM threads", [], |r| r.get(0))?,
            )
        })
        .await
        .unwrap();
    assert_eq!(thread_count, 0, "threads table should exist and be empty");

    // Verify sessions table has TEXT primary key
    let session_pk_type: String = conn
        .call(|db| {
            Ok::<_, rusqlite::Error>(db.query_row(
                "SELECT type FROM pragma_table_info('sessions') WHERE name = 'id'",
                [],
                |r| r.get(0),
            )?)
        })
        .await
        .unwrap();
    assert_eq!(session_pk_type, "TEXT", "sessions.id should be TEXT");

    // Verify file_review_state composite PK
    let frs_pk_count: i64 = conn
        .call(|db| {
            Ok::<_, rusqlite::Error>(db.query_row(
                "SELECT COUNT(*) FROM pragma_table_info('file_review_state') WHERE pk > 0",
                [],
                |r| r.get(0),
            )?)
        })
        .await
        .unwrap();
    assert_eq!(frs_pk_count, 2, "file_review_state should have composite PK");

    // Create a session
    let session = db::detect_or_create_session(
        &conn, "/tmp/test-repo", "staged", "",
    )
    .await
    .unwrap();
    assert!(!session.id.is_empty(), "session ID should be non-empty UUID");
    assert_eq!(session.repo_path, "/tmp/test-repo");
    assert_eq!(session.diff_mode, "staged");

    // Resume same session (should return same ID)
    let resumed = db::detect_or_create_session(
        &conn, "/tmp/test-repo", "staged", "",
    )
    .await
    .unwrap();
    assert_eq!(resumed.id, session.id, "should resume same session");

    // Different diff_mode creates new session
    let other = db::detect_or_create_session(
        &conn, "/tmp/test-repo", "head", "",
    )
    .await
    .unwrap();
    assert_ne!(other.id, session.id, "different mode = new session");

    // Session count should be 2
    let count: i64 = conn
        .call(|db| {
            Ok::<_, rusqlite::Error>(
                db.query_row("SELECT COUNT(*) FROM sessions", [], |r| r.get(0))?,
            )
        })
        .await
        .unwrap();
    assert_eq!(count, 2, "should have 2 sessions");

    // Load review state (initially empty)
    let states = db::load_file_review_state(&conn, &session.id)
        .await
        .unwrap();
    assert!(states.is_empty(), "no files reviewed yet");

    // Toggle file reviewed (false -> true)
    let reviewed = db::toggle_file_reviewed(&conn, &session.id, "src/main.rs")
        .await
        .unwrap();
    assert!(reviewed, "first toggle should set reviewed=true");

    // Toggle again (true -> false)
    let reviewed = db::toggle_file_reviewed(&conn, &session.id, "src/main.rs")
        .await
        .unwrap();
    assert!(!reviewed, "second toggle should set reviewed=false");

    // Toggle back to true, then mark another file
    let reviewed = db::toggle_file_reviewed(&conn, &session.id, "src/main.rs")
        .await
        .unwrap();
    assert!(reviewed);
    let reviewed = db::toggle_file_reviewed(&conn, &session.id, "src/lib.rs")
        .await
        .unwrap();
    assert!(reviewed);

    // Load review state - should have 2 files
    let states = db::load_file_review_state(&conn, &session.id)
        .await
        .unwrap();
    assert_eq!(states.len(), 2, "should have 2 reviewed files");
    let main_state = states.iter().find(|(p, _)| p == "src/main.rs").unwrap();
    assert!(main_state.1, "src/main.rs should be reviewed");
    let lib_state = states.iter().find(|(p, _)| p == "src/lib.rs").unwrap();
    assert!(lib_state.1, "src/lib.rs should be reviewed");

    // Update session timestamp
    db::update_session_timestamp(&conn, &session.id).await.unwrap();

    // Verify persistence: open a second connection to same DB
    let conn2 = db::open_db(&path).await.unwrap();
    let states2 = db::load_file_review_state(&conn2, &session.id)
        .await
        .unwrap();
    assert_eq!(
        states2.len(),
        2,
        "review state should persist across connections"
    );
}

#[tokio::test]
async fn migration_handles_legacy_db() {
    let dir = tempfile::TempDir::new().unwrap();
    let path = dir.path().join("legacy.db").to_string_lossy().to_string();

    // Create a legacy Phase 1 DB with old schema (no diff_mode/diff_args)
    {
        let db = rusqlite::Connection::open(&path).unwrap();
        db.execute_batch(
            "CREATE TABLE sessions (
                id INTEGER PRIMARY KEY AUTOINCREMENT,
                repo_path TEXT NOT NULL,
                created_at TEXT NOT NULL
            );
            INSERT INTO sessions (repo_path, created_at) VALUES ('/old', '2024-01-01');",
        )
        .unwrap();
    }

    // open_db should handle the legacy schema gracefully
    let conn = db::open_db(&path).await.unwrap();

    // Should be able to create a session with the new schema
    let session = db::detect_or_create_session(&conn, "/test", "staged", "")
        .await
        .unwrap();
    assert!(!session.id.is_empty());

    // The old data should be gone (dropped during migration)
    let count: i64 = conn
        .call(|db| {
            Ok::<_, rusqlite::Error>(
                db.query_row("SELECT COUNT(*) FROM sessions", [], |r| r.get(0))?,
            )
        })
        .await
        .unwrap();
    assert_eq!(count, 1, "only the new session should exist");
}
