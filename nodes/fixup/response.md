All tests pass and clippy is clean. Here's a summary of what was fixed:

## Changes Made

### 1. `lib/crates/fabro-sandbox/src/worktree.rs` — Fix `initialize()` command order

**Root cause**: `git branch --force` was called *before* `git worktree remove --force`. Git refuses to force-update a branch that is currently checked out by an existing worktree. When a previous run (or parallel test) left a stale worktree at the same path with the same branch name, the `branch --force` command would exit 128.

**Fix**: Move the best-effort `git worktree remove --force {path}` to **before** `git branch --force {branch} {sha}`. This frees the branch from any stale worktree registration first, allowing the force-update to succeed.

The updated sequence in `initialize()`:
1. Best-effort `git worktree remove --force {path}` (frees the branch)
2. If `!skip_branch_creation`: `git branch --force {branch} {sha}`, emit `BranchCreated`
3. `git worktree add {path} {branch}`, emit `WorktreeAdded`

Updated unit tests accordingly (new expected command order: `worktree remove`, `branch --force`, `worktree add`; and the shell-quoting assertion now checks `cmds[0]` instead of `cmds[1]`).

### 2. `lib/crates/fabro-workflows/src/git.rs` — Remove unused `init_repo_with_remote`

Removed the `init_repo_with_remote` test helper function that was defined in the `#[cfg(test)]` module but never called, eliminating the clippy dead-code warning.