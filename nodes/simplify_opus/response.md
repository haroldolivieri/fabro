All done. Here's a summary of what was found and fixed:

## Review Findings & Fixes

### Fixed (3 issues)

1. **Silent fallback bug (HIGH)** — Three call sites fell back to `provider.as_str().to_string()` (e.g. `"anthropic"`) as a model ID when `default_model_for_provider` returned `None`. This is not a valid model ID and would cause confusing downstream errors. Fixed in:
   - `run.rs` `resolve_model_provider` (line 214)
   - `run.rs` `run_from_branch` (line 1902)
   - `cli.rs` (line 391)

2. **Copy-pasted pattern (HIGH)** — The "resolve default model from env provider" logic was duplicated in 5 places with inconsistent fallback behavior. Extracted `catalog::default_model_from_env()` — a 4-line helper that all sites now use, reducing each call site from 3-6 lines to a single `.unwrap_or_else(catalog::default_model_from_env)`.

3. **Confusing nested fallback in serve.rs (LOW)** — The 3-tier nested closure with a potential double `Provider::default_from_env()` call was collapsed into a clean single chain.

### Confirmed Clean (no action needed)

- **Efficiency**: All `default_from_env()` calls are on cold paths (once per CLI invocation or workflow run). Env var reads are negligible compared to subsequent network I/O. No caching needed.
- **Lazy evaluation**: All `unwrap_or_else` calls correctly defer evaluation.
- **Stringly-typed `default_model_for_provider(&str)`**: Noted as a medium-severity API design issue, but changing the signature would affect many callers beyond this diff (including test code and the guardrails test). Deferred as a follow-up since it's pre-existing.