---
title: "refactor: Clean up Home vs Storage path ownership"
type: refactor
status: active
date: 2026-04-19
---

# refactor: Clean up Home vs Storage path ownership

## Overview

Today the server lifecycle code records the server log as `<storage>/logs/server.log`, but tracing for serving processes still writes under `Home::logs_dir()`. Operators tailing the recorded server log miss server traces, and deployments that rely on durable `Storage` do not actually keep the server's tracing output there. This plan fixes that ownership bug, keeps CLI logs in `Home`, deletes dead legacy config/env compatibility code, and removes `Home::storage_dir()` so storage-placement policy lives in config/user resolution instead of the `Home` abstraction.

This is an aggressive greenfield cleanup. No compatibility shims, deprecation window, migration warnings, or release-note work are included for old config files, old `~/.fabro/.env`, or old `~/.fabro/server.json`.

## Key Changes

### Logging ownership

- Keep the existing server-state accessor and make its exact path the canonical server log destination:
  - `Storage::server_state().log_path()` already exists and resolves to `<storage>/logs/server.log`
- Replace the current logging setup with an explicit internal sink type:
  - CLI sink: daily rolling logs under `Home::logs_dir()`
  - Server sink: fixed file at `<storage>/logs/server.log`
- Drive sink selection from parsed command variants, not `command_name` string matching
- Resolve the sink before tracing init:
  - `server __serve`, `server start --foreground`, and `server restart --foreground` use the server sink
  - `server start` / `server restart` daemon wrappers, `server stop`, `server status`, and all other commands use the CLI sink
- Extend the pre-tracing bootstrap path in `main.rs` so server-specific config/log resolution covers `ServerCommand::Restart` when `foreground` is set, not just `Start` and `Serve`
- Move enough settings/storage resolution ahead of `logging::init_tracing()` to choose the sink for serving processes; if that early resolution fails, return the error from `main_inner()` and let the existing fatal stderr formatter in `main()` render it without initializing tracing
- The pre-tracing bootstrap for serve/foreground paths must call `user_config::load_settings_with_config_and_storage_dir(config, storage_dir)` and then `user_config::storage_dir(&settings)` so the `--storage-dir` CLI override is honored when picking the server sink
- If storage resolution fails before tracing init, keep that failure on the same `main_inner() -> main()` stderr-only fatal path; do not attempt a fallback server sink without a resolved storage root
- Keep the server log policy unchanged aside from ownership:
  - single file at `<storage>/logs/server.log`
  - truncated once on each server start/restart
  - no append+rotation/retention work in this pass
- Make the file-open semantics explicit with one truncation owner per mode:
  - foreground-serving modes own their in-process bootstrap: after acquiring `<storage>/server.lock` and confirming no active server, the foreground process ensures the parent directory exists, creates/truncates `<storage>/logs/server.log` once, and then attaches the append-mode server sink
  - daemonized start/restart keeps bootstrap ownership in the daemon parent: after acquiring `<storage>/server.lock` and confirming no active server, the parent ensures the parent directory exists, creates/truncates `<storage>/logs/server.log` once, opens the redirected stdout/stderr handles against that file before spawn, and then launches `server __serve`
  - the `server __serve` child never truncates `<storage>/logs/server.log`; it only appends once tracing initializes
  - serving-process tracing always appends to `<storage>/logs/server.log` and never performs a second truncate
- Boundary of the durable server log:
  - `<storage>/logs/server.log` becomes authoritative only after storage root resolution succeeds and the relevant serving bootstrap has created/opened the file
  - failures earlier in bootstrap, including config/settings resolution failures and lock-acquire/lock-timeout failures, remain stderr-only in this pass
- Clap parse errors, `--help`, `--version`, and other failures that happen before command classification remain stderr/bootstrap-owned and are out of scope for the server-log ownership fix
- Pre-existing `Home/logs/server*.log` files are accepted historical orphans; this pass does not add a one-time sweep
- Keep `ServerRecord.log_path` pointed at `<storage>/logs/server.log`

### Remove dead Home and legacy path APIs

- Delete `Home::server_config()`, `Home::certs_dir()`, and `Home::storage_dir()`
- Remove `fabro_config::legacy_env` entirely
- Remove legacy config filename support entirely from `fabro_config::user`:
  - delete the old filename constants and path helpers
  - delete legacy warning state and warning logic
  - make `load_settings_config()` load only the explicit path, `FABRO_CONFIG`, or the default `settings.toml`
- Remove CLI warning/reporting code for legacy `~/.fabro/.env` and old config filenames from `install`, `provider login`, and `doctor`
- Remove the legacy server record fallback to `~/.fabro/server.json`; only `<storage>/server.json` remains valid
- Make the aggressive compatibility cut explicit:
  - `server status` / `server stop` only inspect `<storage>/server.json`
  - no compatibility fallback or migration shim is added for daemons that wrote only `~/.fabro/server.json`
  - add a narrow fail-fast detector in every server lifecycle site that previously consulted `active_server_record_details` — not only `server status` and `server stop`, but also `server start`, `server restart`, `ensure_server_running_with_bind`, and the daemon/foreground execute paths — so that if `<storage>/server.json` is absent but the legacy `~/.fabro/server.json` path exists for a process that satisfies the existing `server_record_is_running()` liveness predicate (`fabro_proc::process_alive(pid)` plus `server_process_matches(record)`), the command aborts with an explicit manual-cleanup error instead of silently treating the server as stopped or spawning a duplicate
  - if the legacy record path exists but does not satisfy that existing liveness predicate, treat it as stale legacy state and continue cleanup/removal rather than aborting
  - the fail-fast error should name the legacy record path and current storage record path and tell the operator to stop the old daemon with a legacy CLI or manually clear the stale daemon before retrying
  - upgrading over a still-running legacy daemon remains unsupported; the detector is only there to fail loudly
- Do not add new `Home` accessors for one-off singleton files like `.id` or `last_upgrade_check.json`; keep those joins local to their owning modules instead of replacing one dead abstraction with new speculative ones
- Delete the `check_legacy_env` helper in `doctor.rs` and its two unit tests, and drop the `use fabro_config::legacy_env;` imports in `doctor.rs`, `install.rs`, and `provider/login.rs` so the crate removal leaves no orphaned references

### Move default storage policy out of Home

- Add a canonical `fabro_config::user::default_storage_dir()` helper that returns the preserved external default `~/.fabro/storage`
- Remove `legacy_default_storage_root()` instead of renaming it in place
- Use `default_storage_dir()` anywhere code currently assumes storage lives under `Home`, including:
  - server settings default storage root
  - workflow/run scratch fallback helpers
  - doctor/install/default-record lookup code
  - uninstall fallback when settings load fails
- After this change, the `Home` struct no longer owns or exposes storage placement; the default storage policy remains home-derived, but it lives in config/user resolution instead of the `Home` API
- Keep `Home` as user-scoped local state keyed by `FABRO_HOME`; telemetry identity and upgrade-check state remain home-scoped and intentionally vary when `FABRO_HOME` varies
- Callers that currently do `legacy_default_storage_root().join("storage")` (e.g. `install.rs`, `server/record.rs`) must drop the trailing `.join("storage")` when switching to `default_storage_dir()`, since the new helper already resolves to `~/.fabro/storage`

### Clean up Storage/Home registry drift

- Add and use a distinct `Storage::slatedb_cache_dir()` accessor for `<storage>/cache/slatedb`
- Keep the existing `Storage::slatedb_dir()` meaning unchanged as `<storage>/objects/slatedb`
- Update the SlateDB cache wiring to use `Storage::slatedb_cache_dir()` instead of joining `cache/slatedb` inline
- Keep dead paths dead: do not add new accessors for removed legacy env/config files

## Test Plan

- Update `Home` tests to reflect the trimmed API
- Add unit coverage for log-sink resolution so these cases are explicit:
  - normal CLI command -> home rolling log
  - `server start --foreground` -> storage server log
  - `server restart --foreground` -> storage server log
  - `server __serve` -> storage server log
  - daemon wrapper commands -> CLI/home log
- Add unit coverage for the pre-tracing bootstrap classification so a restart with `foreground == true` (`ServerCommand::Restart(ServerRestartArgs { foreground: true, .. })`) is handled like the corresponding start-foreground and `Serve` variants
- Add or adjust server-start integration tests to verify:
  - the actual server log file is `<storage>/logs/server.log`
  - server tracing output lands there
  - no server rolling log is created under `Home/logs`
- Update daemon/foreground server-start tests to verify the truncate-on-start + append-during-runtime semantics for `<storage>/logs/server.log`
- Add a concurrency test for serving startup so a second concurrent foreground/serve start fails before any second truncate of `<storage>/logs/server.log`
- Update config/user tests so old `cli.toml`, `user.toml`, and `server.toml` are neither loaded nor warned about
- Update doctor/install/provider-login tests to remove legacy env/config warning expectations
- Update server-record tests and `server_start` fixtures so only `<storage>/server.json` is canonical, and add coverage for the new fail-fast legacy-daemon detector
- Update any workflow/run fallback tests that currently rely on `Home::storage_dir()`
- Update docs/help/fixture expectations that still mention legacy filenames, legacy `.env`, or home-root server record paths

## Assumptions

- Greenfield cleanup: no compatibility fallback, migration shim, warning path, changelog work, or announcement work is retained for old config files, old `~/.fabro/.env`, or old `~/.fabro/server.json`
- The external default storage location remains `~/.fabro/storage`; only the abstraction boundary changes
- Unix socket location remains under `Home`
- Dev token remains the intentional shared-machine exception: the home-level token remains the shared local-machine source, and `<storage>/server.dev-token` remains only the storage-owned mirror of that same token
- Home remains user-scoped local CLI state keyed by `FABRO_HOME`
- Temporary or test-only `FABRO_HOME` values continue to produce throwaway home-scoped telemetry ID / upgrade-check state; this pass does not special-case test homes
- Storage-root server state files remain exactly:
  - `<storage>/server.json`
  - `<storage>/server.lock`
  - `<storage>/server.env`
  - `<storage>/server.dev-token` as the mirrored copy described above, not a second canonical source
