Goal: # Wire up missing hook invocations

## Context

Five `HookEvent` variants exist in the enum and are documented in `docs/agents/hooks.mdx`, but `run_hooks()` is never called for them in the engine. Users can configure hooks for these events, but they silently never fire.

**Events to wire up:** `StageRetrying`, `ParallelStart`, `ParallelComplete`
**Events to mark as reserved:** `SandboxReady`, `SandboxCleanup` (sandbox lifecycle is managed outside the engine; wiring these requires significant architecture changes)

## Changes

### 1. Add StageRetrying hook calls in `engine.rs`

File: `lib/crates/fabro-workflows/src/engine.rs`

Two sites in `execute_with_retry`, both immediately after `WorkflowRunEvent::StageRetrying` emission and before `tokio::time::sleep(delay).await`:

- **Site A (~line 1127):** error-retry path
- **Site B (~line 1155):** explicit Retry status path

Pattern (same for both sites):
```rust
{
    let mut hook_ctx = HookContext::new(
        HookEvent::StageRetrying,
        context.run_id(),
        graph.name.clone(),
    );
    hook_ctx.node_id = Some(node.id.clone());
    hook_ctx.node_label = Some(node.label().to_string());
    hook_ctx.handler_type = node.handler_type().map(String::from);
    hook_ctx.attempt = Some(usize::try_from(attempt).unwrap_or(usize::MAX));
    hook_ctx.max_attempts = Some(
        usize::try_from(policy.max_attempts).unwrap_or(usize::MAX),
    );
    let _ = self.run_hooks(&hook_ctx, None).await;
}
```

Available via: `self.run_hooks()` (engine method), `context.run_id()`, `graph.name`, `node`, `attempt`, `policy`.

### 2. Add ParallelStart hook call in `parallel.rs`

File: `lib/crates/fabro-workflows/src/handler/parallel.rs`

After `ParallelStarted` event emission (~line 301). Call `hook_runner.run()` directly since the handler has `services: &EngineServices` (which has `hook_runner` and `sandbox`):

```rust
if let Some(ref runner) = services.hook_runner {
    let hook_ctx = HookContext::new(
        HookEvent::ParallelStart,
        context.run_id(),
        graph.name.clone(),
    );
    let _ = runner
        .run(&hook_ctx, services.sandbox.clone(), None)
        .await;
}
```

### 3. Add ParallelComplete hook call in `parallel.rs`

Same file, after `ParallelCompleted` event emission (~line 713). Same pattern as ParallelStart but with `HookEvent::ParallelComplete`.

### 4. Mark SandboxReady/SandboxCleanup as reserved

File: `lib/crates/fabro-workflows/src/hook/types.rs`

Add doc comments:
```rust
/// Reserved: hooks for this event are not yet invoked by the engine.
SandboxReady,
/// Reserved: hooks for this event are not yet invoked by the engine.
SandboxCleanup,
```

### 5. Update docs

File: `docs/agents/hooks.mdx`

In the lifecycle events table, annotate `sandbox_ready` and `sandbox_cleanup` rows — e.g., append "(not yet wired)" to the description or add a note.

## Verification

1. `cargo fmt --check --all`
2. `cargo clippy --workspace -- -D warnings`
3. `cargo test --workspace` — existing tests pass, no regressions
4. Manual test: create a workflow with a retrying node and a `post_tool_use`-style hook for `stage_retrying`, verify it fires in the CLI log


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
  - Model: claude-opus-4-6, 33.1k tokens in / 7.2k out
  - Files: /home/daytona/workspace/lib/crates/fabro-workflows/src/engine.rs, /home/daytona/workspace/lib/crates/fabro-workflows/src/handler/parallel.rs
- **simplify_opus**: success
  - Model: claude-opus-4-6, 36.9k tokens in / 12.8k out
  - Files: /home/daytona/workspace/lib/crates/fabro-workflows/src/engine.rs
- **simplify_gemini**: success
  - Model: claude-opus-4-6, 39.9k tokens in / 11.2k out
  - Files: /home/daytona/workspace/lib/crates/fabro-workflows/src/handler/mod.rs, /home/daytona/workspace/lib/crates/fabro-workflows/src/handler/parallel.rs


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