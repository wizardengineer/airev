---
type: quick
description: "verify phase 4 persistence layer end-to-end: build, schema, migration, review toggle"
---

<tasks>
<task type="auto">
  <name>Task 1: Fix legacy DB migration and add integration tests</name>
  <files>airev-core/src/schema.rs, airev-core/tests/db_lifecycle.rs, airev-core/Cargo.toml</files>
  <action>
  1. Fix migrate() to DROP legacy tables when schema_version is 0 before applying v1 DDL
  2. Write integration tests exercising full DB lifecycle (session create/resume, review toggle, persistence)
  3. Write migration test verifying legacy Phase 1 DBs are handled gracefully
  </action>
  <verify>cargo test -p airev-core --test db_lifecycle passes (2 tests)</verify>
  <done>Migration handles legacy DBs. Integration tests verify schema, session lifecycle, review toggle, and persistence.</done>
</task>
</tasks>
