Reviewed and cleaned up the detach-mode fix.

What I changed
- `lib/crates/fabro-cli/src/commands/run.rs`
  - Added shared helpers/constants for cached run files:
    - `RUN_GRAPH_FILE`
    - `RUN_CONFIG_FILE`
    - `cached_graph_path()`
    - `cached_run_config_path()`
    - `write_run_config_snapshot()`
  - Removed duplicated `run.toml` serialization logic from both create/run paths.
  - Stopped mutating live `run_cfg` just to rewrite `graph`; snapshot serialization now rewrites the TOML via a serialized value instead.
  - Centralized detached-workflow resolution in `resolve_workflow_source()`, with fallback from missing cached `run.toml` to sibling `graph.fabro` for backward compatibility.
  - Reused the shared graph-path helper in the run-branch resume path too.
  - Added focused tests for:
    - snapshot serialization rewriting `graph`
    - fallback from missing cached `run.toml` to `graph.fabro`

- `lib/crates/fabro-cli/src/commands/create.rs`
  - Reused the shared cached-file helpers.
  - Removed now-unneeded mutability and `Context` import.

- `lib/crates/fabro-cli/src/main.rs`
  - Removed the `exists()` check.
  - Detached engine now always points at cached `run.toml`; `prepare_workflow()` handles legacy fallback cleanly.

Why this cleanup
- Fixes the code-quality issue of mutating runtime config for persistence.
- Removes copy-pasted snapshot-writing logic.
- Removes the TOCTOU-style `exists()` check in detached startup.
- Centralizes the cached run-dir file contract.

Validation
- `cargo fmt --all`
- `cargo test -p fabro-cli`
- `cargo clippy -p fabro-cli -- -D warnings`
- `cargo build --workspace`
- `cargo clippy --workspace -- -D warnings`
- `cargo test --workspace`

All passed.

I did not run the manual PR-creation flow, so that part remains unverified in this environment.