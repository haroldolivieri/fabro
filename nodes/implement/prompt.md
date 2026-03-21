Goal: # Plan: Extract `fabro resume` subcommand

## Context

Resume functionality is currently embedded in `fabro run` via `--resume` (checkpoint file) and `--run-branch` (git branch). This makes the `run` command's arg surface complex with `conflicts_with` annotations, and the UX is unintuitive — users must construct `fabro/run/RUN_ID` branch names manually. The new `fabro resume` subcommand provides a cleaner interface: `fabro resume RUN_ID_OR_PREFIX`.

## New `ResumeArgs` struct

```rust
pub struct ResumeArgs {
    /// Run ID, prefix, or branch (fabro/run/...)
    #[arg(required_unless_present = "checkpoint")]
    pub run: Option<String>,

    /// Resume from a checkpoint file (requires --workflow)
    #[arg(long)]
    pub checkpoint: Option<PathBuf>,

    /// Override workflow graph (required with --checkpoint)
    #[arg(long)]
    pub workflow: Option<PathBuf>,

    // Shared run options: run_dir, dry_run, auto_approve, goal, goal_file,
    // model, provider, verbose, sandbox, no_retro, ssh, preserve_sandbox
}
```

**Run ID resolution** (at top of `resume_command()`):
- If `run` starts with `fabro/run/` → strip prefix to get run_id
- Otherwise → call `find_run_id_by_prefix(&repo, &run)` (same as `rewind`/`fork`)
- Then construct branch name as `fabro/run/{run_id}`

## Files to modify

### 1. New: `lib/crates/fabro-cli/src/commands/resume.rs`
- Define `ResumeArgs` struct
- Move `run_from_branch()` body (~315 lines, `run.rs:1811-2125`) into `pub async fn resume_command()`
- Add run ID resolution logic at top (prefix → full ID via `find_run_id_by_prefix`)
- Add `--checkpoint` path: validate `--workflow` is present, load graph via `prepare_from_file()`, load checkpoint via `Checkpoint::load()`, then run engine

### 2. `lib/crates/fabro-cli/src/commands/run.rs`
- **Remove from `RunArgs`**: `resume` field (line 97-99), `run_branch` field (line 101-103)
- **Simplify `workflow`**: remove `required_unless_present = "run_branch"` — it's now always required
- **Update `conflicts_with_all`**: remove `"resume"`/`"run_branch"` from `preflight` (line 90) and `detach` (line 146)
- **Remove** `run_from_branch()` function (lines 1811-2125)
- **Remove** the `run_branch` early-return at top of `run_command()` (lines 602-604)
- **Simplify** engine call: remove `if let Some(ref checkpoint_path) = args.resume` branch (lines 1467-1476), always pass `None` for checkpoint
- **Widen visibility** of helpers used by `resume.rs`:
  - `local_sandbox_with_callback` (line 439) → `pub(crate)`
  - `resolve_ssh_config` (line 341) → `pub(crate)`
  - `resolve_ssh_clone_params` (line 355) → `pub(crate)`
  - `resolve_exe_config` (line 313) → `pub(crate)`
  - `resolve_exe_clone_params` (line 328) → `pub(crate)`
  - `resolve_preserve_sandbox` (line 261) → `pub(crate)`
  - `generate_retro` (line 2560) → `pub(crate)`
  - `write_finalize_commit` (line 2523) → `pub(crate)`
  - `print_final_output` (line 2128) → `pub(crate)`
  - `print_assets` (line 2149) → `pub(crate)`

### 3. `lib/crates/fabro-cli/src/commands/mod.rs`
- Add `pub mod resume;`

### 4. `lib/crates/fabro-cli/src/main.rs`
- Add `Resume(commands::resume::ResumeArgs)` to `Command` enum (near line 170, alongside `Rewind`/`Fork`)
- Add `Command::Resume(_) => "resume"` to command_name match
- Add dispatch handler (pattern follows `Rewind`/`Fork`/`Wait` — create styles, load cli_config, build github_app/git_author, call `resume_command()`)

### 5. `lib/crates/fabro-workflows/src/run_spec.rs`
- Remove `resume` and `run_branch` fields from `RunSpec`
- Add `#[serde(default)]` to `RunSpec` for backward compat with existing `spec.json` files
- Update `sample_spec()` in tests

### 6. `lib/crates/fabro-cli/src/commands/create.rs`
- Remove lines 86-87 that set `resume` and `run_branch` in the spec

### 7. `lib/crates/fabro-cli/src/main.rs` (`_run_engine` handler)
- Remove lines setting `resume` and `run_branch` when reconstructing `RunArgs` from `RunSpec`

### 8. `lib/crates/fabro-cli/src/commands/rewind.rs` (line 48-52)
- Change hint: `"To resume: fabro resume {run_id}"` (use short prefix)

### 9. `lib/crates/fabro-cli/src/commands/fork.rs` (line 56-60)
- Change hint: `"To resume: fabro resume {new_run_id}"` (use short prefix)

### 10. `lib/crates/fabro-cli/tests/cli.rs`
- Update/remove tests referencing `--resume` or `--run-branch` on `fabro run`
- Add basic parse test for `fabro resume`

### 11. Documentation (`docs/`)
- Update `docs/reference/cli.mdx`: add `fabro resume` section, remove `--resume`/`--run-branch` from `fabro run`
- Update `docs/execution/checkpoints.mdx`: change resume examples
- Update any other docs referencing `fabro run --run-branch` or `fabro run --resume`

## Verification

1. `cargo build --workspace` — compiles cleanly
2. `cargo test --workspace` — all tests pass
3. `cargo clippy --workspace -- -D warnings` — no warnings
4. Manual: `fabro resume --help` shows expected args
5. Manual: `fabro run --help` no longer shows `--resume` or `--run-branch`


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