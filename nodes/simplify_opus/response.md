Here's a summary of the review and what was fixed:

## Review Summary

### Critical Bug Found and Fixed

The `implement` step moved the `StageStarted` emission entirely from the main run loop into `execute_with_retry`. This broke the existing integration test `hook_stage_start_skip_bypasses_node` because when a `StageStart` hook skips a node, `execute_with_retry` is never called — so `StageStarted` was never emitted for skipped nodes.

**Fix:** Split the emission into two sites:
1. **Main loop** (before the hook): Emits `StageStarted` with `attempt: 1` — ensures skipped nodes still get the event
2. **Retry loop** (inside `execute_with_retry`): Emits `StageStarted` only for `attempt > 1` — provides the spinner restart that was the original goal

### Review Agent Findings (no additional changes needed)

- **Code Reuse**: The event-collection pattern in the new test (`Arc<Mutex<Vec<WorkflowRunEvent>>>` + `on_event` closure) follows the existing convention used in 5+ other tests. No helper exists to extract, and introducing one would be over-engineering for test code.
- **Code Quality**: The production change is clean — no redundant state, no parameter sprawl, no copy-paste issues. The `usize::try_from(u32).unwrap_or(usize::MAX)` is technically infallible on all supported platforms but matches the existing codebase convention.
- **Efficiency**: The string allocations inside the retry loop (`node.label().to_string()`, `node_script(node)`, etc.) are negligible since `max_attempts` is typically 1-3 and these are dominated by actual handler execution time (LLM calls, commands, etc.).

### Verification
- `cargo test -p fabro-workflows` — all **179 tests pass** (including the previously failing `hook_stage_start_skip_bypasses_node`)
- `cargo clippy --workspace -- -D warnings` — clean, no warnings