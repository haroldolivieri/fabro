All clean. Here's a summary of what was fixed:

## Summary

**4 issues fixed, 2 skipped as expected/acceptable:**

### Fixed
1. **Unified duplicate GraphQL execution functions** (HIGH) — Extracted `execute_graphql_request()` into `lib.rs` as a `pub(crate)` shared helper. Both `github.rs::execute_github_graphql()` and `linear.rs::execute_graphql()` now delegate to it, eliminating ~80 lines of duplicated request/error-handling logic while preserving provider-specific auth headers and error labels.

2. **Added `make_test_issue()` helper to linear.rs tests** (MEDIUM) — Replaced 4 identical 14-field `Issue` struct constructions with a shared test helper, removing ~55 lines of boilerplate and matching the pattern already used in `github.rs` tests.

3. **Removed unused `serde` dependency** (MEDIUM) — `fabro-tracker/Cargo.toml` listed `serde.workspace = true` but no file in the crate imports or derives from `serde` directly (only `serde_json` is used). Removed the phantom dependency.

4. **Fixed missing trailing newline in `fabro-github/Cargo.toml`** (LOW) — The file was missing a POSIX trailing newline, causing `\ No newline at end of file` in diffs.

### Skipped (acceptable)
- **`test_rsa_key()` duplication** between `fabro-github` and `fabro-tracker` — Expected; Rust `#[cfg(test)]` code can't be shared across crate boundaries without a test-utils crate.
- **`fetch_issues_by_ids` ordering pattern** in both modules — Idiomatic; the provider-specific fetch logic differs enough that extraction would be forced.