All checks pass:

1. ✅ `cargo fmt --check --all` — no formatting issues
2. ✅ `cargo clippy --workspace -- -D warnings` — no warnings
3. ✅ `cargo test --workspace` — all tests pass (179 passed, 0 failed)

## Summary of changes

### Cycle 1: `Provider::default_with()` core logic (`lib/crates/fabro-llm/src/provider.rs`)

- Added `Provider::default_from_env()` — public method that checks which providers have API keys and picks the best one (Anthropic → OpenAI → Gemini precedence, fallback to Anthropic)
- Added `Provider::default_with()` — testable inner function that accepts a predicate instead of checking env vars directly
- Added 6 unit tests covering all precedence/fallback scenarios

### Cycle 2: Replace hardcoded fallbacks (6 sites)

| # | File | Change |
|---|------|--------|
| 1 | `fabro-cli/src/commands/run.rs:211` | `.unwrap_or(Provider::Anthropic)` → `.unwrap_or_else(Provider::default_from_env)` |
| 2 | `fabro-cli/src/commands/run.rs:1175` | `.unwrap_or(Provider::Anthropic)` → `.unwrap_or_else(Provider::default_from_env)` |
| 3 | `fabro-cli/src/commands/run.rs:1898-1905` | Replaced hardcoded `"claude-opus-4-6"` with catalog lookup using `default_from_env()`, and provider fallback uses `default_from_env` |
| 4 | `fabro-api/src/serve.rs:290-310` | Model resolution uses `default_from_env()` for provider-aware catalog lookup; provider fallback uses `default_from_env` |
| 5 | `fabro-workflows/src/handler/prompt.rs:70` | `.unwrap_or(Provider::Anthropic)` → `.unwrap_or_else(Provider::default_from_env)` |
| 6 | `fabro-cli/src/commands/pr.rs:388` | `catalog::default_model()` → provider-aware lookup using `default_from_env()` |