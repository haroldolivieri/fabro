All clean. Let me provide a summary of the changes made:

## Summary of changes

### 1. `lib/crates/fabro-workflows/src/cli/runs.rs`

**Imports**: Added `std::time::Duration` and `fabro_util::terminal::Styles`.

**`RunsListArgs` struct**: Added `--limit N` (default 10) and `--all` flag.

**`RunInfo` struct**: Added `duration_ms: Option<u64>` and `total_cost: Option<f64>` fields (serialized in JSON, skipped when `None`).

**`read_status` function**: Refactored from returning `(RunStatus, Option<DateTime<Utc>>)` to returning a `StatusInfo` struct that also carries `duration_ms` and `total_cost` from the conclusion file.

**`scan_runs` function**: Updated both code paths (manifest and orphan) to populate the new `duration_ms` and `total_cost` fields.

**New helper `format_relative_time`**: Converts `DateTime<Utc>` to human-friendly strings like `"just now"`, `"2m ago"`, `"3h ago"`, `"5d ago"`.

**New helper `style_status`**: Colors the status string based on status type (green for success/partial_success, red for fail, cyan for running, dim for unknown).

**`list_command` function**: 
- New signature takes `&Styles` parameter
- Applies `--limit`/`--all` via `.take(limit)` after filtering
- Bold header row, dim separator line
- Status column colored by status
- Run ID column dimmed, labels dimmed
- Start time shown as relative (e.g., "3h ago") instead of raw RFC 3339
- Added DURATION column (uses `format_duration_short`)
- Added COST column (uses `format_cost`)
- Shows `"-"` for duration/cost when not available
- Footer shows "Showing N of M run(s). Use --all to see all." when truncated
- JSON output still includes full RFC 3339 timestamps and raw values

**Tests**: Added `duration_ms: None, total_cost: None` to all 7 test `RunInfo` literals.

### 2. `lib/crates/fabro-cli/src/main.rs`

**`Command::Ps` handler**: Creates `Styles::detect_stdout()` and passes it to `list_command`.