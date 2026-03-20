All verifications pass. Let me summarize what was found and fixed:

## Summary

The extraction of `fabro-model` from `fabro-llm` was clean. Three review agents found only minor issues:

### Fixed
1. **Missing trailing newlines** in `lib/crates/fabro-agent/Cargo.toml` and `lib/crates/fabro-workflows/Cargo.toml` — both files were missing the final `\n`, which is a POSIX convention and can cause issues with some tools.

2. **Inconsistent `ModelInfo` references** in `lib/crates/fabro-llm/src/cli.rs` — two functions (`model_row` and `print_models_table`) used the fully-qualified `crate::types::ModelInfo` while the file also had `use fabro_model::ModelInfo;` for the same type. Unified to use the imported bare `ModelInfo` for consistency.

### Verified as clean (no action needed)
- No leftover `fabro_llm::catalog` references anywhere
- No duplicate code between `fabro-model` and `fabro-llm` (clean extraction, re-exports only)
- The dual re-export of `Provider`/`ModelId` in both `fabro-llm/src/lib.rs` and `fabro-llm/src/provider.rs` is intentional — consumers use both `fabro_llm::Provider` and `fabro_llm::provider::Provider`
- The `LazyLock` catalog pattern is efficient (single parse, static lifetime)
- `fabro-validate` successfully no longer depends on `fabro-llm`
- All 239+ tests pass, clippy clean, fmt clean