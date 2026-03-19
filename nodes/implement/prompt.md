Goal: # Detect GitHub App visibility mismatch during `repo init`

## Context

When `fabro repo init` detects the GitHub App is not installed for a repo, it shows a generic "install at" URL. But if the repo owner differs from the app owner, the app must be **public** to be installable. Users currently get no guidance about this, leading to confusion when the install link doesn't work.

## Changes

### 1. Add `get_authenticated_app()` to fabro-github

**File:** `lib/crates/fabro-github/src/lib.rs`

Add two public structs near the existing response types (around line 40):

```rust
pub struct AppOwner {
    pub login: String,
}

pub struct AppInfo {
    pub slug: String,
    pub owner: AppOwner,
}
```

Add function after `check_app_installed` (after line 525):

- `pub async fn get_authenticated_app(client, jwt, base_url) -> Result<AppInfo, String>`
- Calls `GET {base_url}/app` with Bearer JWT auth
- Returns `AppInfo` on 200, errors on 401/other

### 2. Add `is_app_public()` to fabro-github

**File:** `lib/crates/fabro-github/src/lib.rs`

Add function after `get_authenticated_app`:

- `pub async fn is_app_public(client, slug, base_url) -> Result<bool, String>`
- Calls `GET {base_url}/apps/{slug}` **without** auth (public apps are visible to unauthenticated requests)
- Returns `Ok(true)` on 200, `Ok(false)` on 404, error on other status

### 3. Update `check_github_app_installation` in init.rs

**File:** `lib/crates/fabro-cli/src/init.rs`

In the `Ok(false)` branch (line 250), before showing the install URL:

1. Call `get_authenticated_app()` to get the app's owner
2. Compare `app_info.owner.login` with the repo `owner` (case-insensitive)
3. If they differ, call `is_app_public()` to check visibility
4. If the app is private and owners differ, show a targeted warning:

```
  ! GitHub App "{slug}" is private but this repo belongs to a different owner ({repo_owner}).
    The app must be made public before it can be installed outside {app_owner}.
    Update visibility at: https://github.com/settings/apps/{slug}
```

All new checks are best-effort — failures are silently ignored (the existing generic warning still shows).

### 4. Tests

**File:** `lib/crates/fabro-github/src/lib.rs` (test module)

Add tests using existing `mockito` patterns:

- `get_authenticated_app_success` — 200 returns parsed `AppInfo`
- `get_authenticated_app_auth_failure` — 401 returns error
- `is_app_public_returns_true_on_200` — public app
- `is_app_public_returns_false_on_404` — private app
- `is_app_public_no_auth_header` — verify no Authorization header is sent

## Verification

1. `cargo test -p fabro-github` — new unit tests pass
2. `cargo build --workspace` — compiles cleanly
3. `cargo clippy --workspace -- -D warnings` — no warnings
4. Manual: run `fabro repo init` in a repo owned by a different org than the app to verify the warning appears


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