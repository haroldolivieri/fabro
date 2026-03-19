Everything looks good. Let me provide a summary of all changes made:

## Summary

All changes implemented following the TDD plan:

### New files

1. **`lib/crates/fabro-cli/src/provider_auth.rs`** — Shared auth helpers extracted from `install.rs`:
   - `provider_key_url()` — maps providers to their API key management URLs
   - `provider_display_name()` — human-readable provider names
   - `openai_oauth_env_pairs()` — converts OAuth tokens to env var pairs
   - `prompt_confirm()` — interactive yes/no prompt
   - `prompt_password()` — masked password input
   - `write_env_file()` — merge-writes key/value pairs to `~/.fabro/.env`
   - `validate_api_key()` — makes a test LLM call to validate a key
   - `prompt_and_validate_key()` — interactive key entry + validation loop
   - Tests: `openai_oauth_env_pairs_*` (4 tests), `every_provider_has_key_url`, `validate_api_key_rejects_invalid_key`

2. **`lib/crates/fabro-cli/src/commands/provider.rs`** — Login command:
   - `ProviderLoginArgs` struct with `--provider` flag (uses `Provider`'s `FromStr`)
   - `login_command()` — OpenAI gets OAuth browser flow with API key fallback; all other providers get direct API key prompt with validation. Writes credentials via `write_env_file()`.

### Modified files

3. **`lib/crates/fabro-cli/src/commands/mod.rs`** — Added `pub mod provider;`

4. **`lib/crates/fabro-cli/src/main.rs`**:
   - Added `mod provider_auth;`
   - Added `ProviderCommand` enum with `Login` variant
   - Added `Command::Provider` variant
   - Added `command_name` arm (`"provider login"`)
   - Added dispatch arm calling `commands::provider::login_command()`
   - Added 4 clap parsing tests (valid openai/anthropic, missing `--provider`, bogus provider)

5. **`lib/crates/fabro-cli/src/install.rs`**:
   - Removed extracted functions (`provider_key_url`, `provider_display_name`, `openai_oauth_env_pairs`, `prompt_confirm`, `prompt_password`, `write_env_file`, `validate_api_key`, `prompt_and_validate_key`)
   - Removed corresponding tests (moved to `provider_auth.rs`)
   - Added import: `use crate::provider_auth::{...}`
   - Cleaned up unused imports (`Confirm`, `Password` from dialoguer)
   - Retained `detect_binary_on_path`, `prompt_select`, `prompt_multiselect`, `prompt_input` (server-only), and all server-only functions/tests

### Verification
- `cargo test --workspace` — **179 passed**, 0 failed
- `cargo clippy --workspace -- -D warnings` — clean
- `cargo fmt --check --all` — clean