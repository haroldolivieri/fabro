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
- **implement**: success
  - Model: claude-opus-4-6, 38.3k tokens in / 4.9k out
  - Files: /home/daytona/workspace/lib/crates/fabro-cli/src/init.rs, /home/daytona/workspace/lib/crates/fabro-github/src/lib.rs


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