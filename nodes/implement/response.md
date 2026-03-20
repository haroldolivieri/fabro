All checks pass:
- ✅ `cargo build --workspace` — compiles
- ✅ `cargo test --workspace` — all tests pass (0 failures)
- ✅ `cargo clippy --workspace -- -D warnings` — clean
- ✅ `cargo fmt --check --all` — clean

Let me now provide a summary of everything that was implemented:

## Summary of Changes

### New Files Created

| File | Purpose |
|------|---------|
| `lib/crates/fabro-workflows/src/run_spec.rs` | `RunSpec` struct — serializable run configuration persisted as `spec.json` |
| `lib/crates/fabro-interview/src/file.rs` | `FileInterviewer` — file-based IPC for interviews between engine and attach processes |
| `lib/crates/fabro-cli/src/commands/create.rs` | `create_run()` — allocates run dir, validates workflow, persists `spec.json` |
| `lib/crates/fabro-cli/src/commands/start.rs` | `start_run()` — spawns detached `fabro _run_engine` process |
| `lib/crates/fabro-cli/src/commands/attach.rs` | `attach_run()` — tails `progress.jsonl` with live rendering + interview handling |

### Modified Files

| File | Changes |
|------|---------|
| `lib/crates/fabro-workflows/src/lib.rs` | Added `pub mod run_spec;` |
| `lib/crates/fabro-interview/src/lib.rs` | Added `pub mod file;` and `pub use file::FileInterviewer;` |
| `lib/crates/fabro-interview/Cargo.toml` | Added `tracing` dependency |
| `lib/crates/fabro-cli/src/commands/mod.rs` | Registered `attach`, `create`, `start` modules; changed `run_progress` to `pub(crate)` |
| `lib/crates/fabro-cli/src/commands/run_progress.rs` | Added `handle_json_line()`, `hide_bars()`, `show_bars()` methods to `ProgressUI` |
| `lib/crates/fabro-cli/src/main.rs` | Added `Create`, `Start`, `Attach`, `RunEngine` command variants; rewrote `--detach` to use create+start; deleted `detach_run()` |

### Architecture

- **`fabro create`** — validates workflow, creates run directory with `spec.json`, `graph.fabro`, `id.txt`, `status.json(Submitted)`, `progress.jsonl`
- **`fabro start`** — validates status is `Submitted`, spawns `fabro _run_engine --run-dir <dir>` as detached process, writes `run.pid`
- **`fabro attach`** — polls `progress.jsonl` feeding lines to `ProgressUI.handle_json_line()`, handles `interview_request.json`/`interview_response.json` IPC, exits on `conclusion.json`
- **`fabro run`** (foreground) — unchanged, still uses in-process `run_command()`
- **`fabro run --detach`** — now uses `create_run() + start_run() + println!(run_id)` instead of the brittle argv-reconstruction in old `detach_run()`
- **`fabro _run_engine`** — hidden command that loads `spec.json`, reconstructs `RunArgs`, and calls `run_command()`
- **`FileInterviewer`** — writes `interview_request.json`, polls for `interview_response.json` with configurable timeout
- **`handle_json_line`** — parses JSONL envelopes and dispatches to existing `ProgressUI` rendering methods (sandbox events, stages, tool calls, compaction, retro, etc.)

### Tests Added

- `run_spec::tests::save_load_roundtrip` — RunSpec serialization roundtrip
- `run_spec::tests::load_nonexistent` — error on missing file
- `file::tests::write_request_poll_response` — end-to-end FileInterviewer flow
- `file::tests::timeout_returns_default` — timeout with default answer
- `file::tests::timeout_without_default_returns_timeout` — timeout without default
- `run_progress::tests::handle_json_line_stage_started_and_completed` — JSONL stage lifecycle
- `run_progress::tests::handle_json_line_tool_call_round_trip` — JSONL tool call events
- `run_progress::tests::handle_json_line_retro_events` — JSONL retro events
- `run_progress::tests::handle_json_line_ignores_invalid_json` — graceful handling of bad input
- `main::tests::parse_create_command` — CLI arg parsing for `fabro create`
- `main::tests::parse_start_command` — CLI arg parsing for `fabro start`
- `main::tests::parse_attach_command` — CLI arg parsing for `fabro attach`
- `main::tests::parse_run_engine_command` — CLI arg parsing for `fabro _run_engine`