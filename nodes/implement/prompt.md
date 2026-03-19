Goal: # Fix: Default provider should respect configured API keys

## Context

Users who only have OpenAI (or Gemini) configured hit an error when running workflows without an explicit `--provider` flag. The system hardcodes `Provider::Anthropic` as the fallback in several places, so it tries to call Anthropic even when no `ANTHROPIC_API_KEY` exists.

The fix: add a `Provider::default_from_env()` method that checks which providers have API keys and picks the best one, then replace all hardcoded `Provider::Anthropic` fallbacks with it.

## Approach: Red/Green TDD

Each step writes failing tests first, then implements to make them pass.

---

### Cycle 1: `Provider::default_with()` core logic

**RED** — Add tests to `lib/crates/fabro-llm/src/provider.rs` (`mod tests`):

```rust
#[test]
fn default_with_all_configured_prefers_anthropic() {
    assert_eq!(Provider::default_with(|_| true), Provider::Anthropic);
}

#[test]
fn default_with_only_openai() {
    assert_eq!(Provider::default_with(|p| p == Provider::OpenAi), Provider::OpenAi);
}

#[test]
fn default_with_only_gemini() {
    assert_eq!(Provider::default_with(|p| p == Provider::Gemini), Provider::Gemini);
}

#[test]
fn default_with_openai_and_gemini_prefers_openai() {
    assert_eq!(
        Provider::default_with(|p| p == Provider::OpenAi || p == Provider::Gemini),
        Provider::OpenAi,
    );
}

#[test]
fn default_with_none_configured_falls_back_to_anthropic() {
    assert_eq!(Provider::default_with(|_| false), Provider::Anthropic);
}

#[test]
fn default_with_only_kimi_falls_back_to_anthropic() {
    assert_eq!(Provider::default_with(|p| p == Provider::Kimi), Provider::Anthropic);
}
```

Run `cargo test -p fabro-llm` → compile error (method doesn't exist).

**GREEN** — Add to `impl Provider` in the same file:

```rust
#[must_use]
pub fn default_from_env() -> Self {
    Self::default_with(Self::has_api_key)
}

fn default_with(is_configured: impl Fn(Self) -> bool) -> Self {
    const PRECEDENCE: [Provider; 3] = [Provider::Anthropic, Provider::OpenAi, Provider::Gemini];
    PRECEDENCE.iter().copied().find(|&p| is_configured(p)).unwrap_or(Provider::Anthropic)
}
```

Run `cargo test -p fabro-llm` → all 6 new tests pass.

---

### Cycle 2: Replace hardcoded fallbacks

These are mechanical substitutions. For each site, the change is the same pattern: `.unwrap_or(Provider::Anthropic)` → `.unwrap_or_else(Provider::default_from_env)`.

**Sites to update:**

| # | File | Line | What changes |
|---|------|------|-------------|
| 1 | `lib/crates/fabro-cli/src/commands/run.rs` | 211 | `resolve_model_provider()` provider fallback |
| 2 | `lib/crates/fabro-cli/src/commands/run.rs` | 1123 | `run_command()` provider parse fallback |
| 3 | `lib/crates/fabro-cli/src/commands/run.rs` | 1846-1853 | `run_from_branch()` — hardcoded `"claude-opus-4-6"` model + `Provider::Anthropic` |
| 4 | `lib/crates/fabro-api/src/serve.rs` | 290-310 | `resolve_model_provider()` — `catalog::default_model()` + provider fallback |
| 5 | `lib/crates/fabro-workflows/src/handler/prompt.rs` | 70 | prompt handler provider fallback |
| 6 | `lib/crates/fabro-cli/src/commands/pr.rs` | 388 | `catalog::default_model()` → provider-aware default |

**Special cases (not just unwrap_or swaps):**

- **run.rs:1846** — replace `"claude-opus-4-6".to_string()` with catalog lookup using `default_from_env()`:
  ```rust
  let default_provider = Provider::default_from_env();
  let model = args.model.unwrap_or_else(|| {
      fabro_llm::catalog::default_model_for_provider(default_provider.as_str())
          .map(|m| m.id)
          .unwrap_or_else(|| default_provider.as_str().to_string())
  });
  ```

- **serve.rs:294** — replace `catalog::default_model()` with provider-aware lookup:
  ```rust
  let default_provider = Provider::default_from_env();
  let default_info = provider_str
      .and_then(fabro_llm::catalog::default_model_for_provider)
      .unwrap_or_else(|| {
          fabro_llm::catalog::default_model_for_provider(default_provider.as_str())
              .unwrap_or_else(fabro_llm::catalog::default_model)
      });
  ```

- **pr.rs:388** — replace `catalog::default_model()` with same pattern.

Run `cargo test --workspace` after each file. Run `cargo clippy --workspace -- -D warnings` at the end.

---

## Verification

1. `cargo fmt --check --all`
2. `cargo clippy --workspace -- -D warnings`
3. `cargo test --workspace`


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