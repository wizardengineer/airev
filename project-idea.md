# Designing a terminal code review tool for AI-generated changes

**Writing comments about AI-generated code produces nearly the same learning retention as writing the code yourself.** A 2024 study testing seven cognitive engagement techniques on AI-generated code found that "Lead-and-Reveal" (predicting what code does before seeing it) and "Trace-and-Predict" (tracing execution and predicting outputs) matched or exceeded writing code for comprehension — while passive review without commenting was the worst performer. This means a well-designed review tool with mandatory annotation isn't a compromise; it's a genuine learning mechanism. The tool described here — code-named `airev` — combines a ratatui-based TUI with structured inline commenting, spaced repetition, and an MCP-based agent feedback loop to close the gap between "the agent wrote it" and "I understand it."

Two existing tools validate this design space: **tuicr**, a Rust/ratatui TUI for reviewing AI code changes with inline comments and clipboard export, and **review.nvim**, a Neovim plugin with the same comment-to-agent export pattern. Neither is complete. `airev` would unify the best of both with the learning retention layer missing from everything on the market.

---

## The cognitive science is unambiguous: writing comments works

The research foundation for this tool is strong across multiple independent lines of evidence.

**The generation effect** (Slamecka & Graf, 1978) establishes that information actively generated from one's own mind is retained far better than information passively read. A meta-analysis by Dunlosky et al. (2013) in *Psychological Science in the Public Interest* rated elaborative interrogation and self-explanation as moderate-utility learning strategies, and practice testing as high-utility — all forms of generative engagement. The landmark Karpicke & Roediger study found **~57% retention with active recall versus ~29% for passive reading**, nearly a 2x improvement.

Applied to code specifically, Bacchelli & Bird's ICSE 2013 study at Microsoft found that code review's primary real-world benefit isn't defect detection — it's **knowledge transfer**. Rigby & Bird (2013) quantified this: conducting peer review increases the number of distinct files a developer knows about by **66% to 150%**. The Chi & Wylie ICAP framework ranks learning activities as Interactive > Constructive > Active > Passive, where writing review comments is "Constructive" — the second-highest tier.

The most directly relevant study is a 2024 paper (arXiv 2410.08922) that tested cognitive engagement techniques specifically for AI-generated code with 124 participants. **"Verify-and-Review"** (essentially passive code review) was tested but underperformed. The winners were **Lead-and-Reveal** (predict what the code should do before seeing it) and **Trace-and-Predict** (trace execution line-by-line, predict variable values). Both require the reviewer to *generate* understanding before seeing the answer — exactly the pattern `airev` should enforce.

For diff-based comprehension specifically, Google's engineering practices explicitly warn that reviewing diffs alone is insufficient: "Sometimes you have to look at the whole file to be sure that the change actually makes sense." The Cisco/SmartBear study found defect-finding ability drops after **200 lines of code** and reviewers scanning more than 400 lines/hour have below-average effectiveness. The design implication: show diffs with expandable context, break large changes into reviewable chunks, and order files by logical dependency rather than alphabetically.

---

## The existing tool landscape has a clear gap

A comprehensive survey of existing tools reveals that no single tool combines structured diff review, inline annotation with a comment taxonomy, AI-optimized export, and a learning retention layer. Here's where each falls short:

**tuicr** (Rust/ratatui, ~11K SLoC) is the closest predecessor. It provides a file list panel, infinite-scroll unified diff with expandable context, inline comments with a Note/Suggestion/Issue/Praise taxonomy, and `:clip`/`:export` commands that copy structured markdown to clipboard for AI agents. It even ships with a Claude Code skill. Its limitations: unified diff only (no side-by-side), no syntax highlighting beyond basic coloring, no Neovim integration, no learning retention features, and works only on uncommitted changes.

**review.nvim** is the Neovim-native equivalent, built on codediff.nvim for side-by-side diffs. It uses the same four-type comment taxonomy, stores comments per branch in `~/.local/share/nvim/review/`, and exports via `:Review export` and `:Review sidekick`. It's very new (24 stars) and lacks spaced repetition, multi-round tracking, or MCP integration.

**diffview.nvim** (5,220+ stars) is the gold-standard Neovim diff viewer — file panel + side-by-side diff using Neovim's built-in diff mode, with `]c`/`[c` hunk navigation and VCS adapter support. But it has zero commenting capability. **octo.nvim** adds GitHub PR inline commenting to Neovim but is tightly coupled to GitHub's API and can't work offline or with local-only diffs.

**OpenCode** (95K+ stars, Go/Bubble Tea) is the dominant terminal AI coding agent. It shows file diffs inline in conversations and has git-based undo/redo, but has no structured review step — the gap between "agent makes changes" and "changes are accepted" is exactly what `airev` fills.

Among commercial tools, **Codex** offers a `codex review` CLI subcommand with diff viewing and can post GitHub PR reviews, but it's designed for AI-reviewing-code, not human-reviewing-AI-code. **Cursor's BugBot** runs multiple parallel analysis passes with majority voting, reviews 2M+ PRs/month, and has a 70%+ resolution rate — but again, it's AI review, not human learning.

The critical insight from GitHub, GitLab, and Gerrit's data models is the comment structure to adopt. Gerrit pioneered the most sophisticated model: comments with character-level ranges, fix suggestions as structured objects (not embedded markdown), unresolved/resolved thread states, and a dedicated `RobotCommentInfo` type for automated comments. GitHub's "pending review" batching pattern (accumulate draft comments, submit all at once) is essential for TUI workflow. GitLab's innovation of unresolved threads blocking merge maps to requiring all critical comments be addressed before accepting.

---

## Architecture: Rust, ratatui, SQLite, and vim-native keybindings

### Framework choice: ratatui decisively

**ratatui** (Rust) wins over Bubble Tea (Go), Textual (Python), and blessed (Node.js) for this use case. It uses **30-40% less memory and 15% less CPU** than Bubble Tea equivalents — critical for rendering large LLVM diffs with syntax highlighting across hundreds of files. The ecosystem provides every needed widget: `edtui` for vim-mode inline editing, `ratatui-code-editor` with tree-sitter syntax highlighting, `ratatui-explorer` for file trees, `tui-popup` for floating windows, and `ansi-to-tui` for colored diff output. tuicr, delta, difftastic, and gitui all prove ratatui's maturity for this exact class of tool.

Key Rust crates for the dependency tree: `ratatui` + `crossterm` (core TUI), `edtui` (vim-mode comment editor), `similar` or `diffy` (diff computation), `git2` (libgit2 bindings), `tree-sitter` + language grammars (syntax parsing and structural diff), `arboard` (system clipboard), `rusqlite` (SQLite), `tokio` (async runtime).

### The three-panel layout

The layout mirrors GitHub's PR review adapted for terminal constraints:

```
┌──────────────────────────────────────────────────────────────────────────┐
│ airev ── llvm-project ── Refactor InstCombine  [3/47 files]  Round 1   │
├────────────┬───────────────────────────────────────────┬────────────────┤
│ FILES      │ DIFF VIEW                                 │ COMMENTS       │
│            │                                           │                │
│ ▸ lib/     │ lib/Transforms/InstCombine/Shifts.cpp     │ ── Line 142 ──│
│   ● Shif…  │ @@ -138,7 +138,9 @@                      │ This should    │
│   ◐ Comp…  │  138│ Value *Op0 = I.getOp(0);            │ use m_Shr()    │
│   ○ Cast…  │ -141│ if (auto *C = dyn_cast…              │ matcher.       │
│ ▸ include/ │ +141│ // Use pattern matcher               │                │
│   ○ Patt…  │ +142│ if (match(Op1, m_Const…              │ severity: med  │
│            │  ┌── 💬 @reviewer (medium) ──────────┐    │                │
│ ── Stats ──│  │ Should use m_Shr() to catch both  │    │ ── Line 187 ──│
│ ● 3 done   │  │ LShr and AShr patterns.           │    │ Missing edge   │
│ ◐ 2 partial│  └──────────────────────────────────┘    │ case for       │
│ ○ 42 left  │  143│   if (C->isNullValue())              │ poison vals.   │
├────────────┴───────────────────────────────────────────┴────────────────┤
│ ]c next hunk  [c prev  c comment  a accept  x reject  ya yank-all  ? │
└──────────────────────────────────────────────────────────────────────────┘
```

Panel proportions adapt to terminal width: at ≥160 columns, full three-panel with side-by-side diff; at 120–159, three-panel with unified diff; at 80–119, two-panel with comments as overlay; below 80, single panel with tab switching. The diff panel needs minimum **120 columns** for side-by-side (60 chars per side including line numbers and gutter). A toggle keybinding (`S`) forces unified/split mode.

### Keybindings designed for a Neovim user

Every keybinding follows vim conventions. The tool is fully keyboard-driven with no mouse requirement.

**Navigation** uses standard vim motions: `j/k` for line movement, `Ctrl-d/u` for half-page scrolling, `gg/G` for top/bottom, `h/l` or `Tab/S-Tab` to switch panels, `/` for search, `n/N` for search results. **Diff navigation** uses vim-diff conventions: `]c/[c` for next/previous hunk, `]f/[f` for next/previous file, `]C/[C` for next/previous comment.

**Review actions** are single-key: `c` to comment on current line, `v` then `C` for line-range comments (visual select), `gc` to edit existing comment, `dc` to delete, `a` to accept hunk, `x` to reject, `r` to mark file as reviewed. The review queue auto-advances: after marking a file reviewed, `]f` jumps to the next unreviewed file.

**Comment editing** uses a hybrid approach. For short comments (the 90% case), `edtui` provides inline vim-mode editing with normal/insert/visual modes directly in the TUI. For longer comments, `Ctrl-e` suspends the TUI and spawns `$EDITOR` (Neovim) on a temp file — the same pattern lazygit uses for commit messages, proven for this audience. **Export** uses `y`-prefix chords: `ya` yanks all comments as markdown, `yj` yanks as JSON, `yf` yanks current file's comments, `yh` yanks current hunk with context.

**Command mode** (`:`) provides power commands: `:sort size` to reorder files by diff size, `:filter test` to show only test files, `:queue` to show only unreviewed files, `:export --json` for file export, `:predict` to enter predict-before-reveal mode, `:quiz` to start a spaced repetition session.

---

## The learning engine: comments, spaced repetition, and TIL summaries

### Data model in SQLite

The tool stores all review data in `.airev/reviews.db` using SQLite. SQLite wins over markdown files (not queryable) and git notes (one note per object, fragile push/pull, no structured queries). For a terminal power user, SQLite is inspectable via `sqlite3` CLI, composable with shell scripts, and supports FTS5 full-text search across all comments.

The core schema has four tables. `review_sessions` captures each review round: session ID (ULID), timestamps, git commit range, branch, agent name and model, review outcome (accepted/rejected/changes_requested/pending), time spent, difficulty and confidence ratings (1–5), round number, and a `thread_id` linking multiple rounds of the same review. `review_comments` stores each annotation: file path, line range, comment text, type (question/concern/til/suggestion/praise/nitpick), severity (critical/major/minor/info), surrounding context, and resolution status. `review_files` tracks per-file metadata. `sr_cards` handles spaced repetition state.

```sql
CREATE TABLE review_sessions (
  id              TEXT PRIMARY KEY,    -- ULID
  created_at      TEXT NOT NULL,       -- ISO 8601
  git_commit_start TEXT,
  git_commit_end  TEXT NOT NULL,
  git_branch      TEXT,
  agent_name      TEXT,                -- 'claude-code', 'aider', 'codex', NULL
  agent_model     TEXT,                -- 'claude-sonnet-4', etc.
  review_outcome  TEXT CHECK(review_outcome IN
    ('accepted','rejected','changes_requested','pending')),
  time_spent_sec  INTEGER,
  difficulty      INTEGER CHECK(difficulty BETWEEN 1 AND 5),
  confidence      INTEGER CHECK(confidence BETWEEN 1 AND 5),
  summary         TEXT,
  round_number    INTEGER DEFAULT 1,
  thread_id       TEXT
);

CREATE TABLE review_comments (
  id              TEXT PRIMARY KEY,
  session_id      TEXT NOT NULL REFERENCES review_sessions(id),
  file_path       TEXT NOT NULL,
  line_start      INTEGER,
  line_end        INTEGER,
  comment_text    TEXT NOT NULL,
  comment_type    TEXT CHECK(comment_type IN
    ('question','concern','til','suggestion','praise','nitpick')),
  severity        TEXT CHECK(severity IN
    ('critical','major','minor','info')) DEFAULT 'info',
  surrounding_context TEXT,
  resolved        INTEGER DEFAULT 0,
  resolved_in_round INTEGER,
  created_at      TEXT NOT NULL
);
```

The `comment_type` taxonomy is the most important design decision in the data model. Six types emerge from the research and from the tuicr/review.nvim precedent: **question** ("Why was this approach chosen?" — activates elaborative interrogation), **concern** ("This could fail with poison values" — flags issues), **til** ("TIL: bounded channels prevent OOM" — marks learnings), **suggestion** ("Use m_Shr() instead" — proposes alternatives), **praise** ("Clean use of the builder pattern" — reinforces good patterns), and **nitpick** ("Naming: prefer `computeKnownBits`" — low-severity style). Requiring the reviewer to categorize each comment prevents rubber-stamping and activates the generation effect.

### Spaced repetition built on SM-2

Comments of type `til` and `question` automatically become candidates for spaced repetition cards. The tool implements the SM-2 algorithm (the same scheduler Anki uses) natively, without requiring Anki as a dependency. Each card has an ease factor (default 2.5), interval in days, and next review date. After each quiz answer rated 1–4, the interval adjusts: correct answers with rating ≥3 extend the interval by the ease factor; incorrect answers reset to 1 day.

The quiz runs in the terminal via `:quiz` or `airev quiz`:

```
╔═══════════════════════════════════════════════════════╗
║  REVIEW CARD  [3 of 12 due today]                     ║
║  From: feature/instcombine-refactor (2026-01-15)      ║
╠═══════════════════════════════════════════════════════╣
║  In InstCombineShifts.cpp:142, the agent replaced     ║
║  dyn_cast<ConstantInt> with a pattern matcher.        ║
║  Why is m_Shr() preferred over m_ConstantInt() here?  ║
║                                                       ║
║  [SPACE to reveal]                                    ║
╠═══════════════════════════════════════════════════════╣
║  m_Shr() matches both LShr and AShr, catching both    ║
║  logical and arithmetic shifts. m_ConstantInt() only  ║
║  matches the constant operand, missing the shift      ║
║  opcode distinction. See PatternMatch.h:423.          ║
║                                                       ║
║  [1-Again] [2-Hard] [3-Good] [4-Easy]                ║
╚═══════════════════════════════════════════════════════╝
```

For Anki integration, the tool optionally exports cards via the AnkiConnect HTTP API (port 8765) using a custom "CodeReview" note type with Front (question), Back (answer with code snippet), Context (commit, branch, agent), and Tags fields. The `airev export --anki` command sends a batch via `addNotes`.

A daily tmux status-bar hook can display due card count: `#(airev due-count)` renders as `📚 3 due` in the tmux status line, prompting daily review without being intrusive.

### TIL summaries close the learning loop

At session close, the tool generates a TIL markdown file in `.airev/til/` by grouping `til` and `question` comments by file, including code context, and listing remaining open questions. This can optionally be LLM-assisted (feed comments + diff to generate a summary) or purely manual for engineers who prefer writing their own. The TIL archive becomes a searchable personal knowledge base — SQL queries like `SELECT * FROM review_comments WHERE comment_type = 'til' AND file_path LIKE '%InstCombine%'` surface past learnings for a specific subsystem instantly.

---

## The predict-before-reveal mode changes everything

The 2024 cognitive engagement study's strongest finding is that **predicting before seeing** dramatically outperforms passive review. `airev` should implement this as a first-class mode (`:predict` or `-p` flag):

**Step 1 — Context**: Show the function signature, surrounding code, and the commit message/agent prompt that generated the change. Hide the actual implementation.

**Step 2 — Predict**: The reviewer writes what they expect the implementation to look like (stored as a `prediction` comment type). For LLVM work, this might be: "I'd expect this to use `match()` with `m_Shr()` to catch both LShr and AShr, then check for a zero shift amount."

**Step 3 — Reveal**: Show the actual diff. Discrepancies between prediction and reality are highlighted — these are the learning moments. The reviewer annotates why the agent's approach differed from their expectation.

**Step 4 — Reflect**: A forced self-explanation prompt: "What did the agent do differently than you expected, and why might their approach be better (or worse)?" This maps directly to the "scaffolded self-explanation" technique that Oli et al. (AIED 2023) found particularly beneficial for learners.

This mode adds friction intentionally — what the research calls "desirable difficulty." It transforms review from a gatekeeping task into a deliberate practice session. For a compiler engineer reviewing AI-generated LLVM changes, predicting the implementation requires activating deep knowledge of LLVM's IR, pattern matching APIs, and optimization semantics — exactly the knowledge you want to retain.

---

## The agent feedback loop: export, MCP, and multi-round review

### Clipboard export in two formats

The primary export (yanked via `ya`) is markdown optimized for pasting directly into Claude Code, Codex, or Aider:

```markdown
## Code Review: feature/instcombine-refactor (Round 1)
**Outcome:** Changes Requested | **Files:** 12/47 reviewed

### 🔴 CRITICAL: lib/Transforms/InstCombine/InstCombineShifts.cpp:187-192
> Missing edge case for poison values
```cpp
// Context (lines 185-195):
  if (match(&I, m_Shr(m_Value(Op0), m_ConstantInt(C)))) {
    // BUG: When shift amount is poison, this is unsound per LangRef
    return replaceInstUsesWith(I, Op0);
```
**Action:** Add `isGuaranteedNotToBePoison()` check before transformation.

### 🟡 MAJOR: lib/Transforms/InstCombine/InstCombineShifts.cpp:142
> Should use m_Shr() matcher for both LShr and AShr
**Suggested fix:**
```cpp
if (match(&I, m_Shr(m_Value(Op0), m_ConstantInt(C))))
```
```

The secondary export (`yj`) produces JSON with explicit fields for `file`, `line_start`, `line_end`, `comment`, `severity`, `type`, `surrounding_context`, and `context_start_line`. Each comment includes **3-5 lines of surrounding code** because agents need local scope to make targeted fixes without re-reading the full file.

### MCP server for bidirectional agent communication

The most powerful integration is an MCP server (`airev mcp-server`) that AI agents connect to via stdio transport. Five tools enable a full review loop:

- **`submit_for_review`**: Agent pushes a commit range + change summary, receives a session ID
- **`get_review_status`**: Agent polls for review completion (pending/accepted/changes_requested)
- **`get_review_comments`**: Agent retrieves structured feedback, filterable by status (open/resolved)
- **`get_review_history`**: Agent queries past reviews for a file or branch to proactively avoid repeated issues
- **`acknowledge_comments`**: Agent marks comments as addressed before the next review round

Three MCP resources provide additional context: `airev://sessions/{id}` for full session details, `airev://threads/{id}` for multi-round history, and `airev://patterns` for frequently raised issues (enabling the agent to self-review against known patterns before submitting).

Configuration for Claude Code is a single addition to `~/.claude/mcp_servers.json`:
```json
{ "airev": { "command": "airev", "args": ["mcp-server"] } }
```

### Multi-round review tracks comment resolution

All rounds of a review share a `thread_id`. Each round is a separate session with incrementing `round_number`. When exporting for round N+1, only `open` comments from previous rounds are included — preventing the agent from re-addressing resolved issues. The SQL query is straightforward:

```sql
SELECT c.* FROM review_comments c
JOIN review_sessions s ON c.session_id = s.id
WHERE s.thread_id = ? AND c.resolved = 0
ORDER BY c.severity DESC;
```

The CLI workflow: `airev start --branch feature/x --agent claude-code` → review in TUI → `airev finish --outcome changes_requested` → `airev export --thread {id} --open-only | pbcopy` → paste to agent → agent fixes → `airev start --thread {id}` → review round 2. Git state is recorded but not managed — no automatic branching or stashing, respecting the power user's existing workflow.

---

## Conclusion: the design decisions that matter most

Three decisions determine whether this tool succeeds or becomes shelfware. **First, mandatory comment typing.** Every annotation must be categorized as question/concern/til/suggestion/praise/nitpick. This single constraint transforms passive scanning into active learning — the generation effect applied at scale. Without it, the tool degrades to a fancy diff viewer.

**Second, the predict-before-reveal mode.** This is the highest-leverage feature for a compiler engineer reviewing AI changes to LLVM. Predicting what `InstCombineShifts` should look like before seeing the agent's implementation forces retrieval practice on the exact knowledge (pattern matching APIs, IR semantics, optimization correctness conditions) that makes the engineer effective. No existing tool implements this.

**Third, the MCP feedback loop.** The clipboard export is table stakes; MCP integration is the differentiator. When Claude Code can call `get_review_history` to check "what has this reviewer flagged on InstCombine before?" and proactively avoid those issues, the review rounds shrink and the tool becomes a force multiplier rather than a bottleneck. The `airev://patterns` resource turns the reviewer's accumulated judgment into a reusable asset that improves the agent's output over time — a flywheel where reviewing code makes the AI write better code, which makes review faster, which generates more learning data.

The implementation order should be: core TUI with diff view and inline comments (weeks 1-3), clipboard export in markdown/JSON (week 4), SQLite persistence and multi-round tracking (weeks 5-6), predict-before-reveal mode (week 7), spaced repetition engine (weeks 8-9), MCP server (weeks 10-11). The Rust/ratatui foundation ensures the tool renders LLVM-scale diffs (hundreds of files, thousands of lines) without perceptible lag — a non-negotiable requirement for a compiler engineer who lives in the terminal.
