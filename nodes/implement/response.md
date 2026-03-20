Here's a summary of all changes made:

## Summary of Changes

### 1. `lib/crates/fabro-tracker/Cargo.toml` — Updated dependencies
Added `fabro-github`, `serde`, `serde_json`, `reqwest`, `tracing`, `tokio` as dependencies, and `mockito`/`tokio` as dev-dependencies to support both tracker implementations.

### 2. `lib/crates/fabro-tracker/src/lib.rs` — Added module declarations and re-exports
Added `pub mod github;`, `pub mod linear;`, and re-exports: `GitHubTracker`, `LinearConfig`, `LinearTracker`, `LINEAR_API_ENDPOINT`.

### 3. `lib/crates/fabro-tracker/src/github.rs` — New file (moved from fabro-github)
Contains `GitHubTracker` struct + impl, `execute_github_graphql`, `normalize_github_item`, `fetch_project_items_page`, and all GitHub tracker tests. Imports auth primitives from `fabro_github` and types from `crate::`.

### 4. `lib/crates/fabro-tracker/src/linear.rs` — New file (adapted from fabro-linear)
Contains `LinearTracker` struct implementing `Tracker` trait, `LinearConfig`, all Linear API functions, and all tests. Uses `crate::{BlockerRef, Issue}` instead of duplicate struct definitions. `normalize_issue` now sets `project_item_id: None`.

### 5. `lib/crates/fabro-github/Cargo.toml` — Removed tracker dependencies
Removed `fabro-tracker` and `async-trait` dependencies.

### 6. `lib/crates/fabro-github/src/lib.rs` — Removed tracker code
Removed: `use async_trait`, `use tokio::sync::OnceCell`, `pub use fabro_tracker::*`, `execute_github_graphql`, `GitHubTracker` struct/impls, `normalize_github_item`, `fetch_project_items_page`, and all tracker-related tests. Kept `create_installation_access_token_for_projects` and all non-tracker code.

### 7. Deleted `lib/crates/fabro-linear/` — Entire directory removed
Workspace glob `members = ["lib/crates/*"]` handles the workspace membership automatically.

### Dependency direction change
- **Before:** `fabro-github → fabro-tracker` 
- **After:** `fabro-tracker → fabro-github` (no circular dependency)