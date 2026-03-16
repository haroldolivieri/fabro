All changes are complete and verified. Here's a summary of what was done:

### Changes Made

1. **`lib/crates/fabro-workflows/src/engine.rs`** — Added `StageRetrying` hook calls at both retry sites in `execute_with_retry`:
   - **Site A (error-retry path, ~line 1128):** After `WorkflowRunEvent::StageRetrying` emission, before `tokio::time::sleep(delay).await`
   - **Site B (explicit Retry status path, ~line 1165):** Same pattern, after the event emission and before the sleep

2. **`lib/crates/fabro-workflows/src/handler/parallel.rs`** — Added `ParallelStart` and `ParallelComplete` hook calls:
   - Added `use crate::hook::{HookContext, HookEvent};` import
   - After `ParallelStarted` event emission: calls `runner.run()` with `HookEvent::ParallelStart`
   - After `ParallelCompleted` event emission: calls `runner.run()` with `HookEvent::ParallelComplete`

3. **`lib/crates/fabro-workflows/src/hook/types.rs`** — Added `/// Reserved: hooks for this event are not yet invoked by the engine.` doc comments to `SandboxReady` and `SandboxCleanup` variants

4. **`docs/agents/hooks.mdx`** — Annotated `sandbox_ready` and `sandbox_cleanup` rows with "(reserved — not yet wired)"

### Verification
- `cargo fmt --check --all` ✅
- `cargo clippy --workspace -- -D warnings` ✅  
- `cargo test --workspace` ✅ (179 passed, 0 failed, 13 ignored)