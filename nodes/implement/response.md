The diff looks exactly right. Let me summarize all the changes made:

## Summary of changes

### 1. `lib/crates/fabro-daytona/src/lib.rs` — Core refactor
- **Added `GitCloneParams` struct** with `url: String` and `branch: Option<String>` fields, publicly exported
- **Changed `DaytonaSandbox` field** from `clone_branch: Option<String>` to `clone_params: Option<GitCloneParams>`
- **Updated `new()` signature**: last param changed from `clone_branch: Option<String>` to `clone_params: Option<GitCloneParams>`
- **Updated `reconnect()`**: `clone_params: None` (was `clone_branch: None`)
- **Refactored `initialize()`**:
  - Removed `let cwd = std::env::current_dir()` — no longer reads the process-global cwd
  - Replaced `match detect_repo_info(&cwd)` with `if let Some(ref clone_params) = self.clone_params`
  - `Some` arm: uses `clone_params.url` / `clone_params.branch` directly (URL is already HTTPS from caller)
  - `else` arm: creates empty working directory (the old `Err` arm logic)
- **Removed unused import** `use fabro_github::ssh_url_to_https`

### 2. `lib/crates/fabro-cli/src/commands/run.rs` — Production callers
- **Main `run` path**: Constructs `GitCloneParams` from `origin_url` (with `ssh_url_to_https`) and `detected_base_branch`, passes it to `DaytonaSandbox::new()`
- **Doctor path**: Already passes `None` — type changed from `Option<String>` to `Option<GitCloneParams>` but `None` is valid for both

### 3. `lib/crates/fabro-workflows/tests/daytona_integration.rs` — Test fixes
- **`create_env_with_github_app`**: Detects repo info with `detect_repo_info(&cwd)` and builds `GitCloneParams` before calling `new()`, preserving clone behavior for all tests that use this helper
- **`daytona_computer_use_browser_screenshot`**: Removed `tempfile::tempdir()` and `set_current_dir()` — passes `None` as last arg which now cleanly means "skip clone"
- **`daytona_playwright_mcp_sandbox_transport`**: Same — removed `tempfile::tempdir()` and `set_current_dir()`