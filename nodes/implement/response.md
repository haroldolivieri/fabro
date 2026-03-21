These are all valid usages - the PR code uses `run_branch` from the manifest (different from the CLI arg), and resume.rs naturally uses `run_branch` as a local variable. The `run.rs` references are the engine's `RunConfig.run_branch` field which is still needed. All correct.

Let me do a final summary of all the changes made:

## Summary of Changes

### New file: `lib/crates/fabro-cli/src/commands/resume.rs`
- New `ResumeArgs` struct with `run`, `checkpoint`, `workflow`, and all shared run options
- `resume_command()` function with two paths:
  - **Checkpoint path**: loads checkpoint file + workflow, runs engine locally
  - **Run ID path**: resolves run ID by prefix (like rewind/fork), reads checkpoint/graph from git metadata branch, resumes with worktree

### Modified: `lib/crates/fabro-cli/src/commands/run.rs`
- Removed `resume` and `run_branch` fields from `RunArgs`
- Simplified `workflow` arg (no longer `required_unless_present`)
- Simplified `preflight` conflicts (removed `resume`/`run_branch`)
- Simplified `detach` conflicts (removed `resume`/`run_branch`)
- Removed `run_branch` early-return at top of `run_command()`
- Simplified engine call: always passes `None` for checkpoint
- Removed entire `run_from_branch()` function (~320 lines)
- Made 10 helper functions `pub(crate)`: `local_sandbox_with_callback`, `resolve_ssh_config`, `resolve_ssh_clone_params`, `resolve_exe_config`, `resolve_exe_clone_params`, `resolve_preserve_sandbox`, `print_final_output`, `print_assets`, `write_finalize_commit`, `generate_retro`

### Modified: `lib/crates/fabro-cli/src/commands/mod.rs`
- Added `pub mod resume;`

### Modified: `lib/crates/fabro-cli/src/main.rs`
- Added `Resume(commands::resume::ResumeArgs)` to `Command` enum
- Added `Command::Resume(_) => "resume"` to command_name match
- Added dispatch handler for `Command::Resume`
- Removed `resume` and `run_branch` from `_run_engine` `RunArgs` reconstruction

### Modified: `lib/crates/fabro-workflows/src/run_spec.rs`
- Removed `resume` and `run_branch` fields from `RunSpec`
- Added `#[serde(default)]` for backward compat with existing spec.json files
- Added `Default` impl for `RunSpec`
- Updated `sample_spec()` in tests

### Modified: `lib/crates/fabro-cli/src/commands/create.rs`
- Removed lines setting `resume` and `run_branch` in the spec

### Modified: `lib/crates/fabro-cli/src/commands/start.rs`
- Removed `resume` and `run_branch` from test spec construction

### Modified: `lib/crates/fabro-cli/src/commands/rewind.rs`
- Changed hint to `"To resume: fabro resume {short_prefix}"`

### Modified: `lib/crates/fabro-cli/src/commands/fork.rs`
- Changed hint to `"To resume: fabro resume {short_prefix}"`

### Modified: `lib/crates/fabro-cli/tests/cli.rs`
- Removed `detach_conflicts_with_resume` test
- Removed `resume`/`run_branch` from spec JSON in test helpers
- Added `resume_help_shows_expected_args` test
- Added `resume_requires_run_or_checkpoint` test
- Added `run_help_no_longer_shows_resume_or_run_branch` test

### Documentation updates:
- **`docs/reference/cli.mdx`**: Added `fabro resume` section, removed `--resume`/`--run-branch` from `fabro run`, updated rewind/fork examples
- **`docs/execution/checkpoints.mdx`**: Updated all resume examples to use `fabro resume`
- **`docs/core-concepts/how-fabro-works.mdx`**: Updated resume examples to use `fabro resume`