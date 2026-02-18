/// SQL DDL for all tables. Applied in open_db() after WAL pragmas.
pub const SCHEMA_SQL: &str = "
    CREATE TABLE IF NOT EXISTS sessions (
        id             INTEGER PRIMARY KEY AUTOINCREMENT,
        repo_path      TEXT    NOT NULL,
        created_at     INTEGER NOT NULL,
        last_opened_at INTEGER NOT NULL
    ) STRICT;

    CREATE TABLE IF NOT EXISTS threads (
        id             INTEGER PRIMARY KEY AUTOINCREMENT,
        session_id     INTEGER NOT NULL REFERENCES sessions(id) ON DELETE CASCADE,
        status         TEXT    NOT NULL DEFAULT 'open'
                               CHECK(status IN ('open', 'addressed', 'resolved')),
        round_number   INTEGER NOT NULL DEFAULT 1
    ) STRICT;

    CREATE TABLE IF NOT EXISTS comments (
        id             INTEGER PRIMARY KEY AUTOINCREMENT,
        session_id     INTEGER NOT NULL REFERENCES sessions(id) ON DELETE CASCADE,
        thread_id      INTEGER NOT NULL REFERENCES threads(id) ON DELETE CASCADE,
        file_path      TEXT    NOT NULL,
        hunk_id        TEXT,
        line_number    INTEGER,
        comment_type   TEXT    NOT NULL
                               CHECK(comment_type IN ('question','concern','til',
                                                      'suggestion','praise','nitpick')),
        severity       TEXT    NOT NULL
                               CHECK(severity IN ('critical','major','minor','info')),
        body           TEXT    NOT NULL,
        created_at     INTEGER NOT NULL,
        resolved_at    INTEGER
    ) STRICT;
";
