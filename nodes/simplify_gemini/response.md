All clean. Here's a summary of what was fixed:

## Summary

**Two issues fixed:**

1. **Correctness: Parallel hooks now populate `node_id`/`node_label`/`handler_type`** — The `ParallelStart` and `ParallelComplete` hook contexts were missing node metadata that all other stage-level hooks provide. This meant user-defined hook matchers filtering on `node_id` or `handler_type` would silently never match these events. Fixed by populating these fields from the available `node` reference.

2. **Abstraction: Extracted `EngineServices::run_hooks()` helper** — The parallel handler was duplicating the `if let Some(ref runner) = services.hook_runner { runner.run(...) }` guard pattern that the engine encapsulates in its own `run_hooks()` method. Added a `run_hooks(&self, hook_context: &HookContext) -> HookDecision` method on `EngineServices` so handlers can call hooks through a clean interface without reaching into `hook_runner` internals. Both parallel hook sites now use `services.run_hooks(&hook_ctx)`.

**Skipped (not worth addressing):**
- `usize::try_from(u32).unwrap_or(usize::MAX)` — infallible on all real platforms, consistent with existing code
- Parameter sprawl in `stage_retrying_hook` — 5 params is borderline, all naturally in scope at call sites
- Retry-path hook latency — absorbed by backoff sleep, negligible
- `Arc::clone` costs — trivially cheap atomic increment