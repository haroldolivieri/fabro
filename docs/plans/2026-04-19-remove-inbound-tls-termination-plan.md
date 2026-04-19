# Remove Inbound TLS Termination From `fabro-server`

## Summary

- Remove all server-side TLS termination from `fabro-server` so Fabro only listens on plain TCP or Unix sockets.
- Make reverse proxy / load balancer TLS termination the only supported deployment model for HTTPS.
- Treat this as a deliberate breaking config change: `[server.listen.tls]` is removed, not retained behind a compatibility layer.

## Public Interface Changes

- Simplify server listen settings to `Tcp { address }` or `Unix { path }`; remove the `tls` field from both sparse and resolved server settings.
- Delete the inbound-listener TLS types from `fabro-types` and stop re-exporting them.
- Remove `server.listen.tls` resolution from `fabro-config`.
- Existing configs that still set `[server.listen.tls]` should fail during settings parsing via the current unknown-field behavior; do not add a migration shim.
- Update published docs/examples so HTTPS is described as proxy-terminated, with public HTTPS URLs still expressed through `[server.api].url` and `[server.web].url`.

## Implementation Changes

- Server runtime:
  - Delete the Rustls listener module and remove the TLS branch from startup.
  - Keep the plain TCP and Unix socket serve paths intact.
  - Preserve `ConnectInfo` and IP allowlist behavior for non-TLS TCP serving.
  - Remove inbound-TLS-only dependencies from `fabro-server`.
- Diagnostics and settings views:
  - Delete server listener cert/key diagnostics.
  - Keep redacting `server.listen` from settings APIs, since bind addresses are still treated as host topology.
  - Update the redaction comments/tests so they no longer reference TLS cert/key paths.
- Tests and fixtures:
  - Delete the Linux-only inbound TLS integration module and its fixture directory.
  - Replace config-resolution tests that currently validate `server.listen.tls.cert/key` with tests asserting the removed `tls` field is rejected.
  - Update settings/runs API tests to remove TLS fixture blocks while keeping the `server.listen` redaction assertions.
- Docs:
  - Update active docs that currently advertise `[server.listen.tls]`.
  - Leave archival brainstorm/plan docs unchanged.

## Test Plan

- `cargo build --workspace`
- `cargo nextest run -p fabro-config`
- `cargo nextest run -p fabro-server`
- `cargo +nightly-2026-04-14 fmt --check --all`
- `cargo +nightly-2026-04-14 clippy -p fabro-config -p fabro-server --all-targets -- -D warnings`
- Cover these scenarios in tests:
  - TCP and Unix listener configs still parse, resolve, and serve correctly.
  - Configs containing `[server.listen.tls]` now fail with an unknown-field style parse error.
  - Settings API responses still redact `server.listen`.

## Assumptions

- Outbound/client TLS in `fabro-cli` remains unchanged.
- Hook/integration TLS settings remain unchanged.
- Proxy-aware behavior such as `X-Forwarded-Proto` handling remains in place.
- No automatic migration or deprecation layer will be added for old listener TLS configs.
