---
phase: 03-git-layer
plan: 01
subsystem: git
tags: [git2, crossbeam-channel, syntect, syntect-tui, similar, ratatui, rust]

# Dependency graph
requires:
  - phase: 02-rendering-skeleton
    provides: AppEvent enum, ratatui 0.30 with Line<'static> type, AppState
provides:
  - git2/crossbeam-channel/syntect/syntect-tui/similar workspace dependencies
  - airev/src/git/types.rs with OwnedDiffHunk, OwnedDiffLine, FileSummary, DiffMode, GitRequest, GitResultPayload
  - AppEvent::GitResult(Box<GitResultPayload>) data-carrying variant
  - airev/src/git/ module tree (mod.rs, types.rs, worker.rs stub)
affects: [03-02, 03-03, 03-04, 03-05]

# Tech tracking
tech-stack:
  added:
    - git2 0.20 (libgit2 Rust bindings, dedicated background thread owns Repository)
    - crossbeam-channel 0.5 (MPSC channel for GitRequest from main thread to worker)
    - syntect 5.3 (syntax highlighting, default-fancy feature for pure-Rust fancy-regex, no Oniguruma C dep)
    - syntect-tui 3.0 (converts syntect highlighting to ratatui Span values)
    - similar 2.7 with inline feature (diff algorithm, word-level inline diff support)
  patterns:
    - All types crossing thread boundary are fully owned (no lifetimes) — String, u32, char, Vec, usize
    - Box<GitResultPayload> in AppEvent keeps enum variant small on channel (payload can be large)
    - ratatui::text::Line<'static> via Cow::Owned enables storing highlighted lines in AppState without arena
    - Worker stub pattern: mod.rs declares pub mod worker; with empty stub, full impl in Plan 02

key-files:
  created:
    - airev/src/git/mod.rs
    - airev/src/git/types.rs
    - airev/src/git/worker.rs
  modified:
    - Cargo.toml (workspace.dependencies: five new deps)
    - airev/Cargo.toml (dependencies: five new workspace refs)
    - airev/src/event.rs (GitResult unit variant -> GitResult(Box<GitResultPayload>), Clone removed)
    - airev/src/main.rs (mod git; added)

key-decisions:
  - "Remove Clone derive from AppEvent — GitResultPayload is not Clone (moved into AppState on receipt, never cloned)"
  - "syntect default-fancy feature uses pure-Rust fancy-regex instead of Oniguruma C library — no C build dependency"
  - "#[allow(dead_code)] on GitResultPayload — types used in Plan 02, not yet wired to a call site"
  - "worker.rs stub is an empty file — actual background thread loop implemented in Plan 02"

patterns-established:
  - "Owned-types pattern: all data crossing thread boundary uses String/u32/char/Vec/usize — zero borrowed lifetimes"
  - "Box<Payload> pattern: large event payloads boxed to keep AppEvent enum variants pointer-sized on channel"
  - "Stub module pattern: mod.rs declares submodules, stubs created immediately, full impl in follow-on plan"

requirements-completed: [Diff View, Diff Modes, Git Integration]

# Metrics
duration: 3min
completed: 2026-02-18
---

# Phase 3 Plan 01: Cargo Dependencies and Owned Git Types Summary

**Five new Phase 3 Cargo deps (git2/crossbeam-channel/syntect/syntect-tui/similar) added to workspace and owned diff type hierarchy (OwnedDiffHunk, OwnedDiffLine, FileSummary, DiffMode, GitResultPayload) defined with AppEvent::GitResult upgraded from unit variant to Box<GitResultPayload> carrier.**

## Performance

- **Duration:** 3 min
- **Started:** 2026-02-18T00:05:37Z
- **Completed:** 2026-02-18T00:08:16Z
- **Tasks:** 2
- **Files modified:** 7

## Accomplishments
- Added git2 0.20, crossbeam-channel 0.5, syntect 5.3 (default-fancy/pure-Rust), syntect-tui 3.0, similar 2.7 to workspace and airev crate
- Created airev/src/git/ module with types.rs defining all six owned types: OwnedDiffLine, OwnedDiffHunk, FileSummary, DiffMode, GitRequest, GitResultPayload
- Upgraded AppEvent::GitResult from unit variant to GitResult(Box<GitResultPayload>) with highlighted_lines Vec<ratatui::text::Line<'static>> and hunk_offsets
- cargo build --workspace exits 0 with zero errors; cargo check shows no E0277 Send/Sync violations

## Task Commits

Each task was committed atomically:

1. **Task 1: Add workspace and crate Cargo dependencies** - `78db240` (chore)
2. **Task 2: Create git/types.rs with owned diff structs and AppEvent payload** - `59cdb0b` (feat)

**Plan metadata:** (docs commit — see below)

## Files Created/Modified
- `airev/src/git/mod.rs` - Git module root declaring pub mod types and pub mod worker
- `airev/src/git/types.rs` - All six owned git types with Google-style docstrings
- `airev/src/git/worker.rs` - Empty stub; full background thread loop in Plan 02
- `Cargo.toml` - Five new workspace.dependencies entries
- `airev/Cargo.toml` - Five new workspace-inherited dependency references
- `airev/src/event.rs` - GitResult variant upgraded to carry Box<GitResultPayload>; Clone derive removed
- `airev/src/main.rs` - mod git; declaration added to module tree

## Decisions Made
- Removed `Clone` derive from `AppEvent`: `GitResultPayload` is intentionally not `Clone` (it is moved into `AppState` on receipt). The existing derive was safe to remove — no code cloned `AppEvent` values.
- Used `syntect` with `default-features = false, features = ["default-fancy"]` to pull in pure-Rust `fancy-regex` instead of Oniguruma, eliminating the C build dependency.
- Added `#[allow(dead_code)]` to `GitResultPayload` since the type is defined now but wired up in Plan 02.
- `worker.rs` is a content-free stub so `mod.rs` compiles; the actual `std::thread::spawn` worker loop comes in Plan 02.

## Deviations from Plan

### Auto-fixed Issues

**1. [Rule 1 - Bug] Removed Clone derive from AppEvent to allow non-Clone GitResultPayload**
- **Found during:** Task 2 (updating event.rs)
- **Issue:** AppEvent derived Clone, but the new GitResult(Box<GitResultPayload>) variant requires GitResultPayload to be Clone. The plan specifies GitResultPayload does NOT need Clone; keeping Clone on AppEvent would cause a compile error.
- **Fix:** Removed `Clone` from `#[derive(Debug, Clone)]` on AppEvent. No existing code cloned AppEvent values — the only `.clone()` in main.rs is on `handler.tx` (the channel sender), not on AppEvent.
- **Files modified:** airev/src/event.rs
- **Verification:** cargo build -p airev exits 0 after removal
- **Committed in:** 59cdb0b (Task 2 commit)

---

**Total deviations:** 1 auto-fixed (Rule 1 - compiler error prevented by removing incompatible Clone derive)
**Impact on plan:** Necessary for correctness — plan itself specified GitResultPayload should not implement Clone. No scope creep.

## Issues Encountered
None — workspace resolved all five new dependencies on first `cargo build --workspace` invocation.

## User Setup Required
None - no external service configuration required.

## Next Phase Readiness
- All five Phase 3 deps are locked in Cargo.lock and compile cleanly
- git/types.rs exports all types Plan 02 needs: GitRequest (for the channel), GitResultPayload (for the response), DiffMode (for mode selection)
- AppEvent::GitResult(Box<GitResultPayload>) is the correct receiving variant — Plan 02's worker can construct and send it
- worker.rs stub satisfies the module tree; Plan 02 replaces its content with the full background thread loop

---
*Phase: 03-git-layer*
*Completed: 2026-02-18*
