# Research Summary: airev

**Project:** airev — Rust/ratatui terminal TUI for reviewing AI-generated code changes
**Domain:** Systems CLI tool — TUI + file watching + git + SQLite + MCP stdio server
**Researched:** 2026-02-17
**Confidence:** HIGH

---

## Executive Summary

airev is a local-first, keyboard-driven code review terminal built in Rust for engineers who
review AI-generated diffs (Claude Code output) daily. The target user is a systems/compiler
engineer with Neovim muscle memory who lives in tmux, and will abandon any tool within minutes
that violates vim conventions or shows startup latency. The research is clear on approach: Rust
workspace with two separate binaries (`airev` TUI + `airev-mcp` MCP server), a Tokio async
event loop driving all I/O through a single unified channel, `git2` on a dedicated background
thread, and a shared SQLite file in WAL mode as the only IPC mechanism between the two processes.
This architecture is neither optional nor clever — it is the only workable design given MCP
stdio's ownership of stdin/stdout and ratatui's ownership of the terminal.

The competitive landscape confirms that airev has clear differentiation: no existing TUI reviewer
offers live file watching, a 3-panel layout with a persistent comments panel, a 6-type/4-severity
comment taxonomy, multi-round thread tracking, or native MCP integration. The closest predecessor
(tuicr) is a 2-panel static viewer with 4-type taxonomy and clipboard-based Claude Code
integration. airev's value proposition is structural, not cosmetic — the multi-round review loop
(human raises concern → agent addresses it → human confirms) is a workflow that no competitor
models at all, and it requires SQLite threading design from day one.

The top risks are all architectural: getting the MCP/TUI process boundary wrong, calling git2 on
the UI thread, omitting WAL mode, and skipping diff virtualization. Every one of these pitfalls has
HIGH recovery cost if discovered late. The mitigation is sequencing: build the terminal scaffolding
with panic hook, the git background thread, and the WAL-configured schema on day one — before
any features are layered on top.

---

## Key Findings

### Recommended Stack

The Rust ecosystem converges tightly here. All versions verified against crates.io on 2026-02-17.
No significant alternatives exist for ratatui (18k+ stars, the only maintained Rust TUI framework),
crossterm (the only cross-platform backend), or `rmcp` (the official MCP Rust SDK, released 0.16
on 2026-02-17). The `git2`/gix choice has one live debate: gix is pre-1.0, lacks write ops, and
is not ready for production — use `git2` and revisit in 2027.

**Core technologies:**
- `ratatui` 0.30 + `crossterm` 0.29: TUI rendering — only viable option; versions must match
- `tokio` 1.49 (full): async runtime — required by crossterm EventStream, notify bridge, tokio-rusqlite, rmcp
- `git2` 0.20: git operations — battle-tested libgit2 bindings; `git2::Repository` is not `Send`, requires dedicated thread
- `notify` 8.2 + `notify-debouncer-full` 0.7: file watching — co-released, always keep versions aligned
- `rusqlite` 0.38 (bundled) + `tokio-rusqlite` 0.7: SQLite persistence — bundled avoids system lib issues
- `rmcp` 0.16: MCP server — official SDK from modelcontextprotocol org; requires `schemars` 1.0 (breaking vs 0.8)
- `similar` 2.7: diff computation — preferred over `diffy` for inline character-level diffs
- `syntect` 5.3 + `syntect-tui` 3.0.6: syntax highlighting — use `default-fancy` to avoid C dep
- `tui-textarea` 0.7: comment text input — more stable than `edtui` for input; `edtui` still viable for vim-modal UX
- `tui-scrollview` 0.6.2: scrollable diff panel — required for diffs that exceed terminal height

**Version compatibility traps:**
- `rmcp` 0.16 requires `schemars` 1.0 — not 0.8. Wrong version silently fails at schema generation.
- `ratatui` 0.30 requires `crossterm` 0.29 exactly.
- `notify` 8.2 and `notify-debouncer-full` 0.7 are co-released — always bump together.
- `git2` should use `features = ["vendored"]` for reproducible builds (embeds libgit2).

### Expected Features

The target audience (Neovim users, compiler engineers) will bounce within minutes if vim keybindings
feel wrong or startup is over 100ms. These are not preferences — they are hard constraints on the
product feeling "native." The 3-panel layout, 6-type taxonomy, and multi-round threads are the
structural differentiators that no competitor ships.

**Must have for v1 (table stakes + differentiators that define identity):**
- Vim-native keybindings (hjkl, g/G, Ctrl-d/u, {/}, [/], q, Esc) — non-negotiable, first keypress test
- Unified diff view with syntax highlighting — 2026 baseline expectation
- 3-panel layout (file list | diff | comments) — the unique layout; do NOT start with 2-panel
- 6-type comment taxonomy (question/concern/til/suggestion/praise/nitpick) + 4-severity levels — airev's identity
- SQLite persistence with session restore — users won't trust a tool that loses their work
- Mark file as reviewed with visual progress indicator — multi-file review completion tracking
- Structured Markdown export (`:clip` / `y`) with severity/type/thread context — closes feedback loop to Claude Code before MCP exists
- Git diff modes: staged, unstaged, commit range, branch comparison — matches critique/tuicr baseline
- `.aireignore` support — AI changesets contain lock files and generated code noise
- Color themes (catppuccin-mocha, gruvbox-dark, dark, light) — expected by target audience
- `?` help overlay — discoverability for new users

**Should have for v1.x (differentiators, add once core workflow is validated daily):**
- Live file watcher with 200ms debounce — the #1 differentiator; technically complex; validate static mode first
- MCP server via stdio — requires comment system to be stable; enables agentic workflow
- Multi-round thread tracking — requires real usage patterns to design correctly
- Vim command line (`:` mode) — power-user feature after keybindings stabilize
- Jujutsu VCS support — growing audience, low risk once git path is solid

**Defer to v2+:**
- Tree-sitter syntax highlighting — syntect is sufficient; tree-sitter adds complexity for incremental perf
- Multi-commit review mode — useful but not the core AI-review workflow
- Custom keybinding TOML config — design the TOML schema early; expose all bindings later

**Hard anti-features (do not build):**
- GitHub/GitLab PR integration — scope conflation; use Markdown export
- Mouse support — target audience disables mouse; breaks tmux scrolling
- Inline AI review suggestions — muddles human/AI responsibility
- Plugin system — premature; TOML config covers 90% of customization

### Architecture Approach

The architecture is a Tokio-based event loop with a single `tokio::mpsc::UnboundedChannel<AppEvent>`
as the sole queue for all inputs: keyboard events (crossterm), file changes (notify bridge), git
results (spawn_blocking), DB results (tokio-rusqlite), and timer ticks. App state lives in the main
loop with single ownership — no `Arc<Mutex<AppState>>` anywhere on the critical path. All blocking
I/O (git2, rusqlite) is offloaded to background threads and results are delivered back as channel
messages. The MCP server is a completely separate binary (`airev-mcp`) that shares state via SQLite
WAL mode only — there is no other IPC.

**Major components:**
1. `airev-core` crate — shared `Session`, `Comment`, `Hunk` types and DB schema migrations
2. `event.rs` — `AppEvent` enum + unified channel; single point of truth for all async boundaries
3. `tui.rs` — terminal lifecycle with `Drop` + panic hook via `ratatui::init()`; raw mode is never left active
4. `git/` module — `AsyncGit` facade; dedicated `std::thread::spawn` thread owns `Repository`; git2 never crosses thread boundary
5. `db/` module — `tokio-rusqlite` with WAL + `BEGIN IMMEDIATE`; single connection instance
6. `ui/` module — pure render functions that take `&AppState` and return nothing; no I/O
7. `watch/` module — `notify-debouncer-full` watcher + `blocking_send` bridge to async channel
8. `airev-mcp` binary — `rmcp` stdio server; `rusqlite` sync (no async needed); reads/writes shared `.db` file

**Key pattern decisions (all are mandatory, not optional):**
- Render separate tick and render events at independent rates (official ratatui pattern)
- All git2 calls via dedicated `std::thread::spawn` (NOT `spawn_blocking` — `Repository` is not `Send`)
- All state mutation only in main loop, never in background tasks
- Single `terminal.draw()` call per loop iteration — all overlays/modals inside one closure

### Critical Pitfalls

All 5 critical pitfalls have HIGH recovery cost if discovered after Phase 1. They must be addressed
before any feature work begins.

1. **Raw mode left active on crash** — Use `ratatui::init()` (not `Terminal::new()`); it installs the panic hook automatically. Also register `signal-hook` for SIGTERM. Verification: intentionally panic the app and confirm the terminal recovers.

2. **MCP stdio conflicts with TUI terminal** — Two-binary architecture is mandatory. ARCHITECTURE and PITFALLS research agree: there is no viable single-process solution. The MCP server must be a separate binary. If for any reason TUI and MCP must coexist in one process, render TUI to stderr via `CrosstermBackend::new(std::io::stderr())` + `BufWriter` — this is an edge case, not the default architecture.

3. **git2 blocking the UI thread** — `git2::Repository` is not `Send`; you cannot move it into `spawn_blocking`. Solution: dedicate a `std::thread::spawn` thread to all git operations; communicate via `crossbeam-channel` or `std::sync::mpsc`; return owned `Vec<DiffHunk>` structs (not `git2::Diff` which borrows the repo).

4. **SQLite concurrency / SQLITE_BUSY deadlock** — Enable WAL mode + `busy_timeout` on every connection on open. Use `BEGIN IMMEDIATE` (not `BEGIN DEFERRED`) for all write transactions. Set `PRAGMA wal_checkpoint(TRUNCATE)` on startup. The `DEFERRED` transaction upgrade failure is NOT fixed by `busy_timeout` — this is the non-obvious SQLite pitfall that causes silent data loss.

5. **notify missing rename-based writes on Linux** — Claude Code writes files via atomic rename (temp file → target). On Linux, `notify` emits `Create` not `Modify` for rename-based writes. Use `notify-debouncer-full` with `EventKind::Any`, and after debounce verify actual changes via `git status` rather than trusting event types.

---

## Implications for Roadmap

All 4 research files agree on build order. The architecture's phase dependencies are hard: nothing
renders without the event loop; no review features work without a rendered UI; no MCP features work
without a stable DB schema. Deviating from this order causes rework.

### Phase 1: Foundation (Terminal + Event Loop + DB Schema)

**Rationale:** Every other component depends on the event loop existing. Terminal lifecycle, panic
hooks, and WAL mode are all zero-cost to get right from day one and HIGH cost to retrofit.
**Delivers:** A running TUI shell (blank panels, no content) with full panic/signal safety and a
healthy database. All architectural decisions locked.
**Addresses:** `airev-core` shared types, `tui.rs` lifecycle, `event.rs` unified channel, placeholder `AppState`
**Avoids:** Pitfall 1 (raw mode), Pitfall 2 (MCP/TUI conflict — architecture decided here), Pitfall 4 (WAL mode), Pitfall 6 (single draw() call), Pitfall 9 (resize handling)
**Research flag:** Standard patterns — no phase research needed. Official ratatui docs cover this completely.

### Phase 2: Rendering Skeleton (3-Panel Layout + Navigation)

**Rationale:** All feature work requires a visible UI. The 3-panel layout is a structural decision
that conflicts architecturally with 2-panel — pick it now and never revisit.
**Delivers:** Responsive 3-panel layout (file list | diff | comments) with vim navigation, `?` help
overlay, mode indicator in status bar, resize-safe layout.
**Addresses:** 3-panel layout, vim-native keybindings (normal-mode nav subset only — NOT a full vim editor)
**Avoids:** Pitfall 7 (vim keybinding complexity — use `tui-textarea`/`edtui` for edit widgets, hand-roll only bounded navigation keys)
**Research flag:** Standard patterns — ratatui layout docs cover this.

### Phase 3: Git Layer (Diff Reading + Hunk Navigation)

**Rationale:** The diff view is the core content of the tool. This phase establishes the async git
boundary that all subsequent git features (file watching, staged/branch modes) depend on.
**Delivers:** Live diff rendering with syntax highlighting, hunk navigation, file list with change summary, all diff modes (staged/unstaged/commit range/branch)
**Uses:** `git2` 0.20 (dedicated thread), `similar` 2.7 (inline diffs), `syntect` 5.3 + `syntect-tui` 3.0.6, `tui-scrollview` 0.6.2
**Avoids:** Pitfall 3 (git blocking — dedicated thread from the start), Pitfall 8 (diff virtualization — build with visible-slice rendering from day one, test with LLVM-scale diff before declaring phase complete)
**Research flag:** `git2` thread-safety pattern (non-Send Repository) needs careful implementation. The asyncgit/gitui reference implementation is the canonical model.

### Phase 4: Persistence Layer (SQLite + Session Management)

**Rationale:** The comment system (airev's identity feature) cannot exist without stable persistence.
The DB schema must be designed for threads/rounds from the start — retrofitting a flat comment table
into a threaded model is a major migration.
**Delivers:** Session create/resume, comment CRUD with 6-type taxonomy and 4-severity levels, mark-as-reviewed state, SQLite migrations via `airev-core`
**Uses:** `rusqlite` 0.38 (bundled), `tokio-rusqlite` 0.7
**Avoids:** Pitfall 4 (WAL + BEGIN IMMEDIATE already wired from Phase 1; this phase adds write volume)
**Research flag:** Schema design for multi-round threads. The schema must accommodate `comment → response → resolution_status` from day one. This is a small design decision with large migration cost if wrong.

### Phase 5: Comment UI (3-Panel Integration + Markdown Export)

**Rationale:** The comment system has its data layer (Phase 4) and its rendering surface (Phase 2).
This phase wires them together with the inline editor and export mechanism — completing the core review loop.
**Delivers:** Inline comment entry via `tui-textarea` (vim keybindings), comment panel with type/severity badges, structured Markdown export (`:clip`, `y`), filter by severity/type
**Uses:** `tui-textarea` 0.7, OSC 52 for SSH-safe clipboard
**Avoids:** Pitfall 10 (SSH clipboard — OSC 52 is mandatory here, not optional)
**Research flag:** Standard patterns — no additional research needed.

### Phase 6: Live File Watcher

**Rationale:** The #1 differentiator. Depends on the git layer (Phase 3) to re-compute diffs.
Staged here after the core review loop is proven in static mode.
**Delivers:** Real-time diff refresh on file save (200ms debounce), `.aireignore` support, cross-platform detection (FSEvents + inotify)
**Uses:** `notify-debouncer-full` 0.7, `notify` 8.2
**Avoids:** Pitfall 5 (rename-based writes on Linux — listen for `EventKind::Any`, verify via git status)
**Research flag:** Cross-platform testing required. Must be explicitly verified on Linux (inotify) before declaring done — macOS-only testing will miss the rename event type difference.

### Phase 7: MCP Server

**Rationale:** Depends on stable DB schema (Phase 4) and a battle-tested comment system (Phase 5).
The MCP server's tool surface should mirror exactly what the UI can do — no new data model concepts.
**Delivers:** `airev-mcp` binary with tools: `list_comments`, `add_annotation`, `get_session`, `list_sessions`, `get_hunk_context`, `resolve_comment`
**Uses:** `rmcp` 0.16 (official SDK), `rusqlite` 0.38 sync (MCP server does not need async), WAL mode shared DB
**Avoids:** Pitfall 2 (separate binary — already settled in Phase 1), Pitfall 4 (WAL + BEGIN IMMEDIATE already in schema)
**Research flag:** `rmcp` 0.16 is newly released (2026-02-17). Test the `stdio()` transport and `#[tool]` proc macro against a real Claude Code configuration before declaring done. The SDK is official but young.

### Phase 8: Polish + Compatibility

**Rationale:** The "looks done but isn't" checklist from PITFALLS.md belongs here — these are not
features, they are correctness requirements that have been deferred by design.
**Delivers:** SSH clipboard via OSC 52, color theme system (catppuccin-mocha, gruvbox-dark, dark, light), `$COLORTERM` detection for 256-color fallback, SIGTERM handler, jj VCS support, vim command line (`:` mode)
**Avoids:** Pitfall 10 (SSH), Pitfall 9 (resize — verify again with the full layout)
**Research flag:** No research needed. Verification against the PITFALLS.md checklist is the acceptance criterion.

### Phase Ordering Rationale

- **Phases 1-2 before everything:** The event loop and rendered shell are the substrate. Nothing
  else can be developed or tested without them. The architectural decision about MCP/TUI process
  separation must be made in Phase 1, not discovered during Phase 7.
- **Phase 3 before Phase 4:** Diff content comes before the ability to annotate it. Reviewing
  without annotations is a coherent workflow; annotating without a diff view is not.
- **Phase 4 before Phase 5:** The comment UI requires the data model to exist. More importantly,
  the thread/round schema must be designed before Phase 5 builds UI assumptions on top of it.
- **Phase 6 after Phase 3:** File watching depends on the git layer to recompute diffs. Wiring
  it before the git layer is implemented produces a watcher with nothing to trigger.
- **Phase 7 after Phase 4+5:** MCP exposes the review state — that state must be stable and used
  in production before an external API is built around it.
- **Phase 8 last:** Compatibility and polish work has no blocking dependencies. Deferring it
  keeps early phases moving fast.

---

## Top 5 Risks with Mitigations

| Risk | Probability | Impact | Mitigation |
|------|-------------|--------|------------|
| git2 not-Send forces architectural rework | HIGH (known compiler error) | HIGH | Design git thread as `std::thread::spawn` from Phase 3 day one; never attempt `spawn_blocking` with `Repository` |
| MCP/TUI process boundary violated | MEDIUM (easy to prototype wrong) | HIGH | Architecture decision locked in Phase 1; two-binary design is mandatory, not a "we can refactor later" choice |
| SQLite DEFERRED transaction upgrade → silent data loss | MEDIUM (non-obvious pitfall) | HIGH | `BEGIN IMMEDIATE` on all writes, WAL mode, `wal_checkpoint(TRUNCATE)` — all configured in Phase 1 schema init |
| notify missing Claude Code's rename-based writes on Linux | HIGH (tests pass on macOS) | MEDIUM | Use `EventKind::Any` + debounce; verify on Linux with Docker before Phase 6 is marked complete |
| rmcp 0.16 API instability | LOW-MEDIUM (official but newly released) | MEDIUM | Prototype the stdio transport and `#[tool]` macro early in Phase 7; pin the version; do not upgrade during active development |

---

## Confidence Assessment

| Area | Confidence | Notes |
|------|------------|-------|
| Stack | HIGH | All versions verified against crates.io API 2026-02-17; primary sources are official docs and official repos |
| Features | MEDIUM-HIGH | Core table stakes from multiple converging competitor analyses; differentiators are well-reasoned but require user validation |
| Architecture | HIGH | Primary sources: official ratatui docs, asyncgit/gitui reference implementation, official MCP spec; all patterns verified |
| Pitfalls | HIGH | Critical pitfalls verified against official GitHub issues (git2-rs #194, libgit2 #3920), official ratatui docs, official SQLite pragma docs |

**Overall confidence:** HIGH

### Gaps to Address

- **Comment taxonomy UX validation:** The 6-type taxonomy (question/concern/til/suggestion/praise/nitpick) is well-reasoned from industry research but has not been validated with the actual target user. Treat the taxonomy as provisional through Phase 5; expose configuration before freezing it.
- **Multi-round thread schema design:** The feature is clear; the correct SQLite schema for `comment → response → resolution_status` cycles needs explicit design work before Phase 4 begins. This is a 30-minute design session, not a research gap — but it must happen before schema migration code is written.
- **rmcp stdio transport in practice:** `rmcp` 0.16 is brand new. The official docs show the stdio transport works; actual Claude Code integration at the configured `mcpServers` entry level needs a smoke test in Phase 7. Prototype it before committing the full MCP tool surface.
- **tui-textarea vs edtui decision:** STACK.md recommends `tui-textarea` for stability; ARCHITECTURE.md references `edtui`. The difference matters for vim-modal comment editing. Decision: use `tui-textarea` for Phase 5; evaluate `edtui` upgrade in Phase 8 if users demand full vim-modal comment entry (ci", dap, etc.).

---

## Sources

### Primary (HIGH confidence)
- ratatui.rs official docs — async event loop, rendering model, panic hooks, layout system
- github.com/ratatui/async-template — official async template confirming tokio::mpsc pattern
- docs.rs/rmcp 0.16 — stdio transport API, confirmed official SDK from modelcontextprotocol org
- github.com/gitui-org/gitui (asyncgit README) — git background thread architecture reference
- libgit2/libgit2 issue #3920 — git2 diff performance vs native git
- git2-rs issue #194 — Repository: not Send (compiler constraint confirmed)
- sqlite.org/pragma.html — WAL mode, BEGIN IMMEDIATE, busy_timeout behavior
- modelcontextprotocol.io/specification/2025-11-25 — MCP stdio transport constraints
- github.com/agavra/tuicr — direct predecessor, feature baseline (HIGH — official repo)
- github.com/sindrets/diffview.nvim — layout patterns (HIGH — official repo)
- BitsAI-CR arxiv paper (2501.15134) — comment taxonomy research

### Secondary (MEDIUM confidence)
- github.com/remorses/critique — diff mode baseline, live watch pattern
- github.com/conikeec/mcp-probe — TUI + MCP coexistence reference implementation
- keliris.dev/articles/improving-spotify-tui — Mutex-on-AppState anti-pattern (widely cited)
- github.com/rhysd/tui-textarea — vim keybinding support
- github.com/joshka/tui-scrollview — scrollable diff panel widget
- parcel/watcher issue #171 — FSEvents vs inotify event type differences
- tenthousandmeters.com blog — SQLite concurrent writes and SQLITE_BUSY

---

*Research completed: 2026-02-17*
*Ready for roadmap: yes*
