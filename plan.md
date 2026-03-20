# Plan: Use short hex IDs for subagents instead of UUIDs

## Issue Summary

GitHub Issue #126: Subagent IDs are currently full UUID v4 strings (36 chars, e.g. `550e8400-e29b-41d4-a716-446655440000`). These appear in CLI output and must be passed back by the LLM to tools like `send_input`, `wait`, and `close_agent`. A shorter, random 8-character hex string (e.g. `a3f1b20c`) is easier to read and less error-prone. The codebase already truncates agent IDs to 8 chars in multiple display locations — this change makes the canonical ID match what's already shown.

## Files to Modify

1. **`lib/crates/fabro-agent/Cargo.toml`** — Add `rand` dependency (already a workspace dep).
2. **`lib/crates/fabro-agent/src/subagent.rs`** — Replace UUID generation with 8-char hex.
3. **`lib/crates/fabro-agent/src/cli.rs`** — Remove 5 instances of `short_id` truncation; use `agent_id` directly.
4. **`lib/crates/fabro-cli/src/commands/run_progress.rs`** — Remove 2 instances of `short_id` truncation; use `agent_id` directly.

No files need to be created.

## Step-by-step Implementation

### Step 1: Add `rand` to `fabro-agent` dependencies

In `lib/crates/fabro-agent/Cargo.toml`, add `rand.workspace = true` to `[dependencies]`. The `rand = "0.8"` workspace dependency already exists in the root `Cargo.toml`.

Note: Do **not** remove `uuid` — it is still used for session IDs in `session.rs:58`.

### Step 2: Replace UUID with random hex in `subagent.rs`

In `lib/crates/fabro-agent/src/subagent.rs`, line 67, change:

```rust
let agent_id = uuid::Uuid::new_v4().to_string();
```

to:

```rust
let agent_id = format!("{:08x}", rand::random::<u32>());
```

This generates an 8-character lowercase hex string from a random `u32` (~4 billion possible values, effectively collision-free within a session).

### Step 3: Remove `short_id` truncation in `cli.rs`

In `lib/crates/fabro-agent/src/cli.rs`, there are 5 occurrences of `let short_id = &agent_id[..8.min(agent_id.len())];` (lines 542, 561, 574, 583, 596). Since the agent ID is now exactly 8 chars, these truncation lines are redundant. For each occurrence:

- Remove the `let short_id = ...` line.
- Replace all uses of `short_id` with `agent_id` in the surrounding format strings.

The 5 locations are inside match arms for:
1. `AgentEvent::SubAgentSpawned` (line 542)
2. `AgentEvent::SubAgentCompleted` (line 561)
3. `AgentEvent::SubAgentFailed` (line 574)
4. `AgentEvent::SubAgentClosed` (line 583)
5. `AgentEvent::SubAgentEvent` (line 596, inside `if verbose` guard)

### Step 4: Remove `short_id` truncation in `run_progress.rs`

In `lib/crates/fabro-cli/src/commands/run_progress.rs`, there are 2 occurrences of `let short_id = &agent_id[..agent_id.len().min(8)];` (lines 1416, 1432). For each:

- Remove the `let short_id = ...` line.
- Replace `short_id` with `agent_id` in the surrounding format strings.

The 2 locations are inside match arms for:
1. `AgentEvent::SubAgentSpawned` (line 1416)
2. `AgentEvent::SubAgentCompleted` (line 1432)

## Verification / Test Cases

No new tests are needed. Existing tests use hardcoded IDs (like `"sa-1"`, `"nonexistent-id"`, `"x"`) and don't depend on the UUID format. The spawn test (`spawn_creates_agent_and_returns_id`) asserts the ID is non-empty, which still passes.

Run these commands to verify:

1. `cargo test -p fabro-agent` — all subagent tests pass
2. `cargo test -p fabro-cli` — run_progress tests pass
3. `cargo clippy --workspace -- -D warnings` — no warnings
4. `cargo build --workspace` — clean build
