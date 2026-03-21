## Summary

**Fixed one real issue, accepted pre-existing snapshots, skipped false positives:**

### Fixed
1. **`run.rs` raw TOML copy → merged serialization** (the main fix): `run.rs` line 701-704 was still copying the raw TOML file from disk (the old buggy behavior), while `create.rs` had been fixed to serialize the merged config. This meant `fabro run` (non-detached) would write a `run.toml` with the wrong `graph` path and without defaults merged. Now both `create.rs` and `run.rs` use the same approach: serialize the merged `WorkflowRunConfig` with `graph` rewritten to `"graph.fabro"`.

2. **Accepted pre-existing snapshot updates**: 7 `fabro-model` catalog test snapshots needed updating for the new `effort` field in `ModelFeatures`. Removed the stale `.pending-snap` file.

### Skipped (not worth addressing)
- **Magic string constants for `"graph.fabro"`/`"run.toml"`**: Valid observation (18+ occurrences across 7+ files), but a cross-crate refactor beyond the scope of this fix.
- **Extract `resolve_sandbox_config` helper**: The two inline `.or()` additions match the existing code pattern. Extracting a helper for two call sites is over-abstraction.
- **No efficiency issues**: All three changes are on cold paths with negligible cost.