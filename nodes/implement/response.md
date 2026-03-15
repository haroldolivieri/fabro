All changes are in place. Here's a summary of what was implemented:

## Changes Made

### 1. `lib/crates/fabro-config/src/cli.rs` — Config: `upgrade_check` field
- Added `default_upgrade_check()` helper function returning `true`
- Added `#[serde(default = "default_upgrade_check")] pub upgrade_check: bool` field to `CliConfig`
- Replaced `#[derive(Default)]` with a manual `impl Default for CliConfig` so `upgrade_check` defaults to `true` in both the TOML-absent and no-config-file paths
- Added 2 tests: `parse_upgrade_check_false` and `parse_upgrade_check_default_true`

### 2. `lib/crates/fabro-cli/src/upgrade.rs` — New file with all upgrade logic
- **`UpgradeArgs`** — Clap args struct with `--version`, `--force`, `--dry-run`
- **`Backend` enum** (Gh/Http) with async methods:
  - `fetch_latest_release_tag()` — `gh release view` or async `reqwest` GET to GitHub API
  - `download_release()` — `gh release download` or async `reqwest` download
- **`select_backend()`** — Checks for `gh` CLI availability and auth status
- **`detect_target()`** — Maps OS/ARCH to target triples (macOS aarch64, Linux x86_64)
- **`parse_version_from_tag()`** — Strips `v` prefix, parses semver
- **`VersionComparison`** enum and `compare_versions()` — Newer/AlreadyCurrent/Downgrade
- **`verify_checksum()`** — SHA256 verification supporting both "hash" and "hash  filename" formats
- **`UpgradeCheckState`** — JSON state file with `checked_at`/`latest_version`, `is_stale()`, `load()`, `save()`
- **`run_upgrade()`** — Full upgrade flow: version check → downgrade protection → download → SHA256 verify → extract → atomic binary replacement
- **`maybe_print_upgrade_notice()`** — Async auto-check with 24h caching, all errors silently swallowed
- **15 unit tests** covering all pure functions

### 3. `lib/crates/fabro-cli/src/main.rs` — Wiring
- Added `mod upgrade;`
- Added `--no-upgrade-check` global argument to `Cli`
- Added `Upgrade(upgrade::UpgradeArgs)` command variant
- Added `Command::Upgrade(_) => "upgrade"` name mapping
- Added `Command::Upgrade(args) => upgrade::run_upgrade(args).await?` dispatch
- Added auto-check hook for `Run`, `Exec`, `Init`, `Install` commands

### 4. `lib/crates/fabro-cli/Cargo.toml` — Dependencies
- Added `tempfile = "3"` to `[dependencies]` (moved from dev-dependencies)
- Added `sha2.workspace = true` to `[dependencies]`
- Removed `tempfile = "3"` from `[dev-dependencies]`