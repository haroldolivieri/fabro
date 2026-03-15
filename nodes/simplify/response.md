All tests pass, clippy clean, fmt clean. Here's a summary:

## Summary of fixes

### Fixed
1. **Missing trailing newlines** in `docs/workflows/transitions.mdx` and `docs/reference/dot-language.mdx` — both files ended without a final newline, which `git diff` flagged as `\ No newline at end of file`. Added the trailing newlines.

2. **No validation for unknown `selection` attribute values** — A typo like `selection="randon"` would silently fall through to deterministic behavior via the `_ =>` catch-all in `pick_edge`. Added `SelectionValidRule` (following the existing `DirectionValidRule` pattern) that emits a Warning when a node has an unrecognized `selection` value. Added 3 tests for the new rule.

3. **Formatting** — `cargo fmt` fixed minor formatting in the new rule code.

### Not fixed (false positives / not worth addressing)
- **5th parameter on `select_edge`**: Could be derived from `node_id + graph`, but the explicit parameter is better for testability (tests pass `"random"`/`"deterministic"` directly without building full node objects). Consistent with how the function already receives `node_id` separately from `graph`.
- **`weighted_random` could use `rand::distributions::WeightedIndex`**: The hand-rolled version is clear, short, and correct. `WeightedIndex` would save ~5 lines but add an import and slightly different error handling. Not worth the churn.
- **`Vec<f64>` allocation in `weighted_random`**: For typical edge counts (2-5), this is negligible overhead on a path that's dominated by LLM calls. No concern.
- **Stringly-typed `selection`**: The codebase consistently uses string attributes for similar features (`retry_policy`, `backend`, `fidelity`). The new `SelectionValidRule` catches typos at validation time, making this safe.
- **Doc comment placement on `EdgeSelection`**: Pre-existing issue, not introduced by this diff.