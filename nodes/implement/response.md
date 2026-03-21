All four changes are in place. Let me summarize what was done:

## Summary of changes

### Fix 1: `lib/crates/fabro-cli/src/commands/create.rs`
- Added `use anyhow::Context;`
- Changed `let prep` to `let mut prep` to allow taking `run_cfg`
- Replaced the raw-file-copy block (which checked the CLI arg's extension — always failing for bare workflow names like `"implement-plan"`) with serialization of the already-merged `WorkflowRunConfig`. The serialized config rewrites `graph` to `"graph.fabro"` (the cached name in the run dir), making the run directory self-contained.

### Fix 2: `lib/crates/fabro-cli/src/main.rs` (RunEngine path)
- Changed the workflow path resolution in the `RunEngine` command to prefer `run.toml` over `graph.fabro`. When `run.toml` exists, `prepare_workflow` → `resolve_workflow` sees `.toml`, calls `load_run_config`, and resolves `"graph.fabro"` relative to the run dir — restoring all TOML-level config (pull_request, assets, etc.).

### Fix 3: `lib/crates/fabro-cli/src/commands/run.rs` (defense-in-depth fallbacks)
- **3a. `pull_request`**: Added `.or(run_defaults.pull_request.as_ref())` fallback
- **3b. `asset_globs`**: Added `.or(run_defaults.assets.as_ref())` fallback
- **3c. `devcontainer`**: Added `.or(run_defaults.sandbox.as_ref())` fallback
- **3d. `sandbox.env`**: Added `.or(run_defaults.sandbox.as_ref())` fallback

All changes compile cleanly, pass all `fabro-cli` tests, and pass clippy with `-D warnings`.