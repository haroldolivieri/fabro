Goal: # Fix: Add OAuth callback URLs to CLI-generated GitHub App manifest

## Context

GitHub issue #97: `fabro install` creates a GitHub App that passes `fabro doctor` but cannot be used to log into `fabro-web`. The CLI manifest (`install.rs:340-354`) omits `callback_urls` and `setup_url`, so GitHub rejects the OAuth login flow with "This GitHub App must be configured with a callback URL."

The web setup flow (`setup.tsx:15-31`) already includes these fields correctly.

## Changes

### 1. Add `--web-url` flag to the `Install` command

**File:** `lib/crates/fabro-cli/src/main.rs`

Convert the `Install` variant from a unit variant (line 121) to a struct variant with a `--web-url` option:

```rust
/// Set up the Fabro environment (LLMs, certs, GitHub)
Install {
    /// Base URL for the web UI (used for OAuth callback URLs)
    #[arg(long, default_value = "http://localhost:5173")]
    web_url: String,
},
```

Update the match arm (line 833) to pass `web_url` through:

```rust
Command::Install { web_url } => {
    install::run_install(&web_url).await?;
}
```

Update the command-name match (line ~481) if it pattern-matches on `Install`.

### 2. Accept `web_url` in `run_install` and pass to `setup_github_app`

**File:** `lib/crates/fabro-cli/src/install.rs`

- Change `pub async fn run_install()` (line 553) signature to `pub async fn run_install(web_url: &str)`
- Change `async fn setup_github_app(arc_dir: &Path, s: &Styles)` (line 325) to also accept `web_url: &str`
- Pass `web_url` through at the call site inside `run_install`

### 3. Add `callback_urls` and `setup_url` to the manifest

**File:** `lib/crates/fabro-cli/src/install.rs` (lines 340-354)

Use the `web_url` parameter to construct the OAuth URLs:

```rust
let manifest = serde_json::json!({
    "name": app_name,
    "url": "https://github.com/apps/arc",
    "redirect_url": format!("http://127.0.0.1:{port}/callback"),
    "callback_urls": [format!("{web_url}/auth/callback")],
    "setup_url": format!("{web_url}/setup/callback"),
    "public": false,
    "default_permissions": {
        "contents": "write",
        "metadata": "read",
        "pull_requests": "write",
        "checks": "write",
        "issues": "write",
        "emails": "read"
    },
    "default_events": []
});
```

### 4. Add unit test for manifest callback fields

**File:** `lib/crates/fabro-cli/src/install.rs` (test module at bottom of file)

Extract a `build_github_app_manifest(port: u16, web_url: &str) -> serde_json::Value` helper from `setup_github_app` and add a test that asserts:
- `callback_urls` contains `["{web_url}/auth/callback"]`
- `setup_url` equals `"{web_url}/setup/callback"`

## Verification

1. `cargo build -p fabro-cli` — compiles without errors
2. `cargo test -p fabro-cli` — all tests pass including new manifest test
3. `cargo clippy --workspace -- -D warnings` — no lint warnings


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