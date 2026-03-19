All 55 tests pass. Here's a summary of all changes made:

### Changes Summary

**1. `lib/crates/fabro-github/src/lib.rs`** — Added two structs and two functions:

- **`AppOwner`** struct — holds the `login` field for the app's owner
- **`AppInfo`** struct — holds `slug` and `owner` (an `AppOwner`), both derived from `Deserialize`
- **`get_authenticated_app(client, jwt, base_url)`** — calls `GET {base_url}/app` with Bearer JWT auth, returns `AppInfo` on 200, error on 401/other
- **`is_app_public(client, slug, base_url)`** — calls `GET {base_url}/apps/{slug}` **without** auth, returns `Ok(true)` on 200, `Ok(false)` on 404, error on other

**5 new tests** added to the test module:
- `get_authenticated_app_success` — verifies 200 response is parsed into `AppInfo`
- `get_authenticated_app_auth_failure` — verifies 401 returns authentication error
- `is_app_public_returns_true_on_200` — public app returns true
- `is_app_public_returns_false_on_404` — private app returns false
- `is_app_public_no_auth_header` — verifies no `Authorization` header is sent (using `Matcher::Missing`)

**2. `lib/crates/fabro-cli/src/init.rs`** — Updated `check_github_app_installation`:

In the `Ok(false)` branch (app not installed), before showing the install URL, added best-effort checks:
1. Calls `get_authenticated_app()` to get the app's owner
2. Compares `app_info.owner.login` with the repo `owner` (case-insensitive via `eq_ignore_ascii_case`)
3. If they differ, calls `is_app_public()` to check visibility
4. If the app is private and owners differ, shows a targeted warning about making the app public

All checks are best-effort — failures at any step are silently ignored and the existing generic warning still shows.