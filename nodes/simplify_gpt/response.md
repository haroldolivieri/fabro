Good — it doesn't exist (was never present, the goal doc mentioned it as a future step but it wasn't created in the first place).

## Summary

The code is already clean. After reviewing all changes across the three dimensions:

**Code Reuse:** No issues found. The `.and_then().filter().cloned()` chain is idiomatic Rust and doesn't duplicate any existing utility. The pattern is simple enough not to warrant extraction.

**Code Quality:** No issues found. The refactoring is clean:
- `None` = disabled, `Some(config)` = enabled — this correctly eliminates the redundant boolean at the `RunConfig` level
- The `enabled` field on `PullRequestConfig` is retained only for TOML deserialization (as designed), and filtered out via `.filter(|p| p.enabled)` during construction
- The `maybe_open_pull_request` function correctly keeps `draft: bool` as a parameter since it's also called from `pr.rs` where no `PullRequestConfig` is available
- The restructured `if let Some(ref pr_cfg)` block in `run.rs` is cleaner than the original flat if-else chain

**Efficiency:** No issues found. The `.cloned()` on `PullRequestConfig` (which is just `bool` + `bool`) is negligible. No new file reads, no redundant work, no hot-path bloat.

All verification checks pass: build ✅, clippy ✅, tests ✅, fmt ✅.