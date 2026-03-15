All verification checks pass. Let me provide a summary of what was implemented and the one fix I made:

## Summary

The plan was almost fully implemented in the existing codebase. All files were already created/modified correctly:

### Files created (already present):
| File | Description |
|------|-------------|
| `lib/crates/fabro-beastie/Cargo.toml` | Crate manifest with platform-specific deps |
| `lib/crates/fabro-beastie/src/lib.rs` | Public `guard(bool) -> Option<SleepInhibitorGuard>` API with platform dispatch + 3 unit tests |
| `lib/crates/fabro-beastie/src/dummy.rs` | No-op backend (fallback for all platforms) |
| `lib/crates/fabro-beastie/src/macos.rs` | IOKit `IOPMAssertionCreateWithName` / `IOPMAssertionRelease` with RAII Drop |
| `lib/crates/fabro-beastie/src/iokit_bindings.rs` | FFI bindings for IOKit power management |
| `lib/crates/fabro-beastie/src/linux.rs` | `systemd-inhibit` / `gnome-session-inhibit` fallback chain with `PR_SET_PDEATHSIG` |

### Files modified (already in place):
| File | Change |
|------|--------|
| `Cargo.toml` (workspace) | Auto-included via `lib/crates/*` glob |
| `lib/crates/fabro-config/src/cli.rs` | `prevent_idle_sleep: bool` field + 2 tests |
| `lib/crates/fabro-cli/Cargo.toml` | `sleep_inhibitor` feature forwarding to `fabro-workflows/sleep_inhibitor` |
| `lib/crates/fabro-cli/src/main.rs` | `cfg`-guarded sleep guard for `exec` and `run` paths |
| `lib/crates/fabro-workflows/Cargo.toml` | `sleep_inhibitor` feature + optional `fabro-beastie` dep |
| `lib/crates/fabro-workflows/src/cli/run.rs` | Accepts `prevent_idle_sleep` param, `cfg`-guarded guard |

### Fix applied:
- **`lib/crates/fabro-beastie/src/linux.rs`**: Removed two `needless_return` statements (clippy `-D warnings` violation) and applied `cargo fmt` for trailing newline consistency across all modified files.

### Verification results:
1. ✅ `cargo test -p fabro-beastie` — 3 tests pass
2. ✅ `cargo test -p fabro-config` — 60 tests pass (including `parse_prevent_idle_sleep_true` and `parse_prevent_idle_sleep_defaults_to_false`)
3. ✅ `cargo build --workspace` — compiles without the feature
4. ✅ `cargo build --workspace --features fabro-cli/sleep_inhibitor` — compiles with the feature
5. ✅ `cargo clippy --workspace -- -D warnings` — clean
6. ✅ `cargo fmt --check --all` — clean