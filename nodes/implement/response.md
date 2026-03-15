All checks pass. Let me provide a summary of all changes made:

## Summary of Changes

### 1. `lib/crates/fabro-workflows/src/cli/run_config.rs` — Add `GitHubConfig` struct + wire through config

- **New struct** `GitHubConfig` with `permissions: HashMap<String, String>` (after `AssetsConfig`)
- **Added** `pub github: Option<GitHubConfig>` to `WorkflowRunConfig`
- **Added** `pub github: Option<GitHubConfig>` to `RunDefaults`
- **`apply_defaults()`**: Added fallthrough: `if self.github.is_none() { self.github = defaults.github.clone(); }`
- **`merge_overlay()`**: Added replace: `if overlay.github.is_some() { self.github = overlay.github; }`
- **6 new tests**: `parse_toml_with_github_permissions`, `parse_toml_without_github_defaults_none`, `apply_defaults_github_inherited`, `apply_defaults_github_task_wins`, `merge_overlay_github_replaces`, `merge_overlay_github_none_keeps_base`

### 2. `lib/crates/fabro-workflows/src/cli/project_config.rs` — Add `github` to `ProjectConfig`

- **Added** `GitHubConfig` to import
- **Added** `pub github: Option<GitHubConfig>` field to `ProjectConfig`
- **Added** `github: self.github` to `into_run_defaults()`
- **2 new tests**: `parse_project_config_with_github`, `into_run_defaults_preserves_github`

### 3. `lib/crates/fabro-github/src/lib.rs` — Make function pub

- Changed `async fn create_installation_access_token_with_permissions` → `pub async fn`

### 4. `lib/crates/fabro-workflows/src/cli/run.rs` — Mint token and inject into sandbox env

- **New helper** `mint_github_token()` — signs JWT, resolves owner/repo from origin URL, requests scoped IAT
- **Token injection**: After building `sandbox_env`, checks `run_cfg.github` (falling back to `run_defaults.github`), mints token, inserts `GITHUB_TOKEN`
- **Preflight check**: New "GitHub Token" check that mints a token to validate credentials/permissions during `--preflight`
- Updated all `WorkflowRunConfig` struct literals in tests to include `github: None`

### 5. Supporting files

- **`docs/api-reference/fabro-api.yaml`**: Added `github` property to `ServerConfiguration` schema and new `GitHubConfiguration` schema
- **`lib/crates/fabro-api/src/demo/mod.rs`**: Added `github: None` to all `WorkflowRunConfig` struct literals
- **`lib/crates/fabro-api/tests/openapi_conformance.rs`**: Added `github: None` to `RunDefaults` struct literal