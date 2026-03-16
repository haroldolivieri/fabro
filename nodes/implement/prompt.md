Goal: # Add `-q`/`--quiet` to `fabro ps`

## Context
`docker ps -q` outputs only container IDs (one per line). We want the same for `fabro ps` — output only run IDs.

## Changes

**File: `lib/crates/fabro-workflows/src/cli/runs.rs`**

1. Add `quiet` field to `RunsListArgs` (after `all`, line ~45):
   ```rust
   /// Only display run IDs
   #[arg(short = 'q', long)]
   pub quiet: bool,
   ```

2. In `list_command()` (~line 474), add a quiet branch before the JSON branch:
   ```rust
   if args.quiet {
       for run in &filtered {
           println!("{}", run.run_id);
       }
       return Ok(());
   }
   ```
   - Prints full run IDs (not truncated), one per line, no header, no footer — matches `docker ps -q` behavior.
   - Skips the "no runs found" messages (empty output = no runs, like docker).

## Verification
- `cargo test -p fabro-workflows`
- `cargo clippy --workspace -- -D warnings`
- Manual: `fabro ps -q`, `fabro ps -qa`, `fabro ps -q --json` (quiet takes precedence)


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