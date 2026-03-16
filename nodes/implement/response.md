All clean. Let me summarize what was done:

## Summary

All changes were made to `/home/daytona/workspace/lib/crates/fabro-workflows/src/preamble.rs`:

### 1. Added constants (after imports)
- `COMPACT_OUTPUT_MAX_LINES = 25` — limit for compact and summary:medium fidelities
- `SUMMARY_HIGH_OUTPUT_MAX_LINES = 50` — limit for summary:high fidelity

### 2. Added `tail_lines` helper (after `format_token_count`)
Returns the last N lines of text, prepending `(N lines omitted)` when truncation occurs. Matches the existing `(N earlier stage(s) omitted)` pattern.

### 3. Applied in `render_compact_stage_details`
Both `stdout.trim()` and `stderr.trim()` are now wrapped with `tail_lines(..., COMPACT_OUTPUT_MAX_LINES)`. This affects **compact** and **summary:medium** fidelities.

### 4. Applied in `render_summary_high_stage_section`
Both `stdout.trim()` and `stderr.trim()` in the non-artifact `else` branches are wrapped with `tail_lines(..., SUMMARY_HIGH_OUTPUT_MAX_LINES)`. Artifact pointer branches are untouched.

### 5. Added 6 tests (TDD red→green)
1. `tail_lines_returns_full_text_when_under_limit` — 3 lines, limit 5
2. `tail_lines_returns_full_text_at_exact_limit` — 3 lines, limit 3
3. `tail_lines_truncates_and_shows_omission` — 5 lines, limit 2
4. `compact_command_stage_truncates_long_stdout` — 30-line stdout with Compact fidelity
5. `summary_high_command_stage_truncates_long_stdout` — 60-line stdout with SummaryHigh fidelity
6. `summary_high_artifact_stdout_not_truncated` — artifact pointer, no truncation

**Verification**: 59 preamble tests pass, clippy clean with `-D warnings`.