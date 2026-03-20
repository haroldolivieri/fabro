All verification steps pass:

1. ✅ `cargo build --workspace` — compiles cleanly
2. ✅ `cargo test -p fabro-model` — all 60 catalog tests pass (snapshot tests included)
3. ✅ `cargo test --workspace` — no regressions (all 179 workspace tests pass)
4. ✅ `cargo clippy --workspace -- -D warnings` — no lint warnings
5. ✅ `cargo fmt --check --all` — formatted
6. ✅ `cargo tree -p fabro-validate | grep fabro-llm` — returns nothing (fabro-validate no longer depends on fabro-llm)

Here's a summary of all changes made:

### Created files
- **`lib/crates/fabro-model/Cargo.toml`** — already existed (from prior preflight)
- **`lib/crates/fabro-model/src/lib.rs`** — crate root with re-exports of all public API
- **`lib/crates/fabro-model/src/types.rs`** — already existed with `ModelInfo`, `ModelLimits`, `ModelFeatures`, `ModelCosts`
- **`lib/crates/fabro-model/src/provider.rs`** — `Provider` enum, `ModelId` struct, all impls and tests (moved from fabro-llm)
- **`lib/crates/fabro-model/src/catalog.rs`** — catalog functions + tests (moved from fabro-llm)
- **`lib/crates/fabro-model/src/catalog.json`** — model catalog data (copied from fabro-llm)

### Deleted files
- **`lib/crates/fabro-llm/src/catalog.rs`** — moved to fabro-model
- **`lib/crates/fabro-llm/src/catalog.json`** — moved to fabro-model

### Modified files
- **`lib/crates/fabro-llm/Cargo.toml`** — added `fabro-model` dependency
- **`lib/crates/fabro-llm/src/lib.rs`** — removed `pub mod catalog;`, changed re-export to `pub use fabro_model::{ModelId, Provider};`
- **`lib/crates/fabro-llm/src/provider.rs`** — removed Provider/ModelId/tests, added `pub use fabro_model::{ModelId, Provider};`
- **`lib/crates/fabro-llm/src/types.rs`** — replaced ModelInfo structs with `pub use fabro_model::{...}`
- **`lib/crates/fabro-llm/src/cli.rs`** — `use fabro_model as catalog` + split ModelInfo import
- **`lib/crates/fabro-llm/src/client.rs`** — `fabro_model::get_model_info`
- **`lib/crates/fabro-llm/src/providers/anthropic.rs`** — `fabro_model::get_model_info`
- **`lib/crates/fabro-validate/Cargo.toml`** — replaced `fabro-llm` with `fabro-model`
- **`lib/crates/fabro-validate/src/rules.rs`** — `fabro_model::*` imports
- **`lib/crates/fabro-cli/Cargo.toml`** — added `fabro-model`
- **`lib/crates/fabro-cli/src/commands/{pr,run}.rs`** — `fabro_model::*` imports
- **`lib/crates/fabro-cli/src/commands/provider.rs`** — `fabro_model::Provider`
- **`lib/crates/fabro-cli/src/doctor.rs`** — `fabro_model::*` imports
- **`lib/crates/fabro-cli/src/install.rs`** — `fabro_model::Provider`
- **`lib/crates/fabro-cli/src/main.rs`** — `fabro_model::Provider`
- **`lib/crates/fabro-cli/src/provider_auth.rs`** — `fabro_model::Provider`
- **`lib/crates/fabro-api/Cargo.toml`** — added `fabro-model`
- **`lib/crates/fabro-api/src/{demo/mod,serve,server,sessions}.rs`** — `fabro_model::*` imports
- **`lib/crates/fabro-workflows/Cargo.toml`** — added `fabro-model`
- **`lib/crates/fabro-workflows/src/backend/{api,cli}.rs`** — `fabro_model::*` imports
- **`lib/crates/fabro-workflows/src/{cost,transform,handler/prompt}.rs`** — `fabro_model::*` imports
- **`lib/crates/fabro-agent/Cargo.toml`** — added `fabro-model`
- **`lib/crates/fabro-agent/src/{cli,memory,profiles/*,provider_profile,session,test_support,tools}.rs`** — `fabro_model::*` imports
- **`lib/crates/fabro-agent/tests/guardrails.rs`** — `fabro_model` imports
- **`lib/crates/fabro-hooks/Cargo.toml`** — added `fabro-model`
- **`lib/crates/fabro-hooks/src/executor.rs`** — `fabro_model::get_model_info`