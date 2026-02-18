# airev

## What This Is

`airev` is a Rust/ratatui terminal tool for reviewing AI-generated code changes. It gives you a live diff viewer (watching your AI agent write files in real time), standard git diff modes (staged, commit range, branch comparison), and inline commenting with a forced 6-type taxonomy — turning passive code acceptance into an active learning loop. It integrates with Claude Code via an MCP server that works over SSH.

## Core Value

Every AI-generated change gets reviewed and annotated before it's accepted — making the review loop the place where understanding is built, not skipped.

## Requirements

### Validated

(None yet — ship to validate)

### Active

- [ ] Live file watcher: diff view auto-updates as Claude Code writes files to disk
- [ ] Git diff modes: staged changes, commit range (git log --patch), branch comparison (main..HEAD)
- [ ] 3-panel layout: file list | diff view | comments panel — responsive to terminal width
- [ ] Inline commenting on specific lines or hunks with mandatory comment type selection
- [ ] 6-type comment taxonomy: question / concern / til / suggestion / praise / nitpick
- [ ] Severity levels per comment: critical / major / minor / info
- [ ] SQLite persistence: all sessions and comments stored in `.airev/reviews.db`
- [ ] Multi-round review: sessions linked via thread_id, open comments carry forward
- [ ] Vim-native keybindings throughout (j/k navigation, ]c/[c hunk navigation, c to comment)
- [ ] MCP server (stdio transport): Claude Code can query review history and open comments
- [ ] Export to structured markdown for manual paste when needed

### Out of Scope

- Predict-before-reveal mode — deferred to v2 (build foundation first)
- SM-2 spaced repetition engine / quiz mode — deferred to v2
- Neovim plugin — standalone TUI first, plugin later if warranted
- Pre-made Neovim distro — separate project entirely
- Clipboard-based export as primary integration — MCP is the right path (SSH-compatible)
- AI-reviewing-code (Cursor BugBot style) — this tool is for human-reviewing-AI-code

## Context

Primary user is a compiler/systems engineer using Claude Code daily. The learning problem is real: AI tools accelerate output but degrade understanding unless you deliberately structure review. A 2024 cognitive engagement study found that mandatory annotation (requiring reviewers to categorize and explain comments) matched or exceeded writing code manually for retention. The comment taxonomy is the mechanism.

The 3-panel visual style is inspired by gitlogue (Rust/ratatui). The closest functional predecessor is `tuicr` — a Rust/ratatui TUI with inline comments and clipboard export. `airev` extends this with live watching, SQLite persistence, and MCP integration.

The live file watcher is the novel UX: watching Claude Code write changes in real time, with the diff updating as files land, creates a natural review moment before anything is accepted.

## Constraints

- **Tech stack**: Rust + ratatui + crossterm. Non-negotiable — large diff performance and the existing ecosystem (edtui, tree-sitter, ratatui-explorer) are all Rust-native.
- **Data**: SQLite via rusqlite. Inspectable with standard `sqlite3` CLI, scriptable, supports FTS5.
- **Integration**: MCP via stdio transport. Works identically local and over SSH — no clipboard dependency.
- **Keybindings**: Vim conventions throughout. Target user works in Neovim; cognitive overhead of switching bindings is unacceptable.
- **Scope**: Personal tool first. Architecture should support open-sourcing, but don't over-engineer for hypothetical users.

## Key Decisions

| Decision | Rationale | Outcome |
|----------|-----------|---------|
| MCP over clipboard as primary Claude Code integration | Clipboard fails on SSH; MCP stdio works everywhere | — Pending |
| Standalone TUI over Neovim plugin for v1 | Ship the tool, not the integration; lazygit model | — Pending |
| Comment taxonomy as learning mechanism (not predict mode) | Mandatory categorization forces engagement; predict mode is enhancement not foundation | — Pending |
| SQLite over markdown/git-notes for storage | Queryable, inspectable, scriptable; git notes are fragile | — Pending |
| Name: airev | Short, typeable, accurate; works in tmux status bar, CLI help text | — Pending |

---
*Last updated: 2026-02-17 after initialization*
