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


Read the plan file referenced in the goal and implement every step. Make all the code changes described in the plan. Use red/green TDD.