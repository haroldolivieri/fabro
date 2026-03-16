Clean and focused. Here's the summary:

## Summary

All three review agents found the same core issue: **leaky abstraction** — the TOML deserialization struct `PullRequestConfig` (with serde attributes and a vestigial `enabled` field) was being passed through runtime APIs where only a `draft: bool` was needed.

### Fixed

**Reverted `maybe_open_pull_request` parameter from `&PullRequestConfig` back to `draft: bool`** (3 files, net -8 lines):

1. **`pull_request.rs`** — Changed parameter from `pr_config: &crate::cli::run_config::PullRequestConfig` to `draft: bool`, and used `draft` directly instead of `pr_config.draft`. Removed unnecessary `PullRequestConfig` construction in the test.

2. **`pr.rs`** — Removed the 4-line `PullRequestConfig` struct construction that existed solely to wrap `draft: true`, now passes `true` directly.

3. **`run.rs`** — Changed call from `pr_cfg` to `pr_cfg.draft` at the single callsite.

### Not changed (false positives / not worth addressing)

- **`RunConfig.pull_request: Option<PullRequestConfig>`** — Keeping this is correct. The `PullRequestConfig` struct may gain more fields (e.g., `auto_merge`, `merge_strategy` per the goal), and `Option<PullRequestConfig>` correctly encodes enabled/disabled semantics at the `RunConfig` level.
- **The `enabled` field on `PullRequestConfig`** — Needed for TOML deserialization (`[pull_request]\nenabled = true`) and filtered out at the config→runtime boundary in `run.rs`. This is the correct layering.
- **No efficiency concerns** — `PullRequestConfig` is two bools; clone/allocation costs are negligible.