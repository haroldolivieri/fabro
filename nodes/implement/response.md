Everything compiles cleanly. Let me provide a summary of all changes made:

## Summary of Changes

### Refactor: Collapsed `pull_request_*` fields on `RunConfig` into `Option<PullRequestConfig>`

**1. `lib/crates/fabro-workflows/src/engine.rs`**
- Added `use crate::cli::run_config::PullRequestConfig;` import
- Replaced two fields (`pull_request_enabled: bool`, `pull_request_draft: bool`) with single `pull_request: Option<PullRequestConfig>` field
- Updated all ~40 test constructors: `pull_request_enabled: false, pull_request_draft: false` → `pull_request: None`

**2. `lib/crates/fabro-workflows/src/cli/run.rs`**
- Simplified `RunConfig` construction: replaced `pr_cfg.is_some_and(|p| p.enabled)` / `pr_cfg.is_none_or(|p| p.draft)` with `.filter(|p| p.enabled).cloned()`
- Restructured PR creation block: `if let Some(ref pr_cfg) = config.pull_request { ... }` instead of `is_none()` check + later `unwrap()`
- Updated `maybe_open_pull_request` call to pass `pr_cfg` reference
- Updated event emission to use `pr_cfg.draft`
- Updated resume config constructor: `pull_request: None`

**3. `lib/crates/fabro-workflows/src/pull_request.rs`**
- Changed `maybe_open_pull_request` signature: replaced `draft: bool` parameter with `pr_config: &crate::cli::run_config::PullRequestConfig`
- Updated internal usage to read `pr_config.draft`
- Updated test to construct and pass `PullRequestConfig`

**4. `lib/crates/fabro-workflows/src/cli/pr.rs`**
- Updated call to `maybe_open_pull_request`: constructs `PullRequestConfig { enabled: true, draft: true }` and passes reference

**5. `lib/crates/fabro-api/src/server.rs`**
- Replaced `pull_request_enabled: false, pull_request_draft: true` with `pull_request: None`

**6. `lib/crates/fabro-workflows/src/handler/manager_loop.rs`**
- Replaced `pull_request_enabled: false, pull_request_draft: true` with `pull_request: None`

**7. `lib/crates/fabro-workflows/tests/integration.rs`**
- Replaced all ~90 occurrences (8-space and 12-space indented) of `pull_request_enabled: false, pull_request_draft: false` with `pull_request: None`

**8. `lib/crates/fabro-workflows/tests/daytona_integration.rs`**
- Replaced all occurrences with `pull_request: None`

### Verification
- ✅ `cargo build --workspace` — clean
- ✅ `cargo clippy --workspace -- -D warnings` — clean
- ✅ `cargo test -p fabro-workflows -- pull_request run_config project_config` — 180 tests passed
- ✅ `cargo fmt --check --all` — clean