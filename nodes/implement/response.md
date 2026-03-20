Here's a summary of all changes made:

### Files changed

1. **`lib/crates/fabro-cli/src/commands/wait.rs`** (new file) — The `fabro wait` command implementation:
   - `WaitArgs` struct with `run`, `--timeout`, `--interval` (default 1000ms), and `--json` flags
   - `run()` function that resolves the run via `resolve_run()`, polls `status.json` via `RunStatusRecord::load()` every `--interval` ms, detects terminal status, reads `conclusion.json` for summary data, prints human-readable output to stderr (or JSON to stdout with `--json`), exits 0 for `Succeeded` and exits 1 for `Failed`/`Dead`
   - `build_json_output()` — builds JSON with run_id, status, duration_ms, and total_cost
   - `print_human_output()` — colored status line with duration and cost
   - 7 unit tests covering JSON output (with/without conclusion, various statuses), human output, poll behavior, and missing status fallback to Dead

2. **`lib/crates/fabro-cli/src/commands/mod.rs`** — Added `pub mod wait;`

3. **`lib/crates/fabro-cli/src/main.rs`** — Three insertions:
   - `Wait(commands::wait::WaitArgs)` variant in the `Command` enum
   - `Command::Wait(_) => "wait"` in the command name mapping
   - `Command::Wait(args) => { ... }` dispatch in the execution block