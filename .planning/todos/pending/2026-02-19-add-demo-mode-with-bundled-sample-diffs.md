---
created: 2026-02-19T07:55:25.312Z
title: Add demo mode with bundled sample diffs
area: general
files: []
---

## Problem

Users who install airev have no way to explore the tool's features without first having a real git repository with active changes. This creates a poor first-run experience — they see "No diff loaded" and have to figure out the workflow on their own.

A demo/tutorial mode would let users immediately try all features: navigating diffs, switching between the 4 diff modes (unstaged, staged, branch comparison, commit range), leaving comments, filtering, and exporting reviews.

## Solution

Add a `--demo` CLI flag that launches airev with bundled sample data:

- **Pre-made diff samples** covering all 4 diff modes — synthetic `OwnedDiffHunk`/`FileSummary` data loaded directly into `AppState` instead of from a real git repo
- **Sample comments/threads** pre-populated in the SQLite database to demonstrate the review workflow (open/addressed/resolved threads, multiple rounds)
- **Guided hints** in the status bar or a welcome overlay pointing users to key keybindings

Target: post-MVP (after Phase 8 Polish). Requires all features to be working before demo data can exercise them.
