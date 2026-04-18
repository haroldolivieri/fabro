# Feat: gate GitHub webhook exposure behind explicit strategy

Today the Rust server silently spawns a separate webhook listener on a random loopback port, shells out to `tailscale funnel`, and mutates the GitHub App's webhook URL whenever `integrations.github.strategy = "app"` and `integrations.github.webhooks` is present. Side effects are host-wide and externally visible. Require explicit opt-in, move the handler onto the main API router at `POST /api/v1/webhooks/github`, and add a `server_url` strategy for operators with a stable public URL.

## 1. Settings (`fabro-types/src/settings/server.rs`)

- Extend `WebhookStrategy` enum:
  - `TailscaleFunnel` (existing, serde `tailscale_funnel`)
  - `ServerUrl` (new, serde `server_url`)
- `IntegrationWebhooksSettings` shape unchanged: `strategy: Option<WebhookStrategy>`.

## 2. Config resolve (`fabro-config/src/resolve/server.rs`)

- Thread new variant through layer merge.
- Validation: `ServerUrl` requires `server.api.url` to be set. Missing → resolve error at startup.

## 3. OpenAPI + main router

- `docs/api-reference/fabro-api.yaml`: add `POST /api/v1/webhooks/github`.
  - Request body: `application/json`, raw (opaque bytes — verified by HMAC before parse).
  - Responses: `200` OK, `401` unauthorized.
  - No auth security scheme (signature-verified).
- `cargo build -p fabro-api` regenerates Rust types + client.
- `server.rs::build_router_with_options`: mount route outside the auth middleware layer. All `AuthMode` values still serve the endpoint; HMAC is the only gate.
- Handler: reject missing/invalid `X-Hub-Signature-256` with 401; on success parse metadata, log event, 200.
- Webhook secret plumbed into `AppState` from `server_secrets["GITHUB_APP_WEBHOOK_SECRET"]`. Route is mounted iff secret is present.

## 4. Rename + slim `github_webhooks.rs`

- Rename `WebhookManager` → `TailscaleFunnelManager` (single-purpose now).
- Delete `spawn_webhook_listener`, `WebhookListener`, standalone router — handler lives on main router.
- `TailscaleFunnelManager::start(main_server_port, app_id, private_key_pem)`:
  - `tailscale funnel <main_server_port>`
  - Parse funnel URL from `tailscale funnel status`
  - Best-effort `PATCH https://api.github.com/app/hook/config` with `{funnel_url}/api/v1/webhooks/github`
  - Log but don't fail on GitHub API errors
- `shutdown()`: `tailscale funnel off <port>`.
- Keep `verify_signature` and `parse_event_metadata` as `pub(crate)` helpers consumed by the new main-router handler.

## 5. `serve.rs` gating (`fabro-server/src/serve.rs:373-422`)

Match on `resolved_server_settings.integrations.github.webhooks.strategy`:

- `None` → no-op. (Route still mounted if secret present, but no funnel, no GitHub App mutation.)
- `Some(TailscaleFunnel)` → `TailscaleFunnelManager::start(main_port, app_id, private_key_pem)`. Keep current best-effort error handling.
- `Some(ServerUrl)` → best-effort `update_github_app_webhook({server.api.url}/api/v1/webhooks/github)`. Log failure, don't fail startup. No funnel, no manager.

Requires wiring the main server's bound port into this block.

## 6. Tests

- Router test (new, in `server.rs` or adjacent module):
  - Valid HMAC → 200
  - Missing `X-Hub-Signature-256` → 401
  - Invalid signature → 401
  - Works regardless of `AuthMode` (bearer header absent, wrong, and correct all behave identically for this route)
- Delete `spawn_listener_serves_route` in `github_webhooks.rs` tests.
- Settings resolve test: `strategy = "server_url"` without `server.api.url` → error.
- Existing OpenAPI conformance test picks up spec addition automatically.

## 7. Docs

- `docs/integrations/github.mdx`: document both strategies, side effects (funnel, App URL mutation), recommended choice per environment.
- `docs/administration/server-configuration.mdx`: reference `[server.integrations.github.webhooks] strategy` field.
- `docs/changelog/2026-04-18.mdx`: **breaking** — funnel no longer auto-enables on App strategy with webhooks config present. Operators must set `strategy = "tailscale_funnel"` explicitly to restore prior behavior. New `server_url` strategy for production deployments with a stable public URL.

## 8. Migration

- None automated. Existing users implicitly relying on auto-funnel must add `strategy = "tailscale_funnel"` to `[server.integrations.github.webhooks]`. Call out in changelog.

## Unresolved questions

1. Is `AppState` the right home for the webhook secret, or should it be injected into the handler via router state layering?
2. Should `ServerUrl`'s GitHub App URL update run on every `serve.rs` start, or only when the current App URL differs? (Rate-limit/noise concern.)
3. Where does the webhook route sit relative to `web_enabled` gating — always on, or only when `integrations.github.enabled`?
