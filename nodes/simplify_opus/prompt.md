Goal: # Emit `StageStarted` on retry attempts

## Context

When a stage fails with a transient error and is retried, the CLI progress UI freezes because:

1. `StageFailed` calls `finish_stage()`, removing the stage from `active_stages`
2. The retry loop in the engine (`continue` at line 1198) re-enters handler execution **without emitting `StageStarted`**
3. All subsequent agent events for the retry attempt silently drop (no matching entry in `active_stages`)

The `StageStarted` event already has `attempt` and `max_attempts` fields, so emitting it per-attempt is the intended design — it just wasn't wired up.

## Changes

### 1. Engine: emit `StageStarted` at the top of the retry loop

**File:** `lib/crates/fabro-workflows/src/engine.rs`

Move the `StageStarted` emission from before the loop (line 1852) to inside the loop, right after `for attempt in 1..=policy.max_attempts {` (line 1079). This way every attempt — including retries — emits the event with the correct `attempt` number.

The existing emission at line 1852 gets replaced, not duplicated. The `attempt` value comes directly from the loop variable (converted via `usize::try_from`).

### 2. Engine: move StageStart hook inside the loop (or keep it outside)

The `StageStart` hook block (lines 1862-1895) currently runs once before the loop. It should stay outside — hooks shouldn't re-fire on retries. Only the `StageStarted` event emission moves inside.

### 3. UI: no changes needed

`on_stage_started` in `run_progress.rs` already handles being called for the same `node_id` — it inserts a fresh `ActiveStage` into the map, creating a new spinner. The `StageFailed` handler correctly finishes the old spinner. The natural event sequence becomes:

```
StageStarted  (attempt 1) → spinner created
StageFailed   (will_retry) → spinner finished with ✗
StageStarted  (attempt 2) → new spinner created
Agent events  → attach to new spinner
StageCompleted (attempt 2) → spinner finished with ✓
```

## Verification

1. `cargo test -p fabro-workflows` — existing tests pass
2. `cargo clippy --workspace -- -D warnings` — no warnings
3. Manual: run a workflow that hits a transient LLM error (or mock one) and verify the CLI shows the retry spinner with tool calls


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
  - Model: claude-opus-4-6, 42.2k tokens in / 12.1k out
  - Files: /home/daytona/workspace/lib/crates/fabro-workflows/src/engine.rs


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