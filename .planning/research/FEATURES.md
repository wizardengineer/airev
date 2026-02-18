# Feature Research

**Domain:** Terminal/TUI code review tool for AI-generated changes
**Researched:** 2026-02-17
**Confidence:** MEDIUM-HIGH (core table stakes from multiple converging sources; differentiators informed by direct competitor analysis and ecosystem patterns)

---

## Context & Framing

airev occupies a specific niche: reviewing changes produced by AI coding agents (Claude Code, OpenCode, Codex) from inside the terminal, with a Neovim-native audience. This shapes which features are table stakes vs. differentiating vs. harmful.

The closest predecessor is tuicr (Rust/ratatui). The gold-standard Neovim diff viewer is diffview.nvim. The target user lives in tmux, has deep vim muscle memory, and expects tools to feel like extensions of their editor — not new GUIs to learn.

---

## Feature Landscape

### Table Stakes (Users Expect These)

Features users assume exist. Missing these = product feels incomplete or broken. Neovim users will leave within minutes if these are wrong.

| Feature | Why Expected | Complexity | Notes |
|---------|--------------|------------|-------|
| Vim-native keybindings (hjkl nav, g/G, Ctrl-d/u, {/}, [/]) | Every TUI targeting Neovim users ships this. tuicr, lazygit, diffview.nvim, gitui all do it. Violations break muscle memory immediately. | LOW | Must feel native from first keypress. `j/k` scroll, `h/l` panel switch, `g/G` jump to top/bottom, `{/}` file jump, `[/]` hunk nav. |
| Unified diff view with syntax highlighting | Standard since delta/bat. Users have trained eyes on color-coded +/- lines. Monochrome diffs feel broken in 2026. | MEDIUM | Tree-sitter (gitlogue pattern) preferred over regex-based; incremental parsing handles large files without stutter. |
| File list panel with change summary | diffview.nvim, tuicr, git-local-review all have it. Users need quick orientation: how many files changed, which ones need attention. | LOW | Show modified/added/deleted counts per file. Allow jumping directly to file from list. |
| Diff scope switching (staged / unstaged / commit range / branch) | critique, tuicr, git-local-review all support multiple diff modes. Users routinely need to switch between reviewing staged changes and uncommitted work. | MEDIUM | `critique` supports: unstaged, --staged, HEAD~1, specific commit, branch comparison. airev should match this baseline. |
| Expandable context between hunks | tuicr's "… expand (N lines) …" is directly expected. Users reviewing AI changes need to see surrounding code for correctness judgments. | LOW | Lazy-expand on Enter. Don't show full file by default — preserve scroll performance. |
| Session persistence (review survives restart) | tuicr auto-saves to XDG path. git-local-review persists state. Users lose trust immediately if closing the tool loses their in-progress review. | MEDIUM | SQLite is appropriate here (vs. tuicr's flat file). Enables richer queries (filter by severity, file, etc.) |
| `?` help overlay | lazygit pattern — every major TUI ships contextual help. Without it, tool is unlearnable without external docs. | LOW | Show available keys for current context. Modal, dismissible with Esc or `?`. |
| Esc / `q` quit with confirmation on unsaved state | Standard modal TUI convention. Users expect `q` to quit (vi-style) and get warned if work is unsaved. | LOW | Confirmation only when there's uncommitted review work. No confirmation on clean state. |
| File search / jump (fuzzy or `/` filter) | critique ships Ctrl+P fuzzy file selector. tuicr has `/` search. Required when reviewing large AI-generated changesets with 20+ files. | MEDIUM | `/` in file list for filter-as-you-type. No need for external fzf dependency — implement inline. |
| Hunk-level navigation | diffview.nvim `[c`/`]c`, tuicr `[/]`. Reviewing at hunk granularity is the unit of attention for code review. | LOW | Standard vim diff motions. `[h`/`]h` or reuse `[/]`. |
| Mark file as reviewed | tuicr, git-local-review both have this. Critical for multi-file reviews — users need to track progress. | LOW | Toggle per file. Visual indicator in file list (checkmark, color). Persisted in SQLite. |
| Color theme support (dark/light + popular themes) | tuicr ships catppuccin + gruvbox. Users have terminal-wide color schemes they expect tools to respect. Clashing colors cause eyestrain and feel amateurish. | LOW | Support at minimum: dark, light, catppuccin-mocha, gruvbox-dark. TOML config at XDG path. |

---

### Differentiators (Competitive Advantage)

Features that set airev apart. These are not assumed, but create loyalty and word-of-mouth among the target audience.

| Feature | Value Proposition | Complexity | Notes |
|---------|-------------------|------------|-------|
| Live file watcher (inotify/FSEvents) with debounced diff refresh | No competitor does this in a native TUI. tuicr and diffview.nvim are static — you re-run them after changes. airev can watch Claude Code write files in real time, updating the diff panel automatically. This is the #1 differentiator. | HIGH | Debounce at 200ms to prevent flicker (diffwatch pattern). Watch project root recursively. Automatically ignore .git, node_modules, __pycache__ etc. Cross-platform: inotify (Linux), FSEvents (macOS), ReadDirectoryChangesW (Windows). |
| 6-type comment taxonomy (question/concern/til/suggestion/praise/nitpick) | tuicr has 4 types (ISSUE/SUGGESTION/NOTE/PRAISE). review.nvim has 4 (Note/Suggestion/Issue/Praise). airev's 6-type taxonomy with `til` (today-I-learned) and `nitpick` is more expressive and matches how senior engineers actually think during review. `nitpick` is particularly important: it signals "fix this but don't block on it," a crucial social signal in review culture. | LOW | Tab-cycles through types in comment mode. Displayed as colored badges in the comments panel. |
| Severity levels per comment (critical/major/minor/info) | Research confirms this is how AI-code-review tools at scale structure findings (Microsoft, CodeRabbit, BitsAI-CR). No terminal TUI competitor offers this. Enables filtering: "show me only critical comments." | LOW | Orthogonal to type (a `question` can be `critical`). Rendered as colored severity prefix in comments panel. |
| MCP server via stdio for Claude Code integration | tuicr uses a Claude Code skill that opens it in a tmux pane and exports via clipboard. airev's MCP integration enables direct programmatic access — Claude Code can read comments, mark items resolved, post responses — without any clipboard intermediary. This is a structural advantage for agentic workflows. | HIGH | MCP over stdio is the current standard for local integrations (per MCP spec 2025-11-25). Expose: list_comments, add_comment, resolve_comment, get_review_status, list_files. |
| Multi-round review with thread tracking | tuicr exports a flat list of comments to clipboard. There is no concept of a thread or round. airev tracks which comments Claude Code responded to, whether the response resolved the concern, and surfaces unresolved threads. This is the core loop of AI-code-review: human raises concern → agent addresses it → human confirms or escalates. | HIGH | Requires SQLite thread model: comment → response → resolution_status. UI shows thread status in comments panel (open/addressed/resolved). |
| 3-panel layout (file list | diff | comments) | tuicr has a 2-panel layout with a toggle. diffview.nvim has 2-panel diff + file list. The 3-panel layout with a persistent comments panel is novel in TUI space and matches the mental model of code review (you read the diff and write a comment side-by-side). | MEDIUM | Panel resizing with `<` / `>`. Collapsible panels for narrow terminals (tmux). Focus moves: `H`/`L` or `Ctrl-h`/`Ctrl-l`. |
| Structured Markdown export optimized for LLM consumption | tuicr exports Markdown to clipboard (good). airev should go further: export includes severity, type, thread status, line number, file context snippet. Format is designed to be pasted into Claude with enough context for one-pass resolution. | LOW | `y` yanks current comment. `:clip` exports full review. Format: file/line/severity/type/comment/thread-history. |
| `.aireignore` support (gitignore-style) | tuicr has `.tuicrignore`. airev should have the same. AI-generated changes often touch lock files, generated files, test fixtures — users need surgical control over what appears in the review diff. | LOW | Gitignore pattern matching. Exclude common noise by default (package-lock.json, *.lock, dist/, .next/). |
| Jujutsu (jj) VCS support | tuicr explicitly supports jj. A growing segment of power users (especially those adopting AI-heavy workflows) use jj. tuicr's detection order: Jujutsu → Git → Mercurial. airev should match this. | MEDIUM | jj repos are git-backed, so most git operations work. Needs custom revset handling for `-r` flag. |
| Vim-mode command line (`:` colon commands) | tuicr has `:w`, `:clip`, `:q`, `:diff`. lazygit has none. This is a powerful affordance for power users who want scriptable actions without reaching for the mouse or memorizing single-key shortcuts. | LOW | `:w` save, `:q` quit, `:clip` export, `:filter <severity>` filter comments, `:round <n>` jump to review round. |

---

### Anti-Features (Commonly Requested, Often Problematic)

Features that seem like good ideas but will harm the product. Resist the urge to build these.

| Anti-Feature | Why Requested | Why Problematic | Alternative |
|--------------|---------------|-----------------|-------------|
| GitHub/GitLab PR integration (remote review) | "I want to post my comments to the PR" | airev is a local, offline, terminal-native tool for AI-generated changes in flight. GitHub integration requires auth, network I/O, API rate limits, and diverges from the core use case. It becomes a worse octo.nvim. The target workflow is Claude Code → airev → feedback to Claude Code, not GitHub. | Export structured Markdown that can be pasted into a GitHub comment manually. Ship this in v2+ only if real demand exists. |
| Mouse support | TUI tools often add mouse because "users ask for it" | The target audience (Neovim users) actively disables mouse. Mouse support adds rendering complexity, breaks tmux scrolling behavior, and signals "this tool is for beginners." tuicr does not ship mouse support for this reason. | Perfect keyboard navigation. Make every action reachable with 2-3 keystrokes from any panel. |
| Inline AI review suggestions (auto-generate comments via LLM) | "Add AI review on top of my review" | This would make airev an AI tool reviewing AI code. The value of airev is the *human* is the reviewer. Generating AI suggestions muddles responsibility and adds latency. Tools that do this (Roborev, CodeRabbit) target a different workflow. | Keep airev as the human-in-the-loop surface. Claude Code does the AI analysis. airev captures the human's judgment. |
| Web preview / shareable URL | critique ships `critique web` for shareable HTML | airev is terminal-native and local. Web previews require a backend, a hosting service, auth, and maintenance. critique's web preview works because it's built on a web runtime (Bun/OpenTUI). For a Rust/ratatui tool this would be a major architectural divergence. | Structured Markdown export is the shareable artifact. Paste it into a PR comment, Slack, or another tool. |
| Real-time collaborative review (multiplayer) | "Two people reviewing the same diff simultaneously" | This requires synchronization infrastructure (CRDTs or operational transforms), conflicts with the local-first SQLite model, and solves a problem the target user (solo AI-code reviewer) does not have. | Multi-round threads handle the async "I reviewed, Claude responded, I re-review" pattern. That's the actual use case. |
| PDF export | critique ships `--pdf` for Kindle/Boox | This is a novelty feature. No terminal developer reviewing AI-generated code needs a PDF. It adds a dependency (headless browser or LaTeX) for zero real value to the target audience. | Markdown export. If someone really wants PDF, they can pipe the Markdown through pandoc. |
| Notification system / desktop alerts | "Notify me when a watched file changes" | OS-level notification integration (libnotify, osascript) adds platform-specific complexity and breaks SSH/headless workflows. Users running airev in a tmux session on a remote server cannot receive desktop notifications. | The diff panel updates visually on file change. That is sufficient. TUI tools are interactive — the user is present. |
| Built-in Git operations (stage, commit, push) | lazygit users might expect this | airev is a review tool, not a git client. Scope conflation leads to mediocre implementations of both. lazygit already exists and is excellent. | `:!git <command>` passthrough for power users who need it. Keep the tool focused. |
| Plugin system / extensibility API | "Let users extend it" | Premature generalization. Building a plugin API before knowing what the core tool should do well leads to bad architecture and maintenance burden. tuicr, diffview.nvim don't have plugin APIs. | Configuration via TOML covers 90% of customization needs. Add extensibility when a concrete need is identified. |

---

## Feature Dependencies

```
Live File Watcher
    └──enables──> Real-time diff refresh
                      └──requires──> Debounced event coalescing (200ms)
                      └──requires──> File filter (.aireignore)

Comment System (type + severity)
    └──requires──> SQLite persistence
                      └──enables──> Session restore
                      └──enables──> Multi-round thread tracking
                      └──enables──> Filter by severity/type

Multi-round Thread Tracking
    └──requires──> Comment System
    └──requires──> SQLite persistence
    └──enables──> MCP server (list_comments, resolve_comment)

MCP Server (stdio)
    └──requires──> Comment System
    └──requires──> SQLite persistence
    └──enhances──> Multi-round Thread Tracking

3-Panel Layout
    └──requires──> File List Panel
    └──requires──> Diff View Panel
    └──requires──> Comments Panel
    └──enables──> Side-by-side review workflow

Structured Markdown Export
    └──requires──> Comment System
    └──enhances──> Multi-round Thread Tracking (includes thread history)

Syntax Highlighting (tree-sitter)
    └──enhances──> Diff View Panel
    └──no hard dependency on anything else (can ship unified diff without it)

Vim command line (: mode)
    └──enhances──> Session persistence (`:w`)
    └──enhances──> Export (`:clip`)
    └──conflicts──> Mouse support (modal input state)

.aireignore
    └──enhances──> Live File Watcher (reduces noise)
    └──enhances──> File List Panel (fewer irrelevant entries)
```

### Dependency Notes

- **MCP server requires Comment System + SQLite:** MCP exposes review state programmatically. Without a structured persistence layer, there is nothing to expose. Ship comment system first, MCP second.
- **Live file watcher enhances but does not require diff view:** You can ship the diff view as a static git-diff reader first, then add the watcher as a layer. This is the correct build order.
- **Multi-round threads require a round/conversation model in SQLite:** A flat comment table is insufficient. Design the schema for threads from the start — retrofitting is painful.
- **3-panel layout and 2-panel layout conflict architecturally:** Pick one layout model for v1. The 3-panel layout is the differentiator. Do not ship a 2-panel mode that you later try to extend — rebuild cost is high.
- **Vim command line conflicts with search (`/`):** Both use a text-entry mode. Ensure the input state machine clearly distinguishes `:` (command) from `/` (search) and `i` (insert comment). This is a common TUI bug.

---

## MVP Definition

### Launch With (v1)

Minimum viable product — enough to replace tuicr for a Claude Code user.

- [ ] Vim-native keybindings (hjkl, g/G, Ctrl-d/u, {/}, [/], q, Esc) — foundation of usability
- [ ] Unified diff view with syntax highlighting — table stakes for any diff tool in 2026
- [ ] 3-panel layout (file list | diff | comments) — the structural differentiator; don't launch without it
- [ ] Comment system with 6-type taxonomy and 4-severity levels — the review taxonomy is airev's identity
- [ ] SQLite persistence with session restore — users won't trust a tool that loses their work
- [ ] Mark file as reviewed with visual progress indicator — completion tracking is essential for multi-file reviews
- [ ] Structured Markdown export (`:clip` / `y`) — closes the feedback loop to Claude Code without MCP
- [ ] Git diff modes: staged, unstaged, commit range, branch comparison — matches critique/tuicr baseline
- [ ] `.aireignore` support — AI-generated changesets contain a lot of noise (lock files, generated code)
- [ ] Color themes (dark, light, catppuccin-mocha, gruvbox-dark) — cosmetic but expected by target audience
- [ ] `?` help overlay — discoverability requirement

### Add After Validation (v1.x)

Features to add once core review workflow is proven.

- [ ] Live file watcher with debounced refresh — the #1 differentiator, but technically complex; validate that static diff mode is used daily first
- [ ] MCP server via stdio — high value, but requires the comment system to be stable first
- [ ] Multi-round thread tracking — requires real usage patterns to design the UX correctly
- [ ] Vim command line (`:` mode) — power-user feature; add after basic keybindings are stable
- [ ] Jujutsu (jj) VCS support — growing audience, low risk to add once git path is solid

### Future Consideration (v2+)

Features to defer until product-market fit is established.

- [ ] Tree-sitter syntax highlighting — can launch with bat/syntect-style highlighting; tree-sitter is complex but worth it long-term for performance on large files
- [ ] Commit selection / multi-commit review mode — tuicr has this; useful but not core for AI-change review (usually reviewing uncommitted or latest changes)
- [ ] Custom keybinding configuration (TOML) — build the TOML config structure early, but don't expose all keybindings as configurable until v1 keybinding design is validated
- [ ] Screensaver/presentation mode (gitlogue-inspired) — purely cosmetic, fun, but zero review value

---

## Feature Prioritization Matrix

| Feature | User Value | Implementation Cost | Priority |
|---------|------------|---------------------|----------|
| Vim-native keybindings | HIGH | LOW | P1 |
| Unified diff + syntax highlighting | HIGH | MEDIUM | P1 |
| 3-panel layout | HIGH | MEDIUM | P1 |
| 6-type comment taxonomy + severity | HIGH | LOW | P1 |
| SQLite persistence + session restore | HIGH | MEDIUM | P1 |
| Mark file as reviewed | HIGH | LOW | P1 |
| Structured Markdown export | HIGH | LOW | P1 |
| Git diff modes (staged/unstaged/range/branch) | HIGH | MEDIUM | P1 |
| `.aireignore` | MEDIUM | LOW | P1 |
| Color themes | MEDIUM | LOW | P1 |
| `?` help overlay | MEDIUM | LOW | P1 |
| Live file watcher | HIGH | HIGH | P2 |
| MCP server via stdio | HIGH | HIGH | P2 |
| Multi-round thread tracking | HIGH | HIGH | P2 |
| Vim command line (`:` mode) | MEDIUM | MEDIUM | P2 |
| Jujutsu VCS support | MEDIUM | MEDIUM | P2 |
| Tree-sitter highlighting | MEDIUM | HIGH | P3 |
| Multi-commit review mode | MEDIUM | MEDIUM | P3 |
| Custom keybinding TOML config | LOW | MEDIUM | P3 |

**Priority key:**
- P1: Must have for launch
- P2: Should have, add when possible
- P3: Nice to have, future consideration

---

## Competitor Feature Analysis

| Feature | tuicr | diffview.nvim | critique | review.nvim | airev target |
|---------|-------|---------------|----------|-------------|--------------|
| Vim keybindings | YES (j/k, g/G, {/}, [/]) | YES (Neovim-native) | NO (arrow keys) | YES (Neovim-native) | YES — match tuicr baseline |
| Syntax highlighting | NO (no highlighting mentioned) | YES (treesitter via Neovim) | YES (Shiki, 18+ langs) | YES (via Neovim) | YES — tree-sitter or syntect |
| File list panel | YES | YES | YES (Ctrl+P dropdown) | YES | YES |
| Multi diff modes | YES (commit, range, uncommitted) | YES (index, log, range) | YES (staged, commit, branch) | YES (branch, commit, uncommitted) | YES — match critique baseline |
| Comment taxonomy | 4 types (ISSUE/SUGGESTION/NOTE/PRAISE) | NONE | NONE | 4 types (Note/Suggestion/Issue/Praise) | 6 types (adds `til`, `nitpick`) |
| Severity levels | NONE | NONE | NONE | NONE | YES (critical/major/minor/info) — unique |
| Session persistence | YES (flat file, XDG) | NONE (Neovim state only) | NONE | YES (~/.local/share/nvim/review/) | YES (SQLite) |
| Mark file reviewed | YES | NO | NO | YES | YES |
| Live file watcher | NO | watch_index (index only) | YES (--watch flag) | NO | YES — real-time full project watch |
| MCP integration | NO (Claude Code skill via tmux+clipboard) | NO | YES (AI review via OpenCode/Claude backend) | NO | YES — stdio MCP server |
| Multi-round threads | NO | NO | NO | NO | YES — unique |
| 3-panel layout | NO (2-panel + toggle) | NO (2-panel diff + file list) | NO (single diff view) | NO | YES — unique |
| Structured Markdown export | YES (clipboard) | NO | NO | YES (clipboard + preview) | YES — richer format with thread history |
| `.ignore` file support | YES (.tuicrignore) | NO | NO | NO | YES (.aireignore) |
| jj support | YES | NO | NO | NO | YES (v1.x) |
| Colon command mode | YES (:w, :clip, :q, :diff) | NO | NO | NO | YES |
| Color themes | YES (catppuccin, gruvbox) | YES (via Neovim colorscheme) | YES (dark/light auto) | YES (via Neovim) | YES |
| Word-level diff | NO | YES (via Neovim diff algo) | YES | NO | YES — within-line diffs are table stakes |

---

## TUI Performance Expectations (Neovim Users)

Neovim users are particularly sensitive to latency. These are not features — they are requirements for the tool to feel "native":

- **Startup time under 100ms.** Rust/ratatui should easily achieve this. Any initialization that blocks the TUI (loading all comments from SQLite, parsing large diffs) must be async.
- **Scroll must feel like Neovim scrolling.** Half-page (`Ctrl-d/u`) and full-page (`Ctrl-f/b`) at 60fps. Frame drops on scroll are disqualifying.
- **Large diff handling.** AI-generated changesets can be 500+ line diffs. Virtual scrolling (don't render off-screen lines) is required. gitlogue's approach (tree-sitter incremental parsing) handles this without stutter.
- **No spinner/loading states for local operations.** SQLite reads and git diff output should be fast enough that no loading indicator is needed. If it appears, it signals a performance problem to fix — not a UX to design around.
- **Debounce, don't throttle.** For live file watching, debounce at 200ms (wait for writes to stop). Throttling causes partial-file diffs on rapid saves from Claude Code.

---

## SSH / Headless Workflow Expectations

A significant portion of the target audience runs Claude Code on remote machines via SSH and reviews from within a tmux session.

- **No X11 / Wayland dependency.** Pure terminal rendering (ANSI escape codes). No OS-level clipboard assumptions.
- **`:clip` must work via OSC 52** for clipboard over SSH. Standard `pbcopy`/`xclip` clipboard will fail in SSH sessions. OSC 52 (terminal clipboard protocol) works in modern terminals (iTerm2, WezTerm, Alacritty, kitty) over SSH.
- **No desktop notification APIs.** libnotify, osascript — all fail in SSH headless contexts. Visual-only status updates in the TUI.
- **tmux-aware layout.** The 3-panel layout must work in a tmux pane. Minimum usable width: 80 columns. The file list and comments panels should be collapsible when viewport is narrow.

---

## Sources

**Competitor repos and documentation analyzed:**
- [tuicr GitHub](https://github.com/agavra/tuicr) — direct predecessor, feature baseline, Claude Code skill pattern (HIGH confidence — official repo)
- [tuicr.dev](https://tuicr.dev/) — official docs (HIGH confidence)
- [diffview.nvim GitHub](https://github.com/sindrets/diffview.nvim) — gold-standard Neovim diff viewer, layout patterns (HIGH confidence)
- [review.nvim GitHub](https://github.com/georgeguimaraes/review.nvim) — comment taxonomy reference (HIGH confidence)
- [critique GitHub](https://github.com/remorses/critique) — diff mode baseline, live watch pattern (HIGH confidence)
- [gitlogue GitHub](https://github.com/unhappychoice/gitlogue) — tree-sitter highlighting, ratatui patterns (HIGH confidence)
- [lazygit GitHub](https://github.com/jesseduffield/lazygit) — keybinding model, UX philosophy (HIGH confidence)
- [OpenCode docs](https://opencode.ai/docs/) — agent workflow patterns, inline diff UX (MEDIUM confidence)

**Industry research:**
- [State of AI Code Review Tools 2025](https://www.devtoolsacademy.com/blog/state-of-ai-code-review-tools-2025/) — severity taxonomy patterns (MEDIUM confidence)
- [BitsAI-CR — Automated Code Review via LLM](https://arxiv.org/html/2501.15134v1) — structured taxonomy research (HIGH confidence — peer reviewed)
- [Atlassian Rovo Dev Research: Comment Types](https://www.atlassian.com/blog/atlassian-engineering/atlassian-rovo-dev-research-what-types-of-code-review-comments-do-developers-most-frequently-resolve/) — comment taxonomy patterns (HIGH confidence)
- [MCP Specification 2025-11-25](https://modelcontextprotocol.io/specification/2025-11-25) — stdio transport as standard for local integrations (HIGH confidence — official spec)
- [diffwatch — real-time file watching TUI](https://github.com/deemkeen/diffwatch) — debounce pattern, event coalescing (MEDIUM confidence)

**Keybinding and UX patterns:**
- [Vim Keybindings Everywhere list](https://github.com/erikw/vim-keybindings-everywhere-the-ultimate-list) — industry convention (HIGH confidence)
- [lazygit UX patterns](https://www.bytesizego.com/blog/lazygit-the-terminal-ui-that-makes-git-actually-usable) — design philosophy (MEDIUM confidence)
- [JetBrains Terminal Architecture](https://blog.jetbrains.com/idea/2025/04/jetbrains-terminal-a-new-architecture/) — muscle memory importance (HIGH confidence)

---
*Feature research for: terminal/TUI code review tool for AI-generated changes (airev)*
*Researched: 2026-02-17*
