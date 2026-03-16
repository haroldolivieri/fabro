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


Read the plan file referenced in the goal and implement every step. Make all the code changes described in the plan. Use red/green TDD.