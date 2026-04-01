Goal: # Add `--force` flag to `fabro pr create`

## Context
`fabro pr create` rejects runs with non-success status. Users sometimes want to create PRs for failed runs (e.g. partial work worth reviewing). A `--force` flag bypasses the status check.

## Changes

### 1. Add `--force` flag to `PrCreateArgs` (`lib/crates/fabro-cli/src/args.rs:582-588`)
Add `#[arg(short, long)] pub(crate) force: bool` to `PrCreateArgs`.

### 2. Pass `force` through and skip status check (`lib/crates/fabro-cli/src/commands/pr/create.rs:76-79`)
Replace the hard bail with a warning when `--force` is set:
```rust
match conclusion.status {
    StageStatus::Success | StageStatus::PartialSuccess => {}
    status if args.force => {
        tracing::warn!("Run status is '{status}', proceeding because --force was specified");
    }
    status => bail!("Run status is '{status}', expected success or partial_success"),
}
```

### 3. Add `setup_failed_run` helper (`lib/crates/fabro-cli/tests/it/cmd/support.rs`)
New helper that runs a real (non-dry-run) workflow with a `shape=parallelogram, script="exit 1"` node. This produces a genuine `conclusion.json` with `status: "fail"`. Pattern follows `run_local_workflow` — uses `--sandbox local --provider openai` with `OPENAI_API_KEY=test`. The helper won't assert CLI exit success since the workflow fails; instead it finds the run dir via `only_run`.

### 4. Add integration tests (`lib/crates/fabro-cli/tests/it/cmd/pr_create.rs`)

**a) `pr_create_failed_run_rejects_without_force`** — `setup_failed_run`, run `pr create <run_id>`, assert error "Run status is 'fail', expected success or partial_success"

**b) `pr_create_failed_run_proceeds_with_force`** — `setup_failed_run`, run `pr create --force <run_id>`, assert it passes status check and hits next validation error ("Run has no run_branch"). Proves `--force` bypassed the status gate.

## Verification
- `cargo clippy -p fabro-cli -- -D warnings`
- `cargo nextest run -p fabro-cli`
- `./target/debug/fabro pr create --help` — confirm `-f`/`--force` appears


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