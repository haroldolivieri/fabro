All checks pass. Here's a summary of all changes made:

## Summary of Changes

### Step 1 & 2: `lib/crates/fabro-workflows/src/git.rs`
- **Added `META_BRANCH_PREFIX` constant**: `pub const META_BRANCH_PREFIX: &str = "fabro/meta/";` after `RUN_BRANCH_PREFIX`
- **Updated `MetadataStore::branch_name()`**: Changed from `format!("refs/fabro/{run_id}")` to `format!("fabro/meta/{run_id}")`
- **Updated doc comments**: `fabro/{run_id}` → `fabro/meta/{run_id}` on the struct doc, and `refs/fabro/{run_id}` → `fabro/meta/{run_id}` on the method doc
- **Added two tests**: `metadata_branch_name_uses_meta_prefix` and `meta_branch_prefix_constant`

### Step 4: `lib/crates/fabro-workflows/src/cli/rewind.rs`
- **Fixed `find_run_id_by_prefix` pattern**: `"refs/heads/refs/fabro/"` → `"refs/heads/fabro/meta/"`
- **Simplified push refspec in `execute_rewind`**: `format!("+refs/heads/{meta_branch}:refs/heads/fabro/meta/{run_id}")` → `format!("+refs/heads/{meta_branch}:refs/heads/{meta_branch}")`
- **Added test**: `rewind_push_refspec_uses_same_name_on_both_sides`

### Step 6: `lib/crates/fabro-workflows/src/engine.rs`
- **Simplified metadata branch push**: Removed `strip_prefix("refs/fabro/")` translation and multi-line comment block, replaced with simple `format!("refs/heads/{meta_branch}")`

### Step 6 & 7: `lib/crates/fabro-workflows/src/cli/run.rs`
- **Updated doc comment**: `refs/fabro/{run_id}` → `fabro/meta/{run_id}`
- **Simplified finalize push refspec**: Removed `strip_prefix("refs/fabro/")` translation, replaced with `format!("refs/heads/{meta_branch}")`

### Step 8: Documentation
- **`docs/execution/checkpoints.mdx`**: Updated 5 occurrences of `refs/fabro/` to `fabro/meta/`, removed parenthetical about local/remote name difference, collapsed separate local/remote git-show examples into one
- **`docs/agents/outputs.mdx`**: Updated `refs/fabro/{run_id}` → `fabro/meta/{run_id}`