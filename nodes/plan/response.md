The plan file is located at **`/home/daytona/workspace/plan.md`**.

It covers the 4 files that need modification across 4 implementation steps:

1. **Add `rand` dependency** to `fabro-agent/Cargo.toml` (workspace dep already exists)
2. **Replace UUID generation** with `format!("{:08x}", rand::random::<u32>())` in `subagent.rs`
3. **Remove 5 `short_id` truncation lines** in `fabro-agent/src/cli.rs`, using `agent_id` directly
4. **Remove 2 `short_id` truncation lines** in `fabro-cli/src/commands/run_progress.rs`, using `agent_id` directly

No new tests or files are needed — existing tests don't depend on UUID format.