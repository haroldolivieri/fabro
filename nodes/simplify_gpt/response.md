All clean. Here's a summary of what was fixed:

## Summary of Changes

### Fixed Issues

1. **Eliminated `FinalizeConfig` struct (−52 lines)** — Changed `run_with_lifecycle` to take `config: &mut RunConfig` instead of consuming `config: RunConfig` by value. This means:
   - The caller retains ownership of `RunConfig` and can read any fields mutated by remote git setup (e.g., `run_branch`, `base_sha`) directly after the call
   - Deleted the entire `FinalizeConfig` struct, its `from_run_config()` and `from_run_config_owned()` constructors
   - Renamed `write_finalize_commit_from(&FinalizeConfig)` back to `write_finalize_commit(&RunConfig)`
   - Simplified all `match &engine_result { Ok((ref o, _)) => ... }` to `match &engine_result { Ok(ref o) => ... }`
   - Eliminated all `finalize_config.field` references, using `config.field` directly
   - The error path is now actually more correct: if remote git setup succeeded but a later step failed, the caller still sees the populated metadata branch fields (previously they were reset to `None` in `from_run_config_owned`)

2. **Eliminated duplicated `run_internal` calls in `run_with_lifecycle` (−12 lines)** — Step 7 was manually calling `run_internal` with identical logic to the existing `run()` and `run_from_checkpoint()` methods. Now it simply delegates to those existing public methods.

3. **Fixed misleading `_ssh_command` variable name** — The variable is used in the `tracing::info!` macro, so the leading underscore was incorrect. Changed to `ssh_command`.

**Net impact: −83 lines** (44 insertions, 127 deletions across the cleanup diff)