Goal: # Plan: `fabro upgrade` command

## Context

Fabro has no self-upgrade mechanism. Users must manually re-run the install script. We need a `fabro upgrade` command that downloads new releases from GitHub using the `gh` CLI, plus a daily auto-check that nudges users when a newer version is available.

Simplified versus qlty: no S3, no attestation, no installer script piping. Prefer `gh` CLI, fall back to plain HTTPS (no auth) when `gh` is missing or not logged in.

## Files to modify

| File | Change |
|------|--------|
| `lib/crates/fabro-config/src/cli.rs` | Add `upgrade_check: bool` field to `CliConfig` |
| `lib/crates/fabro-cli/src/upgrade.rs` | **New file** — all upgrade logic |
| `lib/crates/fabro-cli/src/main.rs` | Add `mod upgrade`, `Upgrade` command variant, `--no-upgrade-check` global arg, dispatch, auto-check hook |
| `lib/crates/fabro-cli/Cargo.toml` | Add `tempfile` and `sha2` to `[dependencies]` |

## 1. Config: `upgrade_check` field

In `CliConfig` (`fabro-config/src/cli.rs`):
```rust
#[serde(default = "default_upgrade_check")]
pub upgrade_check: bool,
```
Default `true`. Users disable with `upgrade_check = false` in `~/.fabro/cli.toml`.

Add tests: parse true, parse false, default is true.

## 2. New file: `upgrade.rs`

### 2a. Clap args

```rust
#[derive(clap::Args)]
pub struct UpgradeArgs {
    #[arg(long)] version: Option<String>,  // target version (e.g. "0.5.0" or "v0.5.0")
    #[arg(long)] force: bool,              // upgrade even if already current
    #[arg(long)] dry_run: bool,            // preview without acting
}
```

### 2b. Download backend abstraction

Two backends behind a common trait/enum, selected at runtime:

**`GhBackend`** (preferred) — uses `std::process::Command`:
- `fetch_latest_release_tag()` — `gh release view --repo fabro-sh/fabro --json tagName -q .tagName`
- `download_release(tag, asset, dest_dir)` — `gh release download {tag} --repo fabro-sh/fabro --pattern {asset} --dir {dest_dir} --clobber`

**`HttpBackend`** (fallback) — uses `reqwest` (already a dep, no auth needed for public repos):
- `fetch_latest_release_tag()` — GET `https://api.github.com/repos/fabro-sh/fabro/releases/latest`, parse `.tag_name` from JSON
- `download_release(tag, asset, dest_dir)` — GET `https://github.com/fabro-sh/fabro/releases/download/{tag}/{asset}`, write to file

**Selection logic** (`select_backend()`):
1. Run `gh --version`. If not found → use `HttpBackend`
2. Run `gh auth status`. If exit code 4 (not authenticated) → use `HttpBackend`
3. Otherwise → use `GhBackend`

This keeps `gh` as the preferred path (handles private repos, rate limits, auth) but lets users without `gh` still upgrade.

### 2c. Platform detection

`detect_target() -> Result<&'static str>` using `std::env::consts::{OS, ARCH}`:
- `("macos", "aarch64")` → `"aarch64-apple-darwin"`
- `("linux", "x86_64")` → `"x86_64-unknown-linux-gnu"`
- Everything else → error

### 2d. `run_upgrade(args)` flow

1. `select_backend()` → `GhBackend` or `HttpBackend`
2. Parse current version: `semver::Version::parse(env!("CARGO_PKG_VERSION"))`
3. Determine target version:
   - `--version` provided → parse it (strip `v` prefix), tag = `"v{version}"`
   - Otherwise → `fetch_latest_release_tag()`, parse version from tag
4. **Downgrade protection:**
   - target < current, no `--version` → error: "latest release is older than installed, skipping"
   - target < current, `--version` explicit → warn + `dialoguer::Confirm` prompt (bail if not tty)
5. target == current, no `--force` → "Already on version {current}", return
6. `--dry-run` → print what would happen, return
7. `detect_target()?` → target triple
8. `current_exe = std::env::current_exe()?.canonicalize()?`
9. `tmp_dir = tempfile::tempdir_in(current_exe.parent())?` (same filesystem for atomic rename)
10. Download `fabro-{triple}.tar.gz` and `fabro-{triple}.tar.gz.sha256` into tmp_dir
11. **Verify SHA256**: read `.sha256` file, compute sha256 of `.tar.gz` with `sha2`, compare
12. Extract: `tar xzf {path} -C {tmp_dir}`
13. **Atomic binary replacement:**
    - `backup = exe_dir.join(".fabro-upgrade-backup")`
    - `rename(current_exe, backup)` — move old out
    - `rename(extracted_binary, current_exe)` — put new in place
    - If second rename fails, restore from backup
    - `remove_file(backup).ok()` — cleanup
    - Set permissions 0o755
14. Print success: "Upgraded fabro to {target_version}"

### 2e. Auto version check

```rust
const CHECK_INTERVAL_SECS: u64 = 86400; // 24 hours
const LAST_CHECK_FILE: &str = "last_upgrade_check.json";
```

State file at `~/.fabro/last_upgrade_check.json`:
```json
{"checked_at": 1710000000, "latest_version": "0.5.0"}
```

`pub fn maybe_print_upgrade_notice(no_upgrade_check: bool)`:

1. If `no_upgrade_check` → return
2. Load `CliConfig`; if `upgrade_check == false` → return
3. Read state file from `~/.fabro/last_upgrade_check.json`
4. If file exists and `checked_at` is within 24h → compare stored `latest_version` vs current, print notice if newer, return
5. If stale/missing → use `select_backend()` to pick gh or HTTP, fetch latest tag synchronously (typically <1s)
6. Write state file with new timestamp and version
7. If discovered version > current → print to stderr:
   ```
   A new version of fabro is available: 0.5.0 (current: 0.4.0)
   Run `fabro upgrade` to update.
   ```
8. **All errors silently swallowed** (debug-logged). Auto-check must never fail a command.

## 3. Wire into `main.rs`

- Add `mod upgrade;`
- Global arg: `#[arg(long, global = true)] no_upgrade_check: bool`
- Command variant: `Upgrade(upgrade::UpgradeArgs)`
- Command name: `Command::Upgrade(_) => "upgrade"`
- Dispatch: `Command::Upgrade(args) => upgrade::run_upgrade(args).await?`
- Auto-check hook — call `upgrade::maybe_print_upgrade_notice(cli.no_upgrade_check)` after logging init, only for select commands:
  ```rust
  let check_upgrade = matches!(
      cli.command,
      Command::Run(_) | Command::Exec(_) | Command::Init | Command::Install
  );
  if check_upgrade {
      upgrade::maybe_print_upgrade_notice(cli.no_upgrade_check);
  }
  ```

## 4. Dependencies (`fabro-cli/Cargo.toml`)

Add to `[dependencies]`:
- `tempfile = "3"` (move from dev-dependencies)
- `sha2.workspace = true`

## 5. Implementation approach: Red/Green TDD

Build each piece test-first in this order:

### Step 1: Config field
- **Red**: Write test `parse_upgrade_check_false`, `parse_upgrade_check_default_true` in `fabro-config/src/cli.rs`
- **Green**: Add `upgrade_check: bool` field with `#[serde(default = "default_upgrade_check")]` to `CliConfig`

### Step 2: Platform detection
- **Red**: Write test `detect_target_returns_known_triple` in `upgrade.rs`
- **Green**: Implement `detect_target()`

### Step 3: Version parsing helpers
- **Red**: Write tests for `parse_version_from_tag("v0.5.0")`, stripping `v` prefix, invalid input
- **Green**: Implement `parse_version_from_tag()`

### Step 4: Downgrade/same-version logic (pure functions, no side effects)
- **Red**: Write tests for version comparison outcomes: newer available, already current, downgrade detected
- **Green**: Implement `VersionComparison` enum and `compare_versions(current, target, explicit)` returning the comparison result

### Step 5: Upgrade check state file (serialization/deserialization)
- **Red**: Write tests for roundtrip serde of `UpgradeCheckState`, staleness check
- **Green**: Implement `UpgradeCheckState` struct, `is_stale()`, `load()`/`save()` methods

### Step 6: Download backends
- **Red**: Write test that `select_backend()` returns `GhBackend` when `gh` is available and authed, `HttpBackend` otherwise
- **Green**: Implement `select_backend()`, `GhBackend` (shells out to `gh`), `HttpBackend` (uses `reqwest` with GitHub public API/download URLs)

### Step 7: SHA256 verification
- **Red**: Write test that verifies a known hash against bytes
- **Green**: Implement `verify_checksum()`

### Step 8: Wire into main.rs
- Add `mod upgrade`, `Upgrade` command variant, `--no-upgrade-check`, dispatch, auto-check hook
- Run `cargo build -p fabro-cli` to confirm compilation

### Step 9: End-to-end smoke tests
- `cargo run -- upgrade --dry-run`
- `cargo run -- upgrade --version 0.3.0 --dry-run` (downgrade warning)
- `cargo clippy --workspace -- -D warnings`


## Completed stages
- **toolchain**: success
  - Script: `command -v cargo >/dev/null || { curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh -s -- -y && sudo ln -sf $HOME/.cargo/bin/* /usr/local/bin/; }; cargo --version 2>&1`
  - Stdout:
    ```
    cargo 1.94.0 (85eff7c80 2026-01-15)
    ```
  - Stderr: (empty)
- **preflight_compile**: success
  - Script: `cargo check 2>&1`
  - Stdout:
    ```
    Updating crates.io index
    Updating git repository `https://github.com/brynary/daytona-sdk-rust`
 Downloading crates ...
  Downloaded anstyle-parse v0.2.7
  Downloaded crossbeam v0.8.4
  Downloaded dunce v1.0.5
  Downloaded idna_adapter v1.2.1
  Downloaded new_debug_unreachable v1.0.6
  Downloaded getrandom v0.2.17
  Downloaded oid-registry v0.7.1
  Downloaded percent-encoding v2.3.2
  Downloaded markup5ever v0.35.0
  Downloaded powerfmt v0.2.0
  Downloaded parking_lot_core v0.9.12
  Downloaded quote v1.0.44
  Downloaded phf_macros v0.13.1
  Downloaded referencing v0.42.2
  Downloaded scopeguard v1.2.0
  Downloaded serde_path_to_error v0.1.20
  Downloaded shell-words v1.1.1
  Downloaded sha1 v0.10.6
  Downloaded serde_derive_internals v0.29.1
  Downloaded siphasher v1.0.2
  Downloaded toml_datetime v0.6.11
  Downloaded utf8_iter v1.0.4
  Downloaded thiserror v2.0.18
  Downloaded zeroize v1.8.2
  Downloaded webpki-roots v0.26.11
  Downloaded url v2.5.8
  Downloaded zerovec-derive v0.11.2
  Downloaded utf8parse v0.2.2
  Downloaded winnow v0.7.14
  Downloaded vcpkg v0.2.15
  Downloaded xml5ever v0.35.0
  Downloaded zmij v1.0.21
  Downloaded x509-parser v0.16.0
  Downloaded aws-lc-rs v1.16.1
  Downloaded zerofrom-derive v0.1.6
  Downloaded regex-automata v0.4.14
  Downloaded version_check v0.9.5
  Downloaded unicode-width v0.2.2
  Downloaded writeable v0.6.2
  Downloaded unicode-width v0.1.14
  Downloaded serde_json v1.0.149
  Downloaded libc v0.2.182
  Downloaded libz-sys v1.1.24
  Downloaded yoke v0.8.1
  Downloaded tokio v1.49.0
  Downloaded tower-layer v0.3.3
  Downloaded rustls v0.23.37
  Downloaded web_atoms v0.1.3
  Downloaded libgit2-sys v0.18.3+1.9.2
  Downloaded tracing-subscriber v0.3.22
  Downloaded zerotrie v0.2.3
  Downloaded unicode-segmentation v1.12.0
  Downloaded encoding_rs v0.8.35
  Downloaded webpki-roots v1.0.6
  Downloaded tokio-util v0.7.18
  Downloaded sync_wrapper v1.0.2
  Downloaded libssh2-sys v0.3.1
  Downloaded yoke-derive v0.8.1
  Downloaded walkdir v2.5.0
  Downloaded uuid-simd v0.8.0
  Downloaded unicode-ident v1.0.24
  Downloaded tower-http v0.6.8
  Downloaded zerovec v0.11.5
  Downloaded ring v0.17.14
  Downloaded linux-raw-sys v0.12.1
  Downloaded tungstenite v0.26.2
  Downloaded icu_properties v2.1.2
  Downloaded tracing v0.1.44
  Downloaded want v0.3.1
  Downloaded toml_edit v0.22.27
  Downloaded termimad v0.34.1
  Downloaded generic-array v0.14.7
  Downloaded rustix v1.1.4
  Downloaded unsafe-libyaml v0.2.11
  Downloaded unicode-general-category v1.1.0
  Downloaded tower-service v0.3.3
  Downloaded toml_write v0.1.2
  Downloaded tokio-rustls v0.26.4
  Downloaded syn v2.0.117
  Downloaded nix v0.29.0
  Downloaded untrusted v0.9.0
  Downloaded ulid v1.2.1
  Downloaded tinystr v0.8.2
  Downloaded time v0.3.47
  Downloaded git2 v0.20.4
  Downloaded serde_with v3.17.0
  Downloaded schemars v1.2.1
  Downloaded quinn-proto v0.11.14
  Downloaded markup5ever_rcdom v0.35.0+unofficial
  Downloaded idna v1.1.0
  Downloaded utf-8 v0.7.6
  Downloaded typenum v1.19.0
  Downloaded tracing-core v0.1.36
  Downloaded tracing-appender v0.2.4
  Downloaded toml v0.8.23
  Downloaded tokio-macros v2.6.0
  Downloaded tinyvec_macros v0.1.1
  Downloaded nix v0.31.2
  Downloaded equivalent v1.0.2
  Downloaded tokio-tungstenite v0.26.2
  Downloaded socket2 v0.6.2
  Downloaded serde_yaml v0.9.34+deprecated
  Downloaded process-wrap v9.0.3
  Downloaded jsonschema v0.42.2
  Downloaded icu_properties_data v2.1.2
  Downloaded htmd v0.5.0
  Downloaded sse-stream v0.2.1
  Downloaded signal-hook-registry v1.4.8
  Downloaded serde_core v1.0.228
  Downloaded regex-syntax v0.8.10
  Downloaded regex v1.12.3
  Downloaded openssl v0.10.75
  Downloaded iri-string v0.7.10
  Downloaded tower v0.5.3
  Downloaded thiserror v1.0.69
  Downloaded string_cache v0.8.9
  Downloaded stable_deref_trait v1.2.1
  Downloaded sha2 v0.10.9
  Downloaded serde_derive v1.0.228
  Downloaded semver v1.0.27
  Downloaded rustls-platform-verifier v0.6.2
  Downloaded rmcp v0.15.0
  Downloaded reqwest v0.12.28
  Downloaded rand v0.8.5
  Downloaded nom v7.1.3
  Downloaded icu_collections v2.1.1
  Downloaded uuid v1.21.0
  Downloaded tracing-attributes v0.1.31
  Downloaded tinyvec v1.10.0
  Downloaded zerocopy v0.8.40
  Downloaded time-core v0.1.8
  Downloaded thread_local v1.1.9
  Downloaded thiserror-impl v1.0.69
  Downloaded tendril v0.4.3
  Downloaded tar v0.4.44
  Downloaded subtle v2.6.1
  Downloaded string_cache_codegen v0.5.4
  Downloaded strict v0.2.0
  Downloaded smallvec v1.15.1
  Downloaded slab v0.4.12
  Downloaded signal-hook-mio v0.2.5
  Downloaded shlex v1.3.0
  Downloaded shell-escape v0.1.5
  Downloaded rustls-webpki v0.103.9
  Downloaded reqwest-middleware v0.4.2
  Downloaded ref-cast v1.0.25
  Downloaded portable-atomic v1.13.1
  Downloaded pin-project-lite v0.2.17
  Downloaded phf_shared v0.11.3
  Downloaded minimal-lexical v0.2.1
  Downloaded memchr v2.8.0
  Downloaded icu_locale_core v2.1.1
  Downloaded clap_builder v4.5.60
  Downloaded chrono v0.4.44
  Downloaded zerofrom v0.1.6
  Downloaded xattr v1.6.1
  Downloaded vsimd v0.8.0
  Downloaded untrusted v0.7.1
  Downloaded unit-prefix v0.5.2
  Downloaded aws-lc-sys v0.38.0
  Downloaded foreign-types-shared v0.1.1
  Downloaded clap_derive v4.5.55
  Downloaded unicase v2.9.0
  Downloaded try-lock v0.2.5
  Downloaded tracing-log v0.2.0
  Downloaded asn1-rs v0.6.2
  Downloaded tokio-stream v0.1.18
  Downloaded tokio-native-tls v0.3.1
  Downloaded time-macros v0.2.27
  Downloaded tempfile v3.26.0
  Downloaded signal-hook v0.3.18
  Downloaded sharded-slab v0.1.7
  Downloaded serde_with_macros v3.17.0
  Downloaded serde_repr v0.1.20
  Downloaded rustls-pki-types v1.14.0
  Downloaded rusticata-macros v4.1.0
  Downloaded rustc-hash v2.1.1
  Downloaded reqwest v0.13.2
  Downloaded rand_core v0.6.4
  Downloaded num-cmp v0.1.0
  Downloaded mime v0.3.17
  Downloaded indexmap v1.9.3
  Downloaded schemars_derive v1.2.1
  Downloaded openssl-probe v0.2.1
  Downloaded log v0.4.29
  Downloaded indicatif v0.18.4
  Downloaded icu_normalizer_data v2.1.1
  Downloaded futures-util v0.3.32
  Downloaded synstructure v0.13.2
  Downloaded simple_asn1 v0.6.4
  Downloaded rustls-pemfile v2.2.0
  Downloaded openssh v0.11.6
  Downloaded once_cell v1.21.3
  Downloaded native-tls v0.2.18
  Downloaded hyper v1.8.1
  Downloaded httparse v1.10.1
  Downloaded getrandom v0.3.4
  Downloaded crossbeam-deque v0.8.6
  Downloaded thiserror-impl v2.0.18
  Downloaded termcolor v1.4.1
  Downloaded strsim v0.11.1
  Downloaded serde_spanned v0.6.9
  Downloaded schemars v0.9.0
  Downloaded same-file v1.0.6
  Downloaded rustc_version v0.4.1
  Downloaded ref-cast-impl v1.0.25
  Downloaded pkg-config v0.3.32
  Downloaded pastey v0.2.1
  Downloaded num-traits v0.2.19
  Downloaded num-iter v0.1.45
  Downloaded mac v0.1.1
  Downloaded hyper-tls v0.6.0
  Downloaded document-features v0.2.12
  Downloaded bytes v1.11.1
  Downloaded signature v2.2.0
  Downloaded serde v1.0.228
  Downloaded rand_core v0.9.5
  Downloaded precomputed-hash v0.1.1
  Downloaded form_urlencoded v1.2.2
  Downloaded lock_api v0.4.14
  Downloaded serde_urlencoded v0.7.1
  Downloaded ryu v1.0.23
  Downloaded mime_guess v2.0.5
  Downloaded matchers v0.2.0
  Downloaded litemap v0.8.1
  Downloaded h2 v0.4.13
  Downloaded futures v0.3.32
  Downloaded derive_more-impl v2.1.1
  Downloaded derive_more v2.1.1
  Downloaded darling_core v0.23.0
  Downloaded rand_chacha v0.3.1
  Downloaded rand v0.9.2
  Downloaded ppv-lite86 v0.2.21
  Downloaded potential_utf v0.1.4
  Downloaded phf_codegen v0.11.3
  Downloaded phf v0.13.1
  Downloaded phf v0.11.3
  Downloaded option-ext v0.2.0
  Downloaded openssl-probe v0.1.6
  Downloaded openssl-macros v0.1.1
  Downloaded num-conv v0.2.0
  Downloaded num v0.4.3
  Downloaded mac_address v1.1.8
  Downloaded lazy-regex v3.6.0
  Downloaded itoa v1.0.17
  Downloaded is-wsl v0.4.0
  Downloaded httpdate v1.0.3
  Downloaded http v1.4.0
  Downloaded futures-sink v0.3.32
  Downloaded fs_extra v1.3.0
  Downloaded foreign-types v0.3.2
  Downloaded errno v0.3.14
  Downloaded email_address v0.2.9
  Downloaded dyn-clone v1.0.20
  Downloaded dialoguer v0.12.0
  Downloaded darling v0.23.0
  Downloaded crossbeam-epoch v0.9.18
  Downloaded crossbeam-channel v0.5.15
  Downloaded coolor v1.1.0
  Downloaded cmake v0.1.57
  Downloaded cfg-if v1.0.4
  Downloaded bollard v0.18.1
  Downloaded bitflags v2.11.0
  Downloaded axum v0.8.8
  Downloaded rustls-native-certs v0.8.3
  Downloaded rmcp-macros v0.15.0
  Downloaded rand_chacha v0.9.0
  Downloaded quinn-udp v0.5.14
  Downloaded proc-macro2 v1.0.106
  Downloaded phf_generator v0.13.1
  Downloaded pem v3.0.6
  Downloaded parking_lot v0.12.5
  Downloaded num-bigint v0.4.6
  Downloaded memoffset v0.9.1
  Downloaded lru-slab v0.1.2
  Downloaded hashbrown v0.12.3
  Downloaded futures-core v0.3.32
  Downloaded fancy-regex v0.17.0
  Downloaded displaydoc v0.2.5
  Downloaded dirs v6.0.0
  Downloaded darling_macro v0.23.0
  Downloaded console v0.15.11
  Downloaded quinn v0.11.9
  Downloaded mio v1.1.1
  Downloaded md5 v0.7.0
  Downloaded ident_case v1.0.1
  Downloaded hyper-util v0.1.20
  Downloaded http-body v1.0.1
  Downloaded hashbrown v0.16.1
  Downloaded futures-io v0.3.32
  Downloaded der-parser v9.0.0
  Downloaded crossbeam-utils v0.8.21
  Downloaded borrow-or-share v0.2.4
  Downloaded aho-corasick v1.1.4
  Downloaded openssl-sys v0.9.111
  Downloaded match_token v0.35.0
  Downloaded lazy_static v1.5.0
  Downloaded indexmap v2.13.0
  Downloaded icu_normalizer v2.1.1
  Downloaded html5ever v0.35.0
  Downloaded getrandom v0.4.1
  Downloaded data-encoding v2.10.0
  Downloaded darling_core v0.21.3
  Downloaded crypto-common v0.1.7
  Downloaded crokey-proc_macros v1.4.0
  Downloaded cpufeatures v0.2.17
  Downloaded convert_case v0.10.0
  Downloaded clap_lex v1.0.0
  Downloaded autocfg v1.5.0
  Downloaded phf_generator v0.11.3
  Downloaded num-integer v0.1.46
  Downloaded num-complex v0.4.6
  Downloaded matchit v0.8.4
  Downloaded litrs v1.0.0
  Downloaded lazy-regex-proc_macros v3.6.0
  Downloaded jsonwebtoken v10.3.0
  Downloaded ipnet v2.11.0
  Downloaded icu_provider v2.1.1
  Downloaded iana-time-zone v0.1.65
  Downloaded hyper-rustls v0.27.7
  Downloaded http-body-util v0.1.3
  Downloaded fraction v0.15.3
  Downloaded dotenvy v0.15.7
  Downloaded outref v0.5.2
  Downloaded open v5.3.3
  Downloaded num-rational v0.4.2
  Downloaded fluent-uri v0.4.1
  Downloaded crossterm v0.29.0
  Downloaded pin-utils v0.1.0
  Downloaded phf_shared v0.13.1
  Downloaded pathdiff v0.2.3
  Downloaded nu-ansi-term v0.50.3
  Downloaded minimad v0.14.0
  Downloaded jobserver v0.1.34
  Downloaded is-docker v0.2.0
  Downloaded futures-channel v0.3.32
  Downloaded find-msvc-tools v0.1.9
  Downloaded clap v4.5.60
  Downloaded hex v0.4.3
  Downloaded heck v0.5.0
  Downloaded futures-task v0.3.32
  Downloaded fnv v1.0.7
  Downloaded fastrand v2.3.0
  Downloaded dirs-sys v0.5.0
  Downloaded cfg_aliases v0.2.1
  Downloaded bollard-stubs v1.47.1-rc.27.3.1
  Downloaded base64 v0.22.1
  Downloaded asn1-rs-impl v0.2.0
  Downloaded allocator-api2 v0.2.21
  Downloaded is_terminal_polyfill v1.70.2
  Downloaded hyperlocal v0.9.1
  Downloaded futures-executor v0.3.32
  Downloaded foldhash v0.2.0
  Downloaded digest v0.10.7
  Downloaded deranged v0.5.8
  Downloaded crokey v1.4.0
  Downloaded cc v1.2.56
  Downloaded filetime v0.2.27
  Downloaded glob v0.3.3
  Downloaded futures-macro v0.3.32
  Downloaded futf v0.1.5
  Downloaded axum-core v0.5.6
  Downloaded async-trait v0.1.89
  Downloaded darling_macro v0.21.3
  Downloaded darling v0.21.3
  Downloaded crossbeam-queue v0.3.12
  Downloaded console v0.16.2
  Downloaded colorchoice v1.0.4
  Downloaded cli-table v0.5.0
  Downloaded bytecount v0.6.9
  Downloaded block-buffer v0.10.4
  Downloaded bit-vec v0.8.0
  Downloaded bit-set v0.8.0
  Downloaded atomic-waker v1.1.2
  Downloaded asn1-rs-derive v0.5.1
  Downloaded anstyle-query v1.1.5
  Downloaded anyhow v1.0.102
  Downloaded ahash v0.8.12
  Downloaded anstyle v1.0.13
  Downloaded anstream v0.6.21
   Compiling proc-macro2 v1.0.106
   Compiling unicode-ident v1.0.24
   Compiling quote v1.0.44
   Compiling libc v0.2.182
    Checking cfg-if v1.0.4
    Checking once_cell v1.21.3
    Checking smallvec v1.15.1
    Checking log v0.4.29
   Compiling find-msvc-tools v0.1.9
   Compiling shlex v1.3.0
   Compiling syn v2.0.117
   Compiling parking_lot_core v0.9.12
   Compiling jobserver v0.1.34
    Checking memchr v2.8.0
   Compiling cc v1.2.56
    Checking scopeguard v1.2.0
    Checking lock_api v0.4.14
   Compiling serde_core v1.0.228
    Checking parking_lot v0.12.5
    Checking itoa v1.0.17
    Checking pin-project-lite v0.2.17
   Compiling serde v1.0.228
    Checking errno v0.3.14
    Checking signal-hook-registry v1.4.8
    Checking bytes v1.11.1
    Checking mio v1.1.1
    Checking futures-core v0.3.32
   Compiling autocfg v1.5.0
    Checking bitflags v2.11.0
    Checking socket2 v0.6.2
    Checking equivalent v1.0.2
   Compiling pkg-config v0.3.32
    Checking futures-sink v0.3.32
    Checking allocator-api2 v0.2.21
    Checking foldhash v0.2.0
    Checking tracing-core v0.1.36
    Checking hashbrown v0.16.1
    Checking slab v0.4.12
   Compiling vcpkg v0.2.15
    Checking stable_deref_trait v1.2.1
    Checking futures-channel v0.3.32
   Compiling synstructure v0.13.2
    Checking indexmap v2.13.0
    Checking http v1.4.0
    Checking zeroize v1.8.2
   Compiling cmake v0.1.57
   Compiling dunce v1.0.5
   Compiling fs_extra v1.3.0
    Checking futures-io v0.3.32
    Checking futures-task v0.3.32
   Compiling aws-lc-sys v0.38.0
   Compiling openssl-sys v0.9.111
    Checking percent-encoding v2.3.2
    Checking http-body v1.0.1
    Checking rustls-pki-types v1.14.0
    Checking getrandom v0.2.17
   Compiling serde_derive v1.0.228
   Compiling tokio-macros v2.6.0
   Compiling zerofrom-derive v0.1.6
   Compiling displaydoc v0.2.5
    Checking tokio v1.49.0
   Compiling tracing-attributes v0.1.31
    Checking zerofrom v0.1.6
   Compiling yoke-derive v0.8.1
    Checking tracing v0.1.44
   Compiling zerovec-derive v0.11.2
    Checking yoke v0.8.1
   Compiling futures-macro v0.3.32
    Checking futures-util v0.3.32
    Checking zerovec v0.11.5
   Compiling httparse v1.10.1
   Compiling aws-lc-rs v1.16.1
   Compiling zmij v1.0.21
    Checking tinystr v0.8.2
   Compiling ring v0.17.14
    Checking writeable v0.6.2
    Checking base64 v0.22.1
    Checking litemap v0.8.1
    Checking icu_locale_core v2.1.1
    Checking potential_utf v0.1.4
    Checking zerotrie v0.2.3
   Compiling num-traits v0.2.19
    Checking tower-service v0.3.3
   Compiling icu_properties_data v2.1.2
    Checking untrusted v0.7.1
   Compiling icu_normalizer_data v2.1.1
    Checking icu_provider v2.1.1
    Checking icu_collections v2.1.1
    Checking tokio-util v0.7.18
    Checking try-lock v0.2.5
    Checking fnv v1.0.7
    Checking untrusted v0.9.0
    Checking atomic-waker v1.1.2
    Checking want v0.3.1
    Checking h2 v0.4.13
   Compiling rustls v0.23.37
    Checking httpdate v1.0.3
   Compiling serde_json v1.0.149
    Checking pin-utils v0.1.0
    Checking icu_properties v2.1.2
    Checking icu_normalizer v2.1.1
    Checking hyper v1.8.1
    Checking http-body-util v0.1.3
    Checking form_urlencoded v1.2.2
    Checking subtle v2.6.1
    Checking ipnet v2.11.0
    Checking hyper-util v0.1.20
    Checking idna_adapter v1.2.1
    Checking openssl-probe v0.2.1
    Checking utf8_iter v1.0.4
    Checking idna v1.1.0
    Checking sync_wrapper v1.0.2
    Checking tower-layer v0.3.3
   Compiling thiserror v2.0.18
    Checking url v2.5.8
    Checking webpki-roots v1.0.6
   Compiling thiserror-impl v2.0.18
    Checking foreign-types-shared v0.1.1
   Compiling version_check v0.9.5
   Compiling openssl v0.10.75
    Checking foreign-types v0.3.2
    Checking tower v0.5.3
   Compiling openssl-macros v0.1.1
   Compiling zerocopy v0.8.40
   Compiling siphasher v1.0.2
    Checking ryu v1.0.23
   Compiling native-tls v0.2.18
    Checking iri-string v0.7.10
    Checking mime v0.3.17
   Compiling ident_case v1.0.1
   Compiling unicase v2.9.0
   Compiling strsim v0.11.1
   Compiling mime_guess v2.0.5
    Checking tower-http v0.6.8
    Checking serde_urlencoded v0.7.1
   Compiling rustix v1.1.4
    Checking tokio-native-tls v0.3.1
   Compiling signal-hook v0.3.18
    Checking linux-raw-sys v0.12.1
    Checking hyper-tls v0.6.0
    Checking encoding_rs v0.8.35
   Compiling unicode-segmentation v1.12.0
   Compiling cfg_aliases v0.2.1
   Compiling getrandom v0.3.4
   Compiling rand_core v0.6.4
   Compiling convert_case v0.10.0
   Compiling rand v0.8.5
   Compiling phf_shared v0.11.3
    Checking num-integer v0.1.46
    Checking aho-corasick v1.1.4
   Compiling crossbeam-utils v0.8.21
    Checking regex-syntax v0.8.10
   Compiling phf_generator v0.11.3
   Compiling derive_more-impl v2.1.1
    Checking ppv-lite86 v0.2.21
   Compiling libz-sys v1.1.24
   Compiling typenum v1.19.0
    Checking regex-automata v0.4.14
    Checking num-bigint v0.4.6
   Compiling generic-array v0.14.7
   Compiling async-trait v0.1.89
   Compiling num-conv v0.2.0
   Compiling litrs v1.0.0
    Checking new_debug_unreachable v1.0.6
   Compiling getrandom v0.4.1
   Compiling time-core v0.1.8
    Checking powerfmt v0.2.0
   Compiling anyhow v1.0.102
    Checking utf-8 v0.7.6
    Checking deranged v0.5.8
   Compiling time-macros v0.2.27
   Compiling document-features v0.2.12
   Compiling darling_core v0.21.3
   Compiling string_cache_codegen v0.5.4
   Compiling phf_codegen v0.11.3
   Compiling libssh2-sys v0.3.1
    Checking lazy_static v1.5.0
   Compiling ref-cast v1.0.25
   Compiling thiserror v1.0.69
    Checking time v0.3.47
   Compiling darling_macro v0.21.3
   Compiling web_atoms v0.1.3
   Compiling thiserror-impl v1.0.69
   Compiling ref-cast-impl v1.0.25
    Checking unicode-width v0.2.2
    Checking mac v0.1.1
    Checking iana-time-zone v0.1.65
    Checking precomputed-hash v0.1.1
    Checking string_cache v0.8.9
    Checking chrono v0.4.44
    Checking futf v0.1.5
    Checking signal-hook-mio v0.2.5
   Compiling darling v0.21.3
    Checking phf v0.11.3
   Compiling toml_datetime v0.6.11
   Compiling serde_spanned v0.6.9
    Checking derive_more v2.1.1
   Compiling libgit2-sys v0.18.3+1.9.2
   Compiling memoffset v0.9.1
   Compiling toml_write v0.1.2
   Compiling winnow v0.7.14
   Compiling toml_edit v0.22.27
   Compiling crossterm v0.29.0
   Compiling serde_with_macros v3.17.0
   Compiling regex v1.12.3
    Checking tendril v0.4.3
    Checking block-buffer v0.10.4
    Checking crypto-common v0.1.7
    Checking crossbeam-epoch v0.9.18
    Checking crossbeam-channel v0.5.15
   Compiling nix v0.31.2
   Compiling nix v0.29.0
   Compiling darling_core v0.23.0
    Checking futures-executor v0.3.32
   Compiling serde_repr v0.1.20
    Checking data-encoding v2.10.0
    Checking fastrand v2.3.0
   Compiling strict v0.2.0
    Checking utf8parse v0.2.2
    Checking anstyle-parse v0.2.7
   Compiling crokey-proc_macros v1.4.0
    Checking tempfile v3.26.0
    Checking futures v0.3.32
    Checking crossbeam-deque v0.8.6
   Compiling lazy-regex-proc_macros v3.6.0
   Compiling darling_macro v0.23.0
    Checking digest v0.10.7
    Checking markup5ever v0.35.0
    Checking serde_with v3.17.0
   Compiling toml v0.8.23
    Checking sharded-slab v0.1.7
    Checking matchers v0.2.0
    Checking crossbeam-queue v0.3.12
    Checking rand_core v0.9.5
   Compiling phf_shared v0.13.1
   Compiling ahash v0.8.12
   Compiling serde_derive_internals v0.29.1
    Checking tracing-log v0.2.0
    Checking thread_local v1.1.9
    Checking is_terminal_polyfill v1.70.2
    Checking anstyle-query v1.1.5
    Checking minimal-lexical v0.2.1
    Checking cpufeatures v0.2.17
    Checking openssl-probe v0.1.6
    Checking colorchoice v1.0.4
    Checking anstyle v1.0.13
    Checking option-ext v0.2.0
    Checking nu-ansi-term v0.50.3
    Checking tracing-subscriber v0.3.22
    Checking anstream v0.6.21
    Checking dirs-sys v0.5.0
    Checking nom v7.1.3
   Compiling schemars_derive v1.2.1
   Compiling phf_generator v0.13.1
    Checking crokey v1.4.0
    Checking rand_chacha v0.3.1
    Checking rand_chacha v0.9.0
    Checking crossbeam v0.8.4
   Compiling fabro-util v0.5.0 (/home/daytona/workspace/lib/crates/fabro-util)
    Checking lazy-regex v3.6.0
   Compiling darling v0.23.0
    Checking coolor v1.1.0
    Checking console v0.16.2
    Checking num-rational v0.4.2
    Checking num-iter v0.1.45
    Checking rustls-native-certs v0.8.3
    Checking num-complex v0.4.6
    Checking tokio-stream v0.1.18
   Compiling match_token v0.35.0
    Checking minimad v0.14.0
    Checking bit-vec v0.8.0
    Checking hex v0.4.3
    Checking unicode-width v0.1.14
   Compiling rmcp v0.15.0
    Checking borrow-or-share v0.2.4
   Compiling unicode-general-category v1.1.0
    Checking dyn-clone v1.0.20
    Checking clap_lex v1.0.0
   Compiling heck v0.5.0
   Compiling clap_derive v4.5.55
    Checking clap_builder v4.5.60
    Checking schemars v1.2.1
    Checking fluent-uri v0.4.1
    Checking termimad v0.34.1
    Checking bit-set v0.8.0
    Checking html5ever v0.35.0
    Checking num v0.4.3
    Checking process-wrap v9.0.3
   Compiling rmcp-macros v0.15.0
    Checking mac_address v1.1.8
    Checking rand v0.9.2
   Compiling phf_macros v0.13.1
    Checking dirs v6.0.0
    Checking xml5ever v0.35.0
    Checking console v0.15.11
    Checking uuid v1.21.0
    Checking sse-stream v0.2.1
    Checking vsimd v0.8.0
    Checking shell-words v1.1.1
    Checking termcolor v1.4.1
   Compiling pastey v0.2.1
    Checking outref v0.5.2
    Checking md5 v0.7.0
    Checking uuid-simd v0.8.0
    Checking cli-table v0.5.0
    Checking dialoguer v0.12.0
    Checking phf v0.13.1
    Checking markup5ever_rcdom v0.35.0+unofficial
    Checking referencing v0.42.2
    Checking fraction v0.15.3
    Checking fancy-regex v0.17.0
    Checking clap v4.5.60
    Checking hyperlocal v0.9.1
    Checking sha1 v0.10.6
    Checking bollard-stubs v1.47.1-rc.27.3.1
    Checking simple_asn1 v0.6.4
    Checking xattr v1.6.1
    Checking pem v3.0.6
    Checking email_address v0.2.9
    Checking filetime v0.2.27
    Checking num-cmp v0.1.0
    Checking bytecount v0.6.9
   Compiling portable-atomic v1.13.1
    Checking signature v2.2.0
    Checking shell-escape v0.1.5
    Checking tar v0.4.44
    Checking htmd v0.5.0
    Checking rusticata-macros v4.1.0
    Checking fabro-tracker v0.5.0 (/home/daytona/workspace/lib/crates/fabro-tracker)
    Checking webpki-roots v0.26.11
   Compiling asn1-rs-derive v0.5.1
   Compiling asn1-rs-impl v0.2.0
    Checking same-file v1.0.6
    Checking dotenvy v0.15.7
    Checking unsafe-libyaml v0.2.11
    Checking glob v0.3.3
    Checking bollard v0.18.1
    Checking serde_yaml v0.9.34+deprecated
    Checking walkdir v2.5.0
    Checking asn1-rs v0.6.2
    Checking openssh v0.11.6
    Checking sha2 v0.10.9
    Checking is-docker v0.2.0
   Compiling oid-registry v0.7.1
    Checking unit-prefix v0.5.2
    Checking indicatif v0.18.4
    Checking is-wsl v0.4.0
    Checking ulid v1.2.1
    Checking axum-core v0.5.6
    Checking rustls-webpki v0.103.9
    Checking jsonwebtoken v10.3.0
    Checking serde_path_to_error v0.1.20
    Checking pathdiff v0.2.3
    Checking matchit v0.8.4
    Checking open v5.3.3
    Checking axum v0.8.8
   Compiling fabro-cli v0.5.0 (/home/daytona/workspace/lib/crates/fabro-cli)
    Checking der-parser v9.0.0
    Checking x509-parser v0.16.0
    Checking tracing-appender v0.2.4
    Checking rustls-pemfile v2.2.0
    Checking semver v1.0.27
    Checking tokio-rustls v0.26.4
    Checking rustls-platform-verifier v0.6.2
    Checking hyper-rustls v0.27.7
    Checking tungstenite v0.26.2
    Checking reqwest v0.12.28
    Checking reqwest v0.13.2
    Checking reqwest-middleware v0.4.2
    Checking daytona-api-client v0.1.0 (https://github.com/brynary/daytona-sdk-rust?rev=06033ca#06033caa)
    Checking jsonschema v0.42.2
    Checking daytona-toolbox-client v0.1.0 (https://github.com/brynary/daytona-sdk-rust?rev=06033ca#06033caa)
    Checking git2 v0.20.4
    Checking fabro-github v0.5.0 (/home/daytona/workspace/lib/crates/fabro-github)
    Checking tokio-tungstenite v0.26.2
    Checking fabro-git-storage v0.5.0 (/home/daytona/workspace/lib/crates/fabro-git-storage)
    Checking fabro-devcontainer v0.5.0 (/home/daytona/workspace/lib/crates/fabro-devcontainer)
    Checking fabro-llm v0.5.0 (/home/daytona/workspace/lib/crates/fabro-llm)
    Checking fabro-openai-oauth v0.5.0 (/home/daytona/workspace/lib/crates/fabro-openai-oauth)
    Checking fabro-mcp v0.5.0 (/home/daytona/workspace/lib/crates/fabro-mcp)
    Checking daytona-sdk v0.1.0 (https://github.com/brynary/daytona-sdk-rust?rev=06033ca#06033caa)
    Checking fabro-agent v0.5.0 (/home/daytona/workspace/lib/crates/fabro-agent)
    Checking fabro-ssh v0.5.0 (/home/daytona/workspace/lib/crates/fabro-ssh)
    Checking fabro-workflows v0.5.0 (/home/daytona/workspace/lib/crates/fabro-workflows)
    Checking fabro-config v0.5.0 (/home/daytona/workspace/lib/crates/fabro-config)
    Finished `dev` profile [unoptimized + debuginfo] target(s) in 1m 05s
    ```
  - Stderr: (empty)
- **preflight_lint**: success
  - Script: `cargo clippy -- -D warnings 2>&1`
  - Stdout:
    ```
    Compiling fabro-util v0.5.0 (/home/daytona/workspace/lib/crates/fabro-util)
    Checking fabro-mcp v0.5.0 (/home/daytona/workspace/lib/crates/fabro-mcp)
    Checking fabro-tracker v0.5.0 (/home/daytona/workspace/lib/crates/fabro-tracker)
    Checking fabro-git-storage v0.5.0 (/home/daytona/workspace/lib/crates/fabro-git-storage)
    Checking fabro-github v0.5.0 (/home/daytona/workspace/lib/crates/fabro-github)
    Checking fabro-devcontainer v0.5.0 (/home/daytona/workspace/lib/crates/fabro-devcontainer)
   Compiling fabro-cli v0.5.0 (/home/daytona/workspace/lib/crates/fabro-cli)
    Checking fabro-openai-oauth v0.5.0 (/home/daytona/workspace/lib/crates/fabro-openai-oauth)
    Checking fabro-llm v0.5.0 (/home/daytona/workspace/lib/crates/fabro-llm)
    Checking fabro-agent v0.5.0 (/home/daytona/workspace/lib/crates/fabro-agent)
    Checking fabro-ssh v0.5.0 (/home/daytona/workspace/lib/crates/fabro-ssh)
    Checking fabro-workflows v0.5.0 (/home/daytona/workspace/lib/crates/fabro-workflows)
    Checking fabro-config v0.5.0 (/home/daytona/workspace/lib/crates/fabro-config)
    Finished `dev` profile [unoptimized + debuginfo] target(s) in 15.76s
    ```
  - Stderr: (empty)


Read the plan file referenced in the goal and implement every step. Make all the code changes described in the plan.