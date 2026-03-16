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


Read the plan file referenced in the goal and implement every step. Make all the code changes described in the plan. Use red/green TDD.