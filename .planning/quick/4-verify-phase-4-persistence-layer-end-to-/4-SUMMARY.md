---
plan: quick-4
status: complete
tasks_completed: 1
tasks_total: 1
---

## Summary

Fixed a migration bug where legacy Phase 1 databases caused `no such column: diff_mode` errors, and added comprehensive integration tests verifying the full Phase 4 persistence layer.

## What Was Done

### Migration Fix (schema.rs)
- Added `DROP TABLE IF EXISTS` for all 4 tables before `CREATE TABLE` in the version 0→1 migration path
- Only fires when `schema_version` is 0 (no versioned data exists), so safe for production DBs at version 1+
- Fixes the "no such column: diff_mode" error when running against a stale `.airev/reviews.db` from Phase 1

### Integration Tests (tests/db_lifecycle.rs)
Two async integration tests added:

**`full_session_lifecycle`** — exercises the complete DB layer:
- Schema validation: `schema_version = 1`, WAL journal mode, threads table exists, `sessions.id` is TEXT, `file_review_state` has composite PK
- Session create/resume: new session has UUID, same params resumes same session, different diff_mode creates new session
- File review toggle: false→true→false→true cycle, multi-file marking
- Load review state: returns correct file paths and reviewed flags
- Persistence: second connection to same DB sees same review state

**`migration_handles_legacy_db`** — verifies the migration fix:
- Creates a Phase 1-style DB with old schema (INTEGER PK, no diff_mode)
- Opens via `open_db()` which triggers migration
- Verifies new session can be created with v1 schema
- Verifies old data was dropped (only 1 session after create)

### Automated Pre-Checks (all passed)
| Check | Expected | Actual |
|-------|----------|--------|
| `cargo build --workspace` | 0 errors | 0 errors |
| `TransactionBehavior::Immediate` count in db.rs | >= 3 | 4 |
| `toggle_file_reviewed` in keybindings.rs | >= 1 | 2 |
| `reviewed` in file_tree.rs | >= 3 | 6 |
| Help overlay `r` keybinding | present | present |

## Commits
- `1483f00` fix(04): handle legacy Phase 1 DB schema in migration
- `65b229f` test(04): add DB lifecycle integration tests for Phase 4 verification

## Items Requiring Human Verification
The TUI cannot be tested headlessly (needs a real terminal). These items need manual verification:
1. Status bar shows `Session: xxxxxxxx...` on launch
2. `r` key toggles `[x]`/`[ ]` checkmarks in the file list
3. Checkmarks persist after quit + relaunch
4. Two concurrent instances produce no SQLITE_BUSY errors
