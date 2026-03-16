Goal: # Plan: Move sandbox lifecycle into the engine

## Context

`SandboxReady` and `SandboxCleanup` hook events are defined in `HookEvent` but never fired. They can't fire naturally because sandbox init/cleanup happens in `cli/run.rs` — outside the engine where all other hooks fire. The root cause is that `cli/run.rs` is a ~2000-line god function orchestrating everything: sandbox creation, init, setup commands, devcontainer lifecycle, engine construction, engine run, retro, PR creation, and cleanup. The engine receives an already-initialized sandbox and has no role in lifecycle management.

This refactor moves sandbox initialization and setup into the engine so `SandboxReady` fires naturally alongside `RunStart`. Cleanup stays callable from the CLI (the retro agent needs the sandbox alive after the graph completes) but fires through an engine method so `SandboxCleanup` also uses the hook infrastructure.

## Design

### Two new engine methods

**`run_with_lifecycle(graph, config, lifecycle, checkpoint?) -> Result<Outcome>`**
1. `sandbox.initialize()`
2. Fire `SandboxReady` hook (blocking — can abort run)
3. Emit `SandboxInitialized` event (CLI listener writes `sandbox.json`, updates progress UI)
4. Remote git setup if `sandbox.is_remote()` — produces `base_sha`, `run_branch`, `base_branch`, merged into `config`
5. Run setup commands inside sandbox
6. Run devcontainer lifecycle phases inside sandbox
7. Call existing `run_internal()` (unchanged — fires RunStart → graph → RunComplete)
8. Return outcome (sandbox still alive)

**`cleanup_sandbox(run_id, workflow_name, preserve) -> Result<()>`**
1. Fire `SandboxCleanup` hook (non-blocking)
2. If `!preserve`: call `sandbox.cleanup()`

Existing `run()` is unchanged — API server and integration tests keep using it with pre-initialized sandboxes.

### New config struct

```rust
// engine.rs
pub struct LifecycleConfig {
    pub setup_commands: Vec<String>,
    pub setup_command_timeout_ms: u64,
    pub devcontainer_phases: Vec<(String, Vec<fabro_devcontainer::Command>)>,
}
```

`run_with_lifecycle` takes `config: RunConfig` by value (currently `&RunConfig`) so it can fill in remote git values. `run_internal` continues to take `&RunConfig`.

### CLI flow after refactor

```
sandbox = create_sandbox()                          // unchanged
sandbox = ReadBeforeWriteSandbox::new(sandbox)       // moved before init (delegate_sandbox! delegates initialize)
engine = build_engine(sandbox, hook_runner, ...)
outcome = engine.run_with_lifecycle(graph, config, lifecycle)
// retro, conclusion, PR creation — sandbox still alive
engine.cleanup_sandbox(run_id, workflow_name, preserve)
```

Single scopeguard around the entire block that calls `engine.cleanup_sandbox()` on panic.

## Steps

### 1. `hook/types.rs` — Make `SandboxReady` blocking by default
- Add `Self::SandboxReady` to `is_blocking_by_default()` match arm (line 29-32)
- `SandboxCleanup` stays non-blocking (correct default)

### 2. `engine.rs` — Add `LifecycleConfig` struct and `run_with_lifecycle` method
- Define `LifecycleConfig` (setup_commands, setup_command_timeout_ms, devcontainer_phases)
- Add `pub async fn run_with_lifecycle(self, graph, config, lifecycle, checkpoint) -> Result<Outcome>` that:
  - Calls `self.services.sandbox.initialize()`
  - Fires `SandboxReady` hook via `self.run_hooks()`
  - Emits `WorkflowRunEvent::SandboxInitialized { working_directory }` via emitter
  - Calls remote git setup if `sandbox.is_remote()`, fills config.base_sha/run_branch/base_branch/git_checkpoint_enabled
  - Runs setup commands via `sandbox.exec_command()`, emitting Setup* events
  - Runs devcontainer lifecycle via `devcontainer_bridge::run_devcontainer_lifecycle()`
  - Calls `self.run_internal()` (or `run_from_checkpoint` path)
  - Returns outcome

### 3. `engine.rs` — Add `cleanup_sandbox` method
- `pub async fn cleanup_sandbox(&self, run_id, workflow_name, preserve) -> Result<(), String>`
- Fires `SandboxCleanup` hook
- If `!preserve`: calls `self.services.sandbox.cleanup()`

### 4. `engine.rs` — Move `setup_remote_git` from `cli/run.rs`
- Move the `setup_remote_git()` function (cli/run.rs line 1636-1679) into engine.rs
- It only uses `sandbox.exec_command()` and `run_id` — no CLI dependencies

### 5. `event.rs` — Add `SandboxInitialized` event variant
- Add `WorkflowRunEvent::SandboxInitialized { working_directory: String }` variant
- This replaces the inline sandbox.json writing in cli/run.rs

### 6. `cli/run.rs` — Register event listener for sandbox.json
- Before calling `run_with_lifecycle`, register a listener on the emitter for `SandboxInitialized`
- Listener captures the pre-built `SandboxRecord` template (all provider-specific fields filled, `working_directory` empty)
- On event: fill `working_directory` from event, call `record.save()`
- Also update progress UI `set_working_directory` in the same listener

### 7. `cli/run.rs` — Refactor `run_command` to use new engine methods
- Move `ReadBeforeWriteSandbox` wrapping to before engine construction (currently at line 966, after init — delegate_sandbox! macro delegates initialize so wrapping before init works)
- Move `HookRunner` creation earlier (before engine construction) — currently line 1222, move to ~line 800
- Remove: `sandbox.initialize()` (line 886), remote git setup (lines 982-996), setup commands (lines 1031-1072), devcontainer lifecycle (lines 1074-1091)
- Build `LifecycleConfig` from `setup_commands` and `devcontainer_config`
- Build `RunConfig` without remote git fields (leave base_sha/run_branch/base_branch as None for remote — engine fills them)
- Call `engine.run_with_lifecycle()` instead of `engine.run()`
- Replace cleanup section (lines 1587-1604) with `engine.cleanup_sandbox()`
- Replace two scopeguards with one that calls `engine.cleanup_sandbox()` on panic
- Remove `status_guard` for SandboxInitFailed — engine handles init errors

### 8. `cli/run.rs` — Refactor `run_from_branch` to use new engine methods
- Use `run_with_lifecycle` with empty `LifecycleConfig` (no setup commands, no devcontainer)
- Add `cleanup_sandbox()` call (currently no scopeguard — this is an improvement)
- This gives the resume path hooks for free (currently has zero hooks)

### 9. `docs/agents/hooks.mdx` — Update docs
- Remove any "reserved" annotations for `sandbox_ready` / `sandbox_cleanup`
- Note that `sandbox_ready` is blocking by default

## Files to modify
- `lib/crates/fabro-workflows/src/hook/types.rs` — SandboxReady blocking default
- `lib/crates/fabro-workflows/src/engine.rs` — LifecycleConfig, run_with_lifecycle, cleanup_sandbox, setup_remote_git
- `lib/crates/fabro-workflows/src/event.rs` — SandboxInitialized event variant
- `lib/crates/fabro-workflows/src/cli/run.rs` — major simplification of run_command and run_from_branch
- `docs/agents/hooks.mdx` — remove "reserved" annotations

## Files unchanged
- `lib/crates/fabro-workflows/src/handler/mod.rs` — EngineServices unchanged
- `lib/crates/fabro-workflows/src/hook/runner.rs` — handles any HookEvent generically
- `lib/crates/fabro-agent/src/sandbox.rs` — Sandbox trait unchanged
- `lib/crates/fabro-agent/src/read_before_write_sandbox.rs` — delegate_sandbox! already delegates initialize/cleanup
- All sandbox implementations — unchanged

## Verification
1. `cargo build --workspace` — compile check
2. `cargo test --workspace` — all existing tests pass (existing `run()` path unchanged)
3. `cargo clippy --workspace -- -D warnings` — no new warnings
4. Manual test: `fabro run` with a workflow that has `sandbox_ready` and `sandbox_cleanup` hooks configured — verify hooks fire
5. Manual test: `fabro run --sandbox daytona` — verify remote git setup still works through the engine
6. Manual test: `fabro run` with `--preserve-sandbox` — verify cleanup is skipped but SandboxCleanup hook still fires


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
  - Model: claude-opus-4-6, 172.3k tokens in / 48.5k out
  - Files: /home/daytona/workspace/docs/agents/hooks.mdx, /home/daytona/workspace/lib/crates/fabro-workflows/src/cli/run.rs, /home/daytona/workspace/lib/crates/fabro-workflows/src/engine.rs, /home/daytona/workspace/lib/crates/fabro-workflows/src/event.rs, /home/daytona/workspace/lib/crates/fabro-workflows/src/hook/types.rs
- **simplify_opus**: success
  - Model: claude-opus-4-6, 90.5k tokens in / 16.9k out
  - Files: /home/daytona/workspace/lib/crates/fabro-workflows/src/cli/run.rs, /home/daytona/workspace/lib/crates/fabro-workflows/src/engine.rs
- **simplify_gemini**: success
  - Model: claude-opus-4-6, 92.0k tokens in / 18.0k out
  - Files: /home/daytona/workspace/lib/crates/fabro-workflows/src/cli/run.rs, /home/daytona/workspace/lib/crates/fabro-workflows/src/engine.rs


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