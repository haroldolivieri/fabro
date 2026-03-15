All 242 tests pass (3 + 60 + 179). Here's a summary of what was fixed:

## Summary of Fixes

| # | File | Issue | Fix |
|---|------|-------|-----|
| **1** | `linux.rs` | **Spawn-kill-respawn anti-pattern**: spawned `systemd-inhibit` twice — once without `pre_exec`, immediately killed it, then re-spawned with `pre_exec`. Wasteful fork+exec+kill+wait cycle. | Rewrote to spawn once with `pre_exec` already configured via a shared `spawn_with_pdeathsig()` helper. |
| **2** | `linux.rs` | **Dead `#[cfg(not(target_os = "linux"))]` block**: `linux.rs` is only compiled on Linux (gated by `#[cfg(target_os = "linux")] mod linux` in `lib.rs`), so this branch was unreachable dead code. | Removed entirely. |
| **3** | `linux.rs` | **Missing `PR_SET_PDEATHSIG` on gnome path**: `try_gnome_inhibit` didn't set the death signal, so if the parent crashed, `gnome-session-inhibit sleep infinity` would leak as an orphan forever. | Both `try_systemd_inhibit` and `try_gnome_inhibit` now use `spawn_with_pdeathsig()`. |
| **4** | `run.rs` + `main.rs` + `fabro-workflows/Cargo.toml` | **Parameter sprawl / leaky abstraction**: `prevent_idle_sleep: bool` was threaded through `run_command` and required `fabro-workflows` to depend on `fabro-beastie`. The sleep guard is a CLI concern, not a workflow engine concern. | Moved guard acquisition to `main.rs` (matching the `Exec` path pattern), removed the parameter from `run_command`, removed `fabro-beastie` dependency and `sleep_inhibitor` feature from `fabro-workflows`. |
| **5** | `iokit_bindings.rs` | **Unused import**: `use core_foundation::base::TCFType` was imported but never used in this file. | Removed. |
| **6** | `Cargo.toml`, `fabro-cli/Cargo.toml`, `fabro-workflows/Cargo.toml` | **Missing trailing newlines**: Previous commit stripped trailing newlines from three `Cargo.toml` files. | Restored trailing newlines. |