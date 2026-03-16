All 179 tests pass and clippy is clean. Here's a summary of the changes made:

**File: `lib/crates/fabro-workflows/src/cli/runs.rs`**

1. **Added `quiet` field to `RunsListArgs`** (after `all`):
   ```rust
   /// Only display run IDs
   #[arg(short = 'q', long)]
   pub quiet: bool,
   ```

2. **Added quiet branch in `list_command()`** before the JSON branch:
   ```rust
   if args.quiet {
       for run in &filtered {
           println!("{}", run.run_id);
       }
       return Ok(());
   }
   ```

This matches `docker ps -q` behavior:
- Prints full run IDs (not truncated), one per line
- No header, no footer
- Empty output when no runs match (no "no runs found" messages)
- Quiet takes precedence over JSON output
- Combinable with `-a` (`fabro ps -qa`)