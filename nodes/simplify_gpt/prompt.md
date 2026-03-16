Goal: # Fix: Stage durations always show 0s in PR descriptions

## Context

`extract_stage_durations()` in `retro.rs` builds a HashMap keyed by `node_label` (the human-readable name like `"Preflight Compile"`), but all three lookup sites use `node_id` (the DOT graph identifier like `"preflight_compile"`). Every lookup misses, falls back to `unwrap_or(0)`, and all stages show `0s`.

The existing test doesn't catch this because it uses nodes where `node_id == node_label` (both `"plan"`).

## Changes

### 1. Fix the key mismatch — `retro.rs:196`

Change `envelope.get("node_label")` to `envelope.get("node_id")` so the HashMap key matches lookups.

```
- let Some(name) = envelope.get("node_label").and_then(|v| v.as_str()) else {
+ let Some(name) = envelope.get("node_id").and_then(|v| v.as_str()) else {
```

**File:** `lib/crates/fabro-workflows/src/retro.rs:196`

### 2. Update existing test to catch this — `retro.rs:529+`

The test at line 529 uses `"node_id": "plan", "node_label": "plan"` for both events. Change the `node_label` values to differ from `node_id` (e.g., `"Plan"` and `"Code"`) so the test would have failed before the fix.

**File:** `lib/crates/fabro-workflows/src/retro.rs` — `extract_stage_durations_from_progress_jsonl` test

## Verification

1. `cargo test -p fabro-workflows -- extract_stage_durations` — confirm test passes with the fix
2. `cargo clippy --workspace -- -D warnings` — no new warnings
3. `cargo test --workspace` — no regressions


## Completed stages
- **toolchain**: success
  - Script: `command -v cargo >/dev/null || { curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh -s -- -y && sudo ln -sf $HOME/.cargo/bin/* /usr/local/bin/; }; cargo --version 2>&1`
  - Stdout:
    ```
    cargo 1.94.0 (85eff7c80 2026-01-15)
    ```
  - Stderr: (empty)
- **preflight_compile**: success
  - Script: `cargo check -q --workspace 2>&1`
  - Stdout: (empty)
  - Stderr: (empty)
- **preflight_lint**: success
  - Script: `cargo clippy -q --workspace -- -D warnings 2>&1`
  - Stdout: (empty)
  - Stderr: (empty)
- **implement**: success
  - Model: claude-opus-4-6, 7.5k tokens in / 1.2k out
  - Files: /home/daytona/workspace/lib/crates/fabro-workflows/src/retro.rs
- **simplify_opus**: success
  - Model: claude-opus-4-6, 6.6k tokens in / 3.3k out
- **simplify_gemini**: success
  - Model: claude-opus-4-6, 7.4k tokens in / 3.1k out


# Simplify: Code Review and Cleanup

Review all changed files for reuse, quality, and efficiency. Fix any issues found.

## Phase 1: Identify Changes

Run git diff (or git diff HEAD if there are staged changes) to see what changed. If there are no git changes, review the most recently modified files that the user mentioned or that you edited earlier in this conversation.

## Phase 2: Launch Three Review Agents in Parallel

Use the Agent tool to launch all three agents concurrently in a single message. Pass each agent the full diff so it has the complete context.

### Agent 1: Code Reuse Review

For each change:

1. Search for existing utilities and helpers that could replace newly written code. Use Grep to find similar patterns elsewhere in the codebase — common locations are utility directories, shared modules, and files adjacent to the changed ones.
2. Flag any new function that duplicates existing functionality. Suggest the existing function to use instead.
3. Flag any inline logic that could use an existing utility — hand-rolled string manipulation, manual path handling, custom environment checks, ad-hoc type guards, and similar patterns are common candidates.

Note: This is a greenfield app, so focus on maximizing simplicity and don't worry about changing things to achieve it.

### Agent 2: Code Quality Review

Review the same changes for hacky patterns:

1. Redundant state: state that duplicates existing state, cached values that could be derived, observers/effects that could be direct calls
2. Parameter sprawl: adding new parameters to a function instead of generalizing or restructuring existing ones
3. Copy-paste with slight variation: near-duplicate code blocks that should be unified with a shared abstraction
4. Leaky abstractions: exposing internal details that should be encapsulated, or breaking existing abstraction boundaries
5. Stringly-typed code: using raw strings where constants, enums (string unions), or branded types already exist in the codebase

Note: This is a greenfield app, so be aggressive in optimizing quality.

### Agent 3: Efficiency Review

Review the same changes for efficiency:

1. Unnecessary work: redundant computations, repeated file reads, duplicate network/API calls, N+1 patterns
2. Missed concurrency: independent operations run sequentially when they could run in parallel
3. Hot-path bloat: new blocking work added to startup or per-request/per-render hot paths
4. Unnecessary existence checks: pre-checking file/resource existence before operating (TOCTOU anti-pattern) — operate directly and handle the error
5. Memory: unbounded data structures, missing cleanup, event listener leaks
6. Overly broad operations: reading entire files when only a portion is needed, loading all items when filtering for one

## Phase 3: Fix Issues

Wait for all three agents to complete. Aggregate their findings and fix each issue directly. If a finding is a false positive or not worth addressing, note it and move on — do not argue with the finding, just skip it.

When done, briefly summarize what was fixed (or confirm the code was already clean).