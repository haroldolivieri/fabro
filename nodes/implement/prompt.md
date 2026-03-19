Goal: # Plan: `fabro provider login` command

## Context

OpenAI OAuth PKCE login is currently only available during the `fabro install` wizard. Users need a way to re-authenticate with providers after initial setup — e.g., when tokens expire or they want to switch accounts. This adds `fabro provider login --provider <name>` as a standalone command. OpenAI gets the browser OAuth flow; all other providers get an API key prompt with validation.

## Changes

### 1. Extract shared auth helpers from `install.rs` into `provider_auth.rs`

**New file:** `lib/crates/fabro-cli/src/provider_auth.rs`

Move these functions from `install.rs` (make them `pub(crate)`):
- `provider_display_name()` (line 220)
- `provider_key_url()` (line 206)
- `openai_oauth_env_pairs()` (line 248)
- `write_env_file()` (line 537)
- `validate_api_key()` (line 901)
- `prompt_and_validate_key()` (line 926) — also needs `prompt_password()` (line 288) and `prompt_confirm()` (line 270)

Move associated tests from `install.rs` (`openai_oauth_env_pairs_*`, `every_provider_has_key_url`).

**Modify:** `lib/crates/fabro-cli/src/install.rs` — replace moved functions with `use crate::provider_auth::*`.

### 2. Create command module

**New file:** `lib/crates/fabro-cli/src/commands/provider.rs`

```
ProviderLoginArgs {
    #[arg(long)]
    provider: Provider,   // Provider already implements FromStr
}
```

`login_command(args)`:
- If `provider == OpenAi`: prompt "Log in via browser (OAuth)?", run `fabro_openai_oauth::run_browser_flow()`, fall back to API key on failure/decline
- Otherwise: call `prompt_and_validate_key()`
- Write credentials via `write_env_file()` (merge semantics, non-destructive)

### 3. Wire into CLI

**Modify:** `lib/crates/fabro-cli/src/commands/mod.rs` — add `pub mod provider;`

**Modify:** `lib/crates/fabro-cli/src/main.rs`:
- Add `mod provider_auth;`
- Add `ProviderCommand` enum with `Login(commands::provider::ProviderLoginArgs)`
- Add `Command::Provider { command: ProviderCommand }` variant (doc: "Provider operations")
- Add dispatch arm and `command_name` arm ("provider login")

No Cargo.toml changes needed — all deps already present.

## Files changed

| File | Action |
|------|--------|
| `lib/crates/fabro-cli/src/provider_auth.rs` | New — shared auth helpers |
| `lib/crates/fabro-cli/src/commands/provider.rs` | New — login command |
| `lib/crates/fabro-cli/src/commands/mod.rs` | Add `pub mod provider;` |
| `lib/crates/fabro-cli/src/main.rs` | Add module, enum, variant, dispatch |
| `lib/crates/fabro-cli/src/install.rs` | Remove extracted functions, import from `provider_auth` |

## Implementation approach: Red/Green TDD

Work in small cycles: write a failing test, then write the minimum code to make it pass.

### Cycle 1: Extract `provider_auth.rs` — tests pass after move
1. **Red**: Move tests from `install.rs` (`openai_oauth_env_pairs_*`, `every_provider_has_key_url`) to a new `provider_auth.rs` — they fail because the functions aren't there yet
2. **Green**: Move the functions (`provider_display_name`, `provider_key_url`, `openai_oauth_env_pairs`, `write_env_file`, `validate_api_key`, `prompt_and_validate_key`, `prompt_password`, `prompt_confirm`) from `install.rs` to `provider_auth.rs`, update `install.rs` to import them
3. **Verify**: `cargo test -p fabro-cli`

### Cycle 2: Wire `ProviderCommand` into clap — command is recognized
1. **Red**: Add a test that parses `["provider", "login", "--provider", "openai"]` via `Cli::try_parse_from` — fails because the command doesn't exist
2. **Green**: Add `ProviderCommand` enum, `Command::Provider` variant, `ProviderLoginArgs` struct, empty `login_command`, dispatch arm, `command_name` arm, `commands/mod.rs` entry
3. **Verify**: `cargo test -p fabro-cli`

### Cycle 3: Clap rejects bad input
1. **Red**: Add tests that `["provider", "login"]` (missing --provider) and `["provider", "login", "--provider", "bogus"]` both fail to parse
2. **Green**: Should already pass from cycle 2 (clap handles this). If not, adjust args.
3. **Verify**: `cargo test -p fabro-cli`

### Cycle 4: Implement `login_command` for non-OpenAI providers
1. **Green**: Implement the API-key path in `login_command` — call `prompt_and_validate_key()` and `write_env_file()`
2. **Verify**: `cargo build --workspace` compiles, manual test `fabro provider login --provider anthropic`

### Cycle 5: Implement `login_command` for OpenAI OAuth
1. **Green**: Add OpenAI branch — prompt for OAuth, run `run_browser_flow()`, fallback to API key
2. **Verify**: `cargo build --workspace` compiles, manual test `fabro provider login --provider openai`

### Final verification
1. `cargo test --workspace`
2. `cargo clippy --workspace -- -D warnings`
3. `cargo fmt --check --all`
4. `fabro install` — still works end-to-end


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