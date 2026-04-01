All changes are clean and focused. Here's a summary of what was implemented:

### Changes Made

1. **`lib/crates/fabro-cli/src/args.rs`** — Added `#[arg(short, long)] pub(crate) force: bool` to `PrCreateArgs`, with help text explaining it bypasses the status check.

2. **`lib/crates/fabro-cli/src/commands/pr/create.rs`** — Updated the status check `match` to include a new arm `status if args.force =>` that logs a warning and continues instead of bailing.

3. **`lib/crates/fabro-cli/tests/it/cmd/support.rs`** — Added `setup_failed_run` helper that creates a workspace with a workflow containing a `shape=parallelogram, script="exit 1", goal_gate=true` node, runs it, and asserts the conclusion status is "fail".

4. **`lib/crates/fabro-cli/tests/it/cmd/pr_create.rs`** — Added two integration tests:
   - `pr_create_failed_run_rejects_without_force` — verifies failed runs are rejected with the expected error message
   - `pr_create_failed_run_proceeds_with_force` — verifies `--force` bypasses the status gate and hits the next validation error ("Run has no run_branch")
   - Updated the `help` snapshot to include the new `-f, --force` flag

### Verification
- `cargo clippy -p fabro-cli -- -D warnings` — passes clean
- `cargo nextest run -p fabro-cli -- pr_create` — all 5 tests pass
- `./target/debug/fabro pr create --help` — confirms `-f`/`--force` appears