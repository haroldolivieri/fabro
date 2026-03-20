Zero warnings. Here's a summary of what was fixed:

## Fixes Applied

### 1. Extracted `WorktreeEvent → WorkflowRunEvent` callback helper (3 → 1)
The same 12-line match block mapping `WorktreeEvent` variants to `WorkflowRunEvent` was duplicated in three places:
- `run.rs`: new-run path (`SandboxProvider::Local`)
- `run.rs`: resume path (`run_from_branch`)
- `parallel.rs`: branch worktree setup

**Fix:** Added `fn worktree_event_callback(emitter: Arc<EventEmitter>) -> WorktreeEventCallback` to each file as a local helper, replacing all three inline match blocks with a single call.

### 2. Extracted `LocalSandbox + event_callback` helper (3 → 1)
In `run.rs`, the pattern `let mut env = LocalSandbox::new(cwd); env.set_event_callback(...); Arc::new(env)` appeared twice in the same match arm (fallback on error + else branch) and again in `run_from_branch`.

**Fix:** Added `fn local_sandbox_with_callback(cwd: PathBuf, emitter: Arc<EventEmitter>) -> Arc<dyn Sandbox>` and replaced all three sites.

### 3. Eliminated `.clone().unwrap()` on `Option<String>`
`worktree_base_sha.clone().unwrap()` (and same for `branch_name`, `wt_path`) cloned the entire `Option` before unwrapping. 

**Fix:** Changed `if worktree_base_sha.is_some()` to `if let (Some(base_sha), Some(branch_name), Some(wt_path)) = (worktree_base_sha.as_ref(), ...)` — borrows the inner values directly, then only clones the `String`/`PathBuf` when actually needed for `WorktreeConfig`.

### 4. Fixed `.to_string_lossy().to_string()` → `.into_owned()` (3 sites)
`.to_string_lossy().to_string()` calls `.to_string()` on a `Cow<str>`, which allocates a new `String` even when `Cow` is already `Owned`. `.into_owned()` moves directly from `Cow::Owned` without the extra allocation.