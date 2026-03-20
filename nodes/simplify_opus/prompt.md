Goal: # Extract `fabro-model` crate from `fabro-llm`

## Context

The model catalog (provider identity, model metadata, alias resolution, fallback chains) is currently embedded inside `fabro-llm`, a heavyweight crate that pulls in tokio, reqwest, and many async runtime dependencies. Seven crates depend on `fabro-llm`, but several only need catalog lookups — not the LLM client. Extracting a focused `fabro-model` crate gives a clean dependency boundary: crates that only need "what models exist?" no longer pull in the entire LLM runtime.

## Public API of `fabro-model`

All items re-exported at the crate root for flat access (`fabro_model::get_model_info()`):

```rust
// Types
pub use types::{ModelInfo, ModelLimits, ModelFeatures, ModelCosts};

// Provider identity
pub use provider::{Provider, ModelId};

// Catalog lookups
pub use catalog::{
    get_model_info, list_models, default_model, default_model_for_provider,
    default_model_from_env, probe_model_for_provider, closest_model,
    build_fallback_chain, FallbackTarget,
};
```

No `Catalog` struct — the catalog is static embedded data with no configuration or lifecycle. Free functions are the right abstraction. The crate name itself is the namespace.

## Key design decisions

1. **No re-export shim in `fabro-llm`** — update all consumers directly. Exception: `fabro-llm` re-exports `Provider` and `ModelId` so `fabro_llm::Provider` stays valid (it's a type alias, not a shim module).
2. **Provider moves entirely** — `Provider` enum, `ModelId`, and all `Provider` methods (ALL, as_str, from_str, api_key_env_vars, has_api_key, default_from_env). Only `ProviderAdapter` trait, `validate_tool_choice()`, and `StreamEventStream` stay in `fabro-llm::provider`.
3. **`fabro-validate` drops `fabro-llm`** — it only uses catalog + Provider, so it can depend solely on `fabro-model`.

## Steps

### 1. Create `lib/crates/fabro-model/` crate

**`Cargo.toml`**:
```toml
[package]
name = "fabro-model"
edition.workspace = true
version.workspace = true
license.workspace = true
description = "LLM model catalog: provider identity, model metadata, and resolution"

[lib]
doctest = false

[dependencies]
serde.workspace = true
serde_json.workspace = true

[dev-dependencies]
insta.workspace = true
```

### 2. Move model types → `fabro-model/src/types.rs`

Extract from `fabro-llm/src/types.rs` (lines 639-675):
- `ModelInfo`, `ModelLimits`, `ModelFeatures`, `ModelCosts`

Remove these 4 structs from `fabro-llm/src/types.rs`.

### 3. Move Provider + ModelId → `fabro-model/src/provider.rs`

Extract from `fabro-llm/src/provider.rs`:
- `Provider` enum + all impl blocks (lines 14-95)
- `Display`, `FromStr` impls (lines 97-118)
- `ModelId` struct + impls (lines 126-146)
- All tests for these items (lines 208-350)

What stays in `fabro-llm/src/provider.rs`:
- `ProviderAdapter` trait (lines 157-181)
- `StreamEventStream` type alias (line 153)
- `validate_tool_choice()` (lines 192-206)
- Tests for ProviderAdapter/validate_tool_choice (lines 351-418)
- Add `use fabro_model::Provider;` import at top

### 4. Move catalog → `fabro-model/src/catalog.rs` + `catalog.json`

Move both files verbatim. Internal `crate::` paths remain valid since Provider and ModelInfo are in the same crate now.

### 5. Write `fabro-model/src/lib.rs`

```rust
pub mod catalog;
pub mod provider;
pub mod types;

pub use catalog::{
    build_fallback_chain, closest_model, default_model, default_model_for_provider,
    default_model_from_env, get_model_info, list_models, probe_model_for_provider,
    FallbackTarget,
};
pub use provider::{ModelId, Provider};
pub use types::{ModelCosts, ModelFeatures, ModelInfo, ModelLimits};
```

### 6. Update `fabro-llm`

- Add `fabro-model = { path = "../fabro-model" }` to `Cargo.toml`
- Remove `pub mod catalog;` from `lib.rs`
- Change `pub use provider::{ModelId, Provider};` → `pub use fabro_model::{ModelId, Provider};`
- `cli.rs`: change `use crate::catalog` → `use fabro_model as catalog`, split `use crate::types::{Message, ModelInfo}` so `ModelInfo` comes from `fabro_model`
- `client.rs`: change `crate::catalog::get_model_info` → `fabro_model::get_model_info`
- `providers/anthropic.rs`: change `crate::catalog::get_model_info` → `fabro_model::get_model_info`
- Any other internal `crate::catalog` or `crate::types::ModelInfo` references

### 7. Update consumer crates

| Crate | Add dep | Import changes | Drop `fabro-llm`? |
|---|---|---|---|
| **fabro-validate** | `fabro-model` | `fabro_llm::catalog::*` → `fabro_model::*`, `fabro_llm::Provider` → `fabro_model::Provider` | **Yes** |
| **fabro-cli** | `fabro-model` | `fabro_llm::catalog::*` → `fabro_model::*` | No |
| **fabro-api** | `fabro-model` | `fabro_llm::catalog::*` → `fabro_model::*` | No |
| **fabro-workflows** | `fabro-model` | `fabro_llm::catalog::*` → `fabro_model::*`, `FallbackTarget` | No |
| **fabro-agent** | `fabro-model` | `fabro_llm::catalog::*` → `fabro_model::*` | No |
| **fabro-hooks** | `fabro-model` | `fabro_llm::catalog::get_model_info` → `fabro_model::get_model_info` | No |

## Files to modify

- **Create**: `lib/crates/fabro-model/Cargo.toml`, `src/lib.rs`, `src/types.rs`, `src/provider.rs`
- **Move**: `lib/crates/fabro-llm/src/catalog.rs` → `lib/crates/fabro-model/src/catalog.rs`
- **Move**: `lib/crates/fabro-llm/src/catalog.json` → `lib/crates/fabro-model/src/catalog.json`
- **Edit**: `lib/crates/fabro-llm/src/lib.rs`, `types.rs`, `provider.rs`, `cli.rs`, `client.rs`, `providers/anthropic.rs`, `Cargo.toml`
- **Edit**: `lib/crates/fabro-validate/Cargo.toml`, `src/rules.rs`
- **Edit**: `lib/crates/fabro-cli/Cargo.toml` + source files with `fabro_llm::catalog` imports
- **Edit**: `lib/crates/fabro-api/Cargo.toml` + source files
- **Edit**: `lib/crates/fabro-workflows/Cargo.toml` + source files
- **Edit**: `lib/crates/fabro-agent/Cargo.toml` + source files
- **Edit**: `lib/crates/fabro-hooks/Cargo.toml` + source files

## Verification

1. `cargo build --workspace` — compiles cleanly
2. `cargo test -p fabro-model` — all catalog tests pass (snapshot tests included)
3. `cargo test --workspace` — no regressions
4. `cargo clippy --workspace -- -D warnings` — no lint warnings
5. `cargo fmt --check --all` — formatted
6. Verify `fabro-validate` no longer depends on `fabro-llm`: `cargo tree -p fabro-validate | grep fabro-llm` should return nothing


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
  - Model: claude-opus-4-6, 119.4k tokens in / 26.9k out
  - Files: /home/daytona/workspace/lib/crates/fabro-agent/Cargo.toml, /home/daytona/workspace/lib/crates/fabro-agent/src/cli.rs, /home/daytona/workspace/lib/crates/fabro-agent/src/test_support.rs, /home/daytona/workspace/lib/crates/fabro-agent/src/tools.rs, /home/daytona/workspace/lib/crates/fabro-agent/tests/guardrails.rs, /home/daytona/workspace/lib/crates/fabro-api/Cargo.toml, /home/daytona/workspace/lib/crates/fabro-cli/Cargo.toml, /home/daytona/workspace/lib/crates/fabro-hooks/Cargo.toml, /home/daytona/workspace/lib/crates/fabro-llm/Cargo.toml, /home/daytona/workspace/lib/crates/fabro-llm/src/cli.rs, /home/daytona/workspace/lib/crates/fabro-llm/src/client.rs, /home/daytona/workspace/lib/crates/fabro-llm/src/lib.rs, /home/daytona/workspace/lib/crates/fabro-llm/src/provider.rs, /home/daytona/workspace/lib/crates/fabro-llm/src/providers/anthropic.rs, /home/daytona/workspace/lib/crates/fabro-llm/src/types.rs, /home/daytona/workspace/lib/crates/fabro-model/src/catalog.rs, /home/daytona/workspace/lib/crates/fabro-model/src/lib.rs, /home/daytona/workspace/lib/crates/fabro-model/src/provider.rs, /home/daytona/workspace/lib/crates/fabro-validate/Cargo.toml, /home/daytona/workspace/lib/crates/fabro-validate/src/rules.rs, /home/daytona/workspace/lib/crates/fabro-workflows/Cargo.toml


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