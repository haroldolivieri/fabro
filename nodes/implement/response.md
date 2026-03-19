All checks pass. Here's a summary of the changes made:

### Changes

**1. `lib/crates/fabro-cli/src/main.rs`**
- Converted `Command::Install` from a unit variant to a struct variant with a `--web-url` option (default: `http://localhost:5173`)
- Updated three pattern match sites (`Command::Install` → `Command::Install { .. }` or `Command::Install { web_url }`)
- Passed `web_url` through to `install::run_install(&web_url)`

**2. `lib/crates/fabro-cli/src/install.rs`**
- Added `build_github_app_manifest(app_name: &str, port: u16, web_url: &str) -> serde_json::Value` helper function that builds the manifest JSON with the new `callback_urls` and `setup_url` fields
- Updated `setup_github_app` to accept `web_url: &str` and call the new helper
- Updated `run_install` to accept `web_url: &str` and pass it through to `setup_github_app`
- Added `manifest_includes_callback_urls_and_setup_url` test verifying the new fields