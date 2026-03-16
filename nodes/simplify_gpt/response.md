Clean. Here's a summary of what was fixed:

## Summary

**Added:** `HookContext::set_node(&mut self, node: &Node)` helper in `hook/types.rs` — populates `node_id`, `node_label`, and `handler_type` from a graph `Node`.

**Simplified 6 call sites** using the new helper:
- `engine.rs`: `stage_retrying_hook` (new code), `StageStart` hook, `StageFailed` hook, `StageComplete` hook
- `parallel.rs`: `ParallelStart` hook (new code), `ParallelComplete` hook (new code)

**Fixed import inconsistency** in `parallel.rs`: `HookContext` was used via full path (`crate::hook::HookContext`) while `HookEvent` was imported — now both are imported at the top.

**Skipped (not actionable):**
- `work_dir: None` in `EngineServices::run_hooks` and `stage_retrying_hook` — intentional per spec; these hooks fire in contexts where work_dir isn't available
- Unnecessary braces around hook blocks — matches existing convention throughout engine.rs
- Duplicate `run_hooks` on `EngineServices` vs `WorkflowRunEngine` — architecturally necessary since handlers don't have engine access
- No efficiency issues — hooks fire on non-hot paths (retry delays, parallel lifecycle)