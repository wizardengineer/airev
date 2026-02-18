---
phase: quick
plan: 2
type: execute
wave: 1
depends_on: []
files_modified:
  - airev/src/git/worker.rs
  - airev/src/ui/help.rs
autonomous: true
requirements: []

must_haves:
  truths:
    - "No raw text appears on stderr while the TUI is running"
    - "The help overlay lists Tab, Enter/l, [, ] as working keybindings"
    - "The help overlay does not describe [/] as a placeholder"
  artifacts:
    - path: "airev/src/git/worker.rs"
      provides: "Silent error handling — no eprintln!/println! calls"
    - path: "airev/src/ui/help.rs"
      provides: "Accurate Phase 3 keybinding descriptions"
  key_links:
    - from: "airev/src/git/worker.rs handle_request"
      to: "GitResultPayload (empty)"
      via: "returns empty payload on error, no eprintln!"
      pattern: "GitResultPayload \\{"
---

<objective>
Fix two bugs causing a broken TUI experience:
1. `eprintln!` calls in `git/worker.rs` write to stderr (the TUI backend), corrupting the display with raw error text.
2. The help overlay in `ui/help.rs` lists [/] as placeholders and omits Tab (diff mode cycle) and Enter/l (file jump) which are fully wired since Phase 3 Plan 04.

Purpose: Eliminate display corruption and give users accurate keybinding documentation.
Output: Silent git worker errors, accurate help overlay content.
</objective>

<execution_context>
@/Users/juliusalexandre/.claude/get-shit-done/workflows/execute-plan.md
@/Users/juliusalexandre/.claude/get-shit-done/templates/summary.md
</execution_context>

<context>
@.planning/PROJECT.md
@.planning/ROADMAP.md
@.planning/STATE.md

@airev/src/git/worker.rs
@airev/src/ui/help.rs
</context>

<tasks>

<task type="auto">
  <name>Task 1: Remove eprintln! calls from git worker</name>
  <files>airev/src/git/worker.rs</files>
  <action>
    Remove all `eprintln!` macro calls from `git_worker_loop` and `handle_request`.

    Specific changes:
    1. Line 43: Remove the `eprintln!("[git_worker] Failed to open repository at {path}: {e}");` line.
       The thread already returns early on repo-open failure, which results in the diff panel
       showing "No diff loaded" — this is the correct graceful behaviour. No logging needed.

    2. Line 68: Remove the `eprintln!("[git_worker] Diff error for mode {mode:?}: {e}");` line.
       The function already returns an empty `GitResultPayload` (hunks/files/lines all empty),
       which the UI handles gracefully by showing a blank diff panel. The error reason (e.g.
       "revspec 'main' not found" for BranchComparison, "CommitRange requires LoadDiffRange"
       for the CommitRange no-op) does not need to be surfaced to stderr.

    Do NOT add any replacement logging mechanism — silent failure with empty payload is the
    correct behaviour for a TUI app where stderr is the terminal backend.

    Also update the doc comment on `handle_request` (line 56) to remove the phrase
    "logs to stderr" since it will no longer log to stderr. Change it to:
    "On git2 errors, returns an empty payload for graceful degradation."

    The `use std::sync::LazyLock;` import and all other code remain unchanged.
  </action>
  <verify>
    Run: `cd /Users/juliusalexandre/Projects/PersonalWork/diff-grief/airev && cargo build 2>&1`
    Then grep to confirm no eprintln remains:
    `grep -n "eprintln" /Users/juliusalexandre/Projects/PersonalWork/diff-grief/airev/src/git/worker.rs`
    (should return no output)
  </verify>
  <done>
    `cargo build` succeeds with no errors. No `eprintln!` or `println!` macros exist in
    `airev/src/git/worker.rs`.
  </done>
</task>

<task type="auto">
  <name>Task 2: Update help overlay with Phase 3 keybindings</name>
  <files>airev/src/ui/help.rs</files>
  <action>
    Update `build_help_text()` in `airev/src/ui/help.rs` to accurately reflect all
    keybindings wired through Phase 3 Plan 04.

    Replace the entire `Text::from(vec![...])` body with:

    ```
    Text::from(vec![
        Line::from("Navigation"),
        Line::from("  j / k         Scroll down / up one line"),
        Line::from("  g / G         Jump to top / bottom"),
        Line::from("  Ctrl-d / u    Scroll half page down / up"),
        Line::from("  Ctrl-f / b    Scroll full page down / up"),
        Line::from("  H / L         Move panel focus left / right"),
        Line::from(""),
        Line::from("File List"),
        Line::from("  { / }         Previous / next file"),
        Line::from("  Enter / l     Jump to selected file in diff view"),
        Line::from(""),
        Line::from("Diff View"),
        Line::from("  [ / ]         Previous / next hunk"),
        Line::from("  < / >         Shrink / grow diff panel by 5%"),
        Line::from(""),
        Line::from("Diff Mode  (Tab cycles through all modes)"),
        Line::from("  Unstaged  ->  Staged  ->  Branch vs main  ->  Commit Range"),
        Line::from(""),
        Line::from("General"),
        Line::from("  ?             Open / close this help overlay"),
        Line::from("  q / Esc       Quit (confirms if unsaved comments exist)"),
    ])
    ```

    Key changes from the current version:
    - Add `Enter / l` line under File List (wired in Phase 3 Plan 04)
    - Remove "(placeholder)" from the [/] hunk navigation line — these are fully wired
    - Add a "Diff Mode" section explaining Tab cycling (wired in Phase 3 Plan 04)
    - Rename "Navigation (all modes)" to "Navigation" (simpler, accurate)

    The function signature, module doc, imports, and `render_help_overlay` function are
    all unchanged.
  </action>
  <verify>
    Run: `cd /Users/juliusalexandre/Projects/PersonalWork/diff-grief/airev && cargo build 2>&1`
    Visually inspect the updated file:
    `grep -n "placeholder\|Tab\|Enter" /Users/juliusalexandre/Projects/PersonalWork/diff-grief/airev/src/ui/help.rs`
    Expected: "Tab" appears, "placeholder" does not appear, "Enter" appears.
  </verify>
  <done>
    `cargo build` succeeds. The help text contains Tab diff mode cycling and Enter/l file
    jump. The word "placeholder" does not appear in the [/] hunk navigation line.
  </done>
</task>

</tasks>

<verification>
Run the full build to confirm both files compile cleanly:
```
cd /Users/juliusalexandre/Projects/PersonalWork/diff-grief/airev && cargo build 2>&1
```

Confirm no eprintln! in worker:
```
grep -rn "eprintln\|println!" /Users/juliusalexandre/Projects/PersonalWork/diff-grief/airev/src/git/worker.rs
```
(no output expected)

Confirm help overlay has Phase 3 keybindings:
```
grep -n "Tab\|Enter\|placeholder" /Users/juliusalexandre/Projects/PersonalWork/diff-grief/airev/src/ui/help.rs
```
(Tab and Enter appear, placeholder does not)
</verification>

<success_criteria>
- `cargo build` exits 0 with no errors or warnings
- `git/worker.rs` contains zero `eprintln!` or `println!` calls
- `ui/help.rs` lists Tab (diff mode cycle) and Enter/l (file jump) as active keybindings
- `ui/help.rs` does not describe [/] as placeholder
- Running the TUI against a repo without a "main" branch produces no visible stderr corruption
</success_criteria>

<output>
After completion, create `.planning/quick/2-fix-status-bar-error-spam-from-git-worke/2-SUMMARY.md`
following the summary template.
</output>
