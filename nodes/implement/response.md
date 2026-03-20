Zero failures and zero errors. Let me provide a summary of all changes made:

## Summary of Changes

### New file: `lib/crates/fabro-sandbox/src/worktree.rs`
- Defines `WorktreeEvent` enum (BranchCreated, WorktreeAdded, WorktreeRemoved, Reset)
- Defines `WorktreeEventCallback` type alias
- Defines `WorktreeConfig` struct (branch_name, base_sha, worktree_path, skip_branch_creation)
- Implements `WorktreeSandbox` struct wrapping `Arc<dyn Sandbox>` with:
  - `initialize()`: runs git branch + worktree remove + worktree add + reset, emitting events; does NOT call inner's `initialize()`
  - `cleanup()`: runs git worktree remove, emits WorktreeRemoved; does NOT call inner's `cleanup()`
  - `working_directory()`: returns `config.worktree_path`
  - `exec_command()`: defaults `working_dir` to `config.worktree_path` when `None`
  - All other `Sandbox` methods delegate to inner
- 14 unit tests covering all specified behaviors (TDD green)

### Modified: `lib/crates/fabro-sandbox/src/test_support.rs`
- Added `captured_commands: Mutex<Vec<String>>` to `MockSandbox` (captures all commands in sequence)
- Added `captured_working_dirs: Mutex<Vec<Option<String>>>` to `MockSandbox` (captures working_dir per call)
- Updated `exec_command()` to populate both new fields alongside existing `captured_command`

### Modified: `lib/crates/fabro-sandbox/src/lib.rs`
- Added `pub mod worktree;`
- Added `pub use worktree::{WorktreeConfig, WorktreeEvent, WorktreeEventCallback, WorktreeSandbox};`

### Modified: `lib/crates/fabro-agent/src/sandbox.rs` + `lib.rs`
- Re-exported `WorktreeConfig`, `WorktreeEvent`, `WorktreeEventCallback`, `WorktreeSandbox` from `fabro_sandbox`
- Also re-exported `shell_quote` (which was already in `sandbox.rs` but not in `lib.rs`)

### Modified: `lib/crates/fabro-workflows/src/handler/parallel.rs`
- Removed the private `WorktreeSandbox` struct and its full `impl Sandbox` (126 lines deleted)
- Removed `use fabro_agent::LocalSandbox`
- Added `use fabro_agent::{WorktreeConfig, WorktreeEvent, WorktreeSandbox}`
- Replaced the inline git setup loop (branch creation, worktree add, reset, `is_remote()` branch for sandbox type) with a `WorktreeSandbox` that handles all of this uniformly for any inner sandbox

### Modified: `lib/crates/fabro-cli/src/commands/run.rs`
- Added `WorktreeConfig`, `WorktreeEvent`, `WorktreeSandbox` to imports
- **New runs**: Replaced `setup_worktree()` call with inline config computation (head_sha, branch_name, worktree_path), and replaced `LocalSandbox::new(cwd)` in `SandboxProvider::Local` with `WorktreeSandbox::new(LocalSandbox(original_cwd), config)` + `initialize().await` + `set_current_dir()`; graceful fallback on failure
- **Resume** (`run_from_branch`): Replaced `replace_worktree()` + `LocalSandbox::new(wt)` with `WorktreeSandbox::new(LocalSandbox(original_cwd), {skip_branch_creation: true})` + `initialize().await`
- Deleted `setup_worktree()` function (logic absorbed into `WorktreeSandbox`)
- Replaced `worktree_work_dir.is_some()` with `worktree_path.is_some()` for `meta_branch` and `git_checkpoint_enabled`