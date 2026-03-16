Goal: # Limit command stdout/stderr to last N lines in preamble

## Context
Command nodes (e.g., `cargo check`, `cargo clippy`) can produce 300+ lines of stdout/stderr. The preamble includes all of it inline, wasting tokens on build progress/download noise. The useful content (errors, summaries) is almost always at the tail. Limit to last N lines, varying by fidelity.

## File to modify
`/Users/bhelmkamp/p/fabro-sh/fabro/lib/crates/fabro-workflows/src/preamble.rs`

## Changes

### 1. Add constants (near top of file, after imports)
```rust
const COMPACT_OUTPUT_MAX_LINES: usize = 25;
const SUMMARY_HIGH_OUTPUT_MAX_LINES: usize = 50;
```

### 2. Add `tail_lines` helper (after `format_token_count`, ~line 122)
```rust
fn tail_lines(text: &str, max_lines: usize) -> String {
    let all_lines: Vec<&str> = text.lines().collect();
    if all_lines.len() <= max_lines {
        return text.to_string();
    }
    let omitted = all_lines.len() - max_lines;
    let mut result = format!("({omitted} lines omitted)\n");
    result.push_str(&all_lines[all_lines.len() - max_lines..].join("\n"));
    result
}
```
Omission format `(N lines omitted)` matches existing `(N earlier stage(s) omitted)` pattern at lines 470/517.

### 3. Apply in `render_compact_stage_details` (affects compact + summary:medium)
Two changes — lines 167 and 178:
- `stdout.trim()` -> `tail_lines(stdout.trim(), COMPACT_OUTPUT_MAX_LINES)`
- `stderr.trim()` -> `tail_lines(stderr.trim(), COMPACT_OUTPUT_MAX_LINES)`

### 4. Apply in `render_summary_high_stage_section` (affects summary:high)
Two changes — lines 240 and 255 (only in the non-artifact `else` branches):
- `stdout.trim()` -> `tail_lines(stdout.trim(), SUMMARY_HIGH_OUTPUT_MAX_LINES)`
- `stderr.trim()` -> `tail_lines(stderr.trim(), SUMMARY_HIGH_OUTPUT_MAX_LINES)`

Artifact pointer branches (lines 231-232, 246-247) are untouched.

### 5. Tests
Add to existing `#[cfg(test)] mod tests`:
1. `tail_lines_returns_full_text_when_under_limit` — 3 lines, limit 5
2. `tail_lines_returns_full_text_at_exact_limit` — 3 lines, limit 3
3. `tail_lines_truncates_and_shows_omission` — 5 lines, limit 2, assert omission indicator + correct lines kept/dropped
4. `compact_command_stage_truncates_long_stdout` — build_preamble with Compact fidelity, >25 line stdout, assert truncation
5. `summary_high_command_stage_truncates_long_stdout` — same for SummaryHigh with >50 lines
6. `summary_high_artifact_stdout_not_truncated` — artifact pointer value, assert no truncation

## What's NOT changing
- `summary:low` — already doesn't render stdout/stderr
- `truncate` / `full` — no stage details rendered
- LLM response text in summary:high — out of scope

## Verification
```sh
cd /Users/bhelmkamp/p/fabro-sh/fabro
cargo test -p fabro-workflows -- preamble
cargo clippy -p fabro-workflows -- -D warnings
```


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
- **implement**: success
  - Model: claude-opus-4-6, 26.2k tokens in / 6.0k out
  - Files: /home/daytona/workspace/lib/crates/fabro-workflows/src/preamble.rs
- **simplify_opus**: success
  - Model: claude-opus-4-6, 23.5k tokens in / 8.0k out
  - Files: /home/daytona/workspace/lib/crates/fabro-workflows/src/preamble.rs


# Simplify: Code Review and Cleanup

Review all changed files for reuse, quality, and efficiency. Fix any issues found.

## Phase 1: Identify Changes

Run git diff (or git diff HEAD if there are staged changes) to see what changed. If there are no git changes, review the most recently modified files that the user mentioned or that you edited earlier in this conversation.

## Phase 2: Launch Three Review Agents in Parallel

Use the Agent tool to launch all three agents concurrently in a single message. Pass each agent the full diff so it has the complete context.

### Agent 1: Code Reuse Review

For each change:

1. Search for existing utilities and helpers that could replace newly written code. Use Grep to find similar patterns elsewhere in the codebase — common locations are utility directories, shared modules, and files adjacent to the changed ones.
2. Flag any new function that duplicates existing functionality. Suggest the existing function to use instead.
3. Flag any inline logic that could use an existing utility — hand-rolled string manipulation, manual path handling, custom environment checks, ad-hoc type guards, and similar patterns are common candidates.

Note: This is a greenfield app, so focus on maximizing simplicity and don't worry about changing things to achieve it.

### Agent 2: Code Quality Review

Review the same changes for hacky patterns:

1. Redundant state: state that duplicates existing state, cached values that could be derived, observers/effects that could be direct calls
2. Parameter sprawl: adding new parameters to a function instead of generalizing or restructuring existing ones
3. Copy-paste with slight variation: near-duplicate code blocks that should be unified with a shared abstraction
4. Leaky abstractions: exposing internal details that should be encapsulated, or breaking existing abstraction boundaries
5. Stringly-typed code: using raw strings where constants, enums (string unions), or branded types already exist in the codebase

Note: This is a greenfield app, so be aggressive in optimizing quality.

### Agent 3: Efficiency Review

Review the same changes for efficiency:

1. Unnecessary work: redundant computations, repeated file reads, duplicate network/API calls, N+1 patterns
2. Missed concurrency: independent operations run sequentially when they could run in parallel
3. Hot-path bloat: new blocking work added to startup or per-request/per-render hot paths
4. Unnecessary existence checks: pre-checking file/resource existence before operating (TOCTOU anti-pattern) — operate directly and handle the error
5. Memory: unbounded data structures, missing cleanup, event listener leaks
6. Overly broad operations: reading entire files when only a portion is needed, loading all items when filtering for one

## Phase 3: Fix Issues

Wait for all three agents to complete. Aggregate their findings and fix each issue directly. If a finding is a false positive or not worth addressing, note it and move on — do not argue with the finding, just skip it.

When done, briefly summarize what was fixed (or confirm the code was already clean).