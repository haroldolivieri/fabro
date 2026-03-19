Everything passes. Here's a summary of the review and fixes:

## Review Summary

### Three parallel reviews were conducted:

**Agent 1 (Code Reuse)** found:
1. **Hardcoded `"https://api.github.com"` strings** — The codebase already has `fabro_github::GITHUB_API_BASE_URL` constant (used in `pr.rs` and `run.rs`), but `init.rs` was using hardcoded strings everywhere. → **Fixed**: Replaced all 4 occurrences with the constant.
2. Repeated `.header(...)` pattern — Pre-existing across the entire codebase (12+ occurrences). Not worth addressing here since it's the established convention.
3. `AppOwner` vs `PullRequestUser` — Both have `login: String` but represent different API entities. Separate structs is correct.

**Agent 2 (Code Quality)** found:
1. **Duplicate `let yellow` construction** — `console::Style::new().yellow()` was created twice: once inside the visibility warning block and once after it for the "not installed" message. → **Fixed**: Hoisted to a single declaration before both uses.
2. Match-on-status pattern and `Result<_, String>` error types — Consistent with existing codebase conventions. Not an issue.

**Agent 3 (Efficiency)** found:
1. Sequential HTTP calls are dependent (`is_app_public` needs slug from `get_authenticated_app`), so parallelization isn't possible.
2. This runs during `repo init` (one-time setup), not a hot path — no efficiency concern.
3. No other issues found.

### Changes made:
- **`init.rs`**: Replaced 4 hardcoded `"https://api.github.com"` strings with `fabro_github::GITHUB_API_BASE_URL`
- **`init.rs`**: Hoisted `let yellow` to single declaration shared by both warning messages (removing duplicate)