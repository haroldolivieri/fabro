Goal: # Fix: Workflow TOML config lost in detach mode

## Context

When running `fabro run -d implement-plan`, the `[pull_request]` config from `fabro.toml` / `cli.toml` / `workflow.toml` is silently dropped, so no PR is created despite `enabled = true` at all config levels.

**Root cause chain**:
1. `create.rs` tries to copy the workflow TOML as `run.toml`, but checks the **raw CLI arg** (`"implement-plan"` — no extension) instead of the resolved path. So `run.toml` is never saved.
2. `RunEngine` always uses cached `graph.fabro` (a DOT file), so `prepare_workflow` returns `run_cfg = None`, losing all TOML-level config.
3. `pull_request` and `asset_globs` in `RunConfig` only check `run_cfg` without falling back to `run_defaults`.

## Fix 1: Serialize merged `run_cfg` to `run.toml` in create.rs

**File**: `lib/crates/fabro-cli/src/commands/create.rs`

Replace the raw-file-copy block (lines 53-58) with serialization of the already-merged `WorkflowRunConfig`:

- Change `let prep = prepare_workflow(...)` to `let mut prep = ...`
- Replace the extension check with:
  ```rust
  if let Some(mut cfg) = prep.run_cfg.take() {
      cfg.graph = "graph.fabro".to_string();
      let toml_str = toml::to_string_pretty(&cfg)
          .context("Failed to serialize run config")?;
      tokio::fs::write(run_dir.join("run.toml"), toml_str).await?;
  }
  ```
- Add `use anyhow::Context;` if needed

**Why serialize instead of copy**: The raw TOML's `graph` field (e.g. `"workflow.fabro"`) would point to a nonexistent file in the run dir. Serializing lets us rewrite `graph` to `"graph.fabro"` (the cached name). The serialized config also has all defaults merged, env vars resolved, and dockerfiles inlined — making the run dir self-contained.

**Why `take()` not `clone()`**: `WorkflowRunConfig` doesn't derive `Clone`, and `prep.run_cfg` is unused after this point in `create.rs`.

## Fix 2: Use `run.toml` in RunEngine path

**File**: `lib/crates/fabro-cli/src/main.rs` (lines 727-733)

Replace the workflow path resolution to use `run.toml`:

```rust
let cached_toml = run_dir.join("run.toml");
let workflow_path = if cached_toml.exists() {
    cached_toml
} else {
    run_dir.join("graph.fabro")
};
```

When `run.toml` exists, `prepare_workflow` → `resolve_workflow` sees `.toml`, calls `load_run_config`, and `resolve_graph_path` resolves `"graph.fabro"` relative to the run dir — pointing to the cached graph that already exists there.

## Fix 3: Add `run_defaults` fallbacks (defense-in-depth)

**File**: `lib/crates/fabro-cli/src/commands/run.rs`

Even with Fixes 1+2, bare `.fabro` files passed directly would still hit `run_cfg = None`. Add fallbacks matching the pattern already used elsewhere in the file:

**3a. `pull_request`** (line 1424-1428):
```rust
pull_request: run_cfg
    .as_ref()
    .and_then(|c| c.pull_request.as_ref())
    .or(run_defaults.pull_request.as_ref())
    .filter(|p| p.enabled)
    .cloned(),
```

**3b. `asset_globs`** (line 1429-1433):
```rust
asset_globs: run_cfg
    .as_ref()
    .and_then(|c| c.assets.as_ref())
    .or(run_defaults.assets.as_ref())
    .map(|a| a.include.clone())
    .unwrap_or_default(),
```

**3c. `devcontainer`** (line 969-973):
```rust
let devcontainer_config = if run_cfg
    .as_ref()
    .and_then(|c| c.sandbox.as_ref())
    .or(run_defaults.sandbox.as_ref())
    .and_then(|s| s.devcontainer)
    .unwrap_or(false)
```

**3d. `sandbox.env`** (line 1300-1306):
```rust
if let Some(toml_env) = run_cfg
    .as_ref()
    .and_then(|c| c.sandbox.as_ref())
    .or(run_defaults.sandbox.as_ref())
    .and_then(|s| s.env.clone())
```

## Verification

1. `cargo build --workspace`
2. `cargo test --workspace`
3. `cargo clippy --workspace -- -D warnings`
4. Manual: `fabro run -d implement-plan` with `[pull_request] enabled = true` in `fabro.toml` → verify `run.toml` in run dir has `graph = "graph.fabro"` and `[pull_request]` → verify PR created


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
  - Model: claude-opus-4-6, 20.7k tokens in / 5.3k out
  - Files: /home/daytona/workspace/lib/crates/fabro-cli/src/commands/create.rs, /home/daytona/workspace/lib/crates/fabro-cli/src/commands/run.rs, /home/daytona/workspace/lib/crates/fabro-cli/src/main.rs


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