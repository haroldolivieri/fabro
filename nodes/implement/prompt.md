Goal: # Refactor: Collapse `pull_request_*` fields on `RunConfig` into `Option<PullRequestConfig>`

## Context

`RunConfig` has 4 flat `pull_request_*` fields that encode a natural tree structure. This refactor collapses them into a single `Option<PullRequestConfig>` field, reusing the existing TOML config struct directly.

## Approach

Replace on `RunConfig`:
```rust
pub pull_request_enabled: bool,
pub pull_request_draft: bool,
pub pull_request_auto_merge: bool,
pub pull_request_merge_strategy: MergeStrategy,
```

With:
```rust
pub pull_request: Option<PullRequestConfig>,
```

- `None` = disabled (replaces `enabled: false`)
- `Some(config)` = enabled, read `.draft`, `.auto_merge`, `.merge_strategy` directly
- The `enabled` field on `PullRequestConfig` is still needed for TOML deserialization but is redundant at runtime

Delete `AutoMergeConfig` from `pull_request.rs` — pass `Option<MergeStrategy>` directly instead (derived from `auto_merge` + `merge_strategy` on `PullRequestConfig`).

## Files to modify

1. **`engine.rs`** — replace 4 fields with `pull_request: Option<PullRequestConfig>`, update all test constructors (4 lines → 1: `pull_request: None`)
2. **`run.rs`** — simplify construction: `pull_request: pr_cfg.filter(|p| p.enabled).cloned()` with draft override for auto_merge. Simplify read site: `if let Some(ref pr) = config.pull_request { ... }`
3. **`pull_request.rs`** — change `maybe_open_pull_request` to take `&PullRequestConfig` instead of `draft: bool` + `auto_merge: Option<AutoMergeConfig>`. Delete `AutoMergeConfig`
4. **`pr.rs`** — update call to `maybe_open_pull_request`
5. **`server.rs`, `manager_loop.rs`** — `pull_request: None`
6. **`tests/integration.rs`, `tests/daytona_integration.rs`** — `pull_request: None`

## Verification

- `cargo build --workspace`
- `cargo clippy --workspace -- -D warnings`
- `cargo test -p fabro-workflows -- pull_request run_config project_config`
- `cargo fmt --check --all`


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