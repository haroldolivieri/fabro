---
title: "feat: Add IP address allowlist to server"
type: feat
status: active
date: 2026-04-15
origin: docs/brainstorms/2026-04-15-ip-whitelist-requirements.md
deepened: 2026-04-18
---

# feat: Add IP address allowlist to server

## Overview

Add configurable IP address allowlisting to the fabro server so users without network-level controls (VPN, firewall) can restrict access by client IP. The allowlist is configured via `[server.ip_allowlist]` in settings.toml, validated at startup (fail-closed), and enforced as request middleware after tracing but before auth. To preserve TOML compatibility for an upcoming GitHub webhook endpoint, the config shape also reserves an optional GitHub-specific webhook overlay at `[server.integrations.github.webhooks.ip_allowlist]` plus a reserved `github_meta_hooks` entry keyword for GitHub-origin webhook traffic.

## Problem Frame

Some fabro users deploy on infrastructure without firewalls or VPNs. They need fabro itself to restrict which IP addresses can reach the web UI and API. (see origin: docs/brainstorms/2026-04-15-ip-whitelist-requirements.md)

## Requirements Trace

- R1. Non-allowlisted IPs receive 403 before auth, for all requests (API, web UI, static assets)
- R2. No allowlist configured = all IPs allowed (current behavior)
- R3. Health check endpoint `/health` is exempt from IP filtering
- R4. Support individual IPv4 and IPv6 addresses
- R5. Support CIDR notation for ranges
- R6. Detect client IP from X-Forwarded-For behind reverse proxies (X-Real-IP is not used — no positional trust model)
- R7. Proxy header trust must be explicitly configured; default uses TCP remote address
- R8. Configured via settings.toml / environment variables only
- R9. No hot-reload; read once at startup
- R10. Log rejected requests at warn level with client IP and path
- R11. Unix socket + allowlist requires trusted_proxy_count > 0 or server refuses to start
- R12. Invalid entries (malformed CIDR) cause startup failure (fail-closed)
- R13. The TOML schema must support provider-specific webhook IP allowlist overrides in integration-owned namespaces without breaking existing `[server.ip_allowlist]` configs
- R14. Webhook-specific entries may include the reserved keyword `github_meta_hooks`, which resolves to GitHub's current webhook delivery ranges from `GET /meta`
- R15. `github_meta_hooks` is only valid in `[server.integrations.github.webhooks.ip_allowlist].entries`; using it outside the GitHub webhook allowlist is a config error
- R16. `[server.integrations.github.webhooks.ip_allowlist].trusted_proxy_count` inherits from `[server.ip_allowlist].trusted_proxy_count` unless explicitly overridden
- R17. GitHub webhook IP allowlist config falls back to `[server.ip_allowlist]` when the GitHub-specific block or one of its fields is omitted, so webhook routes inherit the server allowlist by default

## Scope Boundaries

- No web UI management
- No per-user/per-role IP rules
- No blocklist (deny-list)
- No hot-reload of IP rules
- No implicit loopback exemption
- This plan does not add the webhook endpoint itself; it only reserves the config and resolution shape needed so GitHub webhook-specific enforcement can be wired later without a TOML break
- `/health/diagnostics` is NOT exempt (requires auth, probes server internals)

## Context & Research

### Relevant Code and Patterns

- **Config layer/resolved pattern**: `ServerLayer` (sparse, `deny_unknown_fields`) -> `resolve_server()` -> `ServerSettings` (fully populated) in `lib/crates/fabro-types/src/settings/server.rs` and `lib/crates/fabro-config/src/resolve/server.rs`
- **Nested integration webhook config already exists**: GitHub webhook delivery strategy already lives under `[server.integrations.github.webhooks]`, so a provider-specific IP allowlist fits the existing integration-owned namespace
- **Middleware pattern**: `middleware::from_fn_with_state` used for `cookie_and_demo_middleware` at `lib/crates/fabro-server/src/server.rs:971`
- **Router structure**: `/health` registered on outer router (line 948), dispatch service_fn routes to demo/real sub-routers (line 914)
- **Serve paths**: Three listener branches in `lib/crates/fabro-server/src/serve.rs:492` — Unix, TCP+TLS, TCP plain. None currently use `ConnectInfo`
- **TLS accept loop**: `lib/crates/fabro-server/src/tls.rs:61` captures `remote_addr` but does not inject it into requests
- **Auth mode resolution**: `auth_mode` is resolved once at startup (serve.rs:315) and passed to the router, not hot-reloaded — same pattern for IP allowlist
- **Test pattern**: `tower::ServiceExt::oneshot()` with `Request::builder()` in server.rs test module (line 6561+)

### External References

- `ipnet` crate (v2.11.0 already transitive dep) — `IpNet::contains(&IpAddr)` for CIDR matching
- Axum `ConnectInfo<SocketAddr>` via `into_make_service_with_connect_info::<SocketAddr>()`
- X-Forwarded-For rightmost-minus-N: the secure approach is to count N trusted proxies from the right of the comma-separated list
- GitHub `GET /meta` returns service-specific IP ranges including `hooks`; GitHub recommends querying it directly for current webhook source ranges

## Key Technical Decisions

- **Custom proxy IP extraction over `axum-client-ip`**: The crate's `RightmostXForwardedFor` takes only the single rightmost value, not rightmost-minus-N. Since R7 specifies trusted proxy count support, a simple custom extractor is more appropriate than adding a dependency that doesn't cover the requirement
- **No X-Real-IP fallback**: X-Real-IP has no positional trust model — any client can forge it. When `trusted_proxy_count > 0`, only X-Forwarded-For is used for IP extraction. If the header is absent or has too few entries, the request is rejected (fail-closed)
- **Fail-closed on short X-Forwarded-For chain**: If `X-Forwarded-For` has fewer entries than `trusted_proxy_count`, the request is rejected (403) rather than falling back to a potentially spoofable entry
- **Generic 403 response body**: Use `"Access denied."` (matching existing `ApiError::forbidden()`) rather than a message that reveals the filtering mechanism
- **TLS ConnectInfo must use Axum newtype**: The TLS accept loop must inject `axum::extract::ConnectInfo<SocketAddr>` (the wrapper type), not a bare `SocketAddr`, or the middleware extractor will silently fail
- **`ipnet` as direct dependency**: Present in Cargo.lock as a transitive dependency (v2.11.0); added as a direct dependency to fabro-types, fabro-config, and fabro-server for CIDR parsing and matching
- **`[server.ip_allowlist]` remains the global default scope, and GitHub webhook overrides live under `[server.integrations.github.webhooks.ip_allowlist]`**: This matches the existing integration-owned webhook config shape and avoids inventing a parallel top-level webhook namespace
- **The GitHub webhook allowlist is an overlay on the global allowlist, not a separate root policy**: If `[server.integrations.github.webhooks.ip_allowlist]` is absent, GitHub webhook requests use `[server.ip_allowlist]`. If the GitHub block is present but omits `entries` or `trusted_proxy_count`, the omitted field inherits from `[server.ip_allowlist]`
- **`entries` resolves to a typed enum, not raw strings**: User-provided strings parse into `IpAllowEntry`, with `Literal(IpNet)` for direct IP/CIDR values and `GitHubMetaHooks` for the reserved keyword `github_meta_hooks`
- **`github_meta_hooks` is GitHub-webhook-only**: The reserved keyword is accepted only in `[server.integrations.github.webhooks.ip_allowlist].entries`, where it means "expand to GitHub's current `/meta` `hooks` ranges". Using it elsewhere is invalid config
- **GitHub meta expansion should use startup-time fetch plus cache-aware reuse**: Resolve `github_meta_hooks` at startup, using `GET /meta` and the `hooks` array. Reuse cached data on `304 Not Modified`; if GitHub is unavailable and there is no usable cache, fail startup
- **Middleware after TraceLayer, before cookie middleware**: Rejected requests still get trace spans (including 403 status), but no cookie/demo processing for blocked IPs
- **Health exemption inside middleware**: The middleware checks the request path for `/health` rather than selectively applying to route sets, keeping router structure simple
- **IPv4-mapped IPv6 normalization**: Convert `::ffff:x.x.x.x` to IPv4 equivalent before matching, so `10.0.0.0/8` matches regardless of how the OS reports the address
- **Land the schema before the webhook route if convenient**: The default-scope middleware can ship first. The GitHub-specific webhook overlay and `github_meta_hooks` resolution model should land now so the later webhook feature can wire into it without a TOML migration

## Open Questions

### Resolved During Planning

- **Which Axum ecosystem tools for IP detection?** Custom implementation. `axum-client-ip` doesn't support trusted-proxy-count. `ConnectInfo<SocketAddr>` for TCP remote address, manual X-Forwarded-For parsing for proxy support
- **Config shape?** `[server.ip_allowlist]` is the global default scope. `[server.integrations.github.webhooks.ip_allowlist]` is an optional GitHub-specific overlay, which fits the existing integration-owned webhook namespace and `deny_unknown_fields` constraint
- **How should GitHub webhook origins be represented?** As the reserved keyword `github_meta_hooks` inside `[server.integrations.github.webhooks.ip_allowlist].entries`, resolved into concrete IP nets from GitHub `GET /meta`
- **Which health endpoints to exempt?** Only `/health`. Not `/health/diagnostics` (requires auth, exposes server internals)
- **IPv4-mapped IPv6 handling?** Normalize to IPv4 before matching
- **X-Forwarded-For position?** Rightmost-minus-N where N = `trusted_proxy_count`. If N=0, use TCP remote address (ConnectInfo)

### Deferred to Implementation

- **Cross-domain config validation sequencing**: The Unix socket + allowlist check requires both `listen` and `ip_allowlist` to be resolved. Perform this check in `resolve_server()` after both `resolve_listen()` and `resolve_ip_allowlist()` return, not inside the IP allowlist resolver itself
- **Whether `ipnet` version needs bumping**: Currently 2.11.0 as transitive; verify `contains()` API availability at implementation time

## High-Level Technical Design

> *This illustrates the intended approach and is directional guidance for review, not implementation specification. The implementing agent should treat it as context, not code to reproduce.*

```
Representative TOML:

  [server.ip_allowlist]
  entries = ["10.0.0.0/8"]
  trusted_proxy_count = 1

  [server.integrations.github]
  strategy = "app"
  app_id = "123456"
  client_id = "Iv1.abc123"
  slug = "fabro-app"

  [server.integrations.github.webhooks]
  strategy = "tailscale_funnel"

  [server.integrations.github.webhooks.ip_allowlist]
  entries = ["github_meta_hooks"]
  # trusted_proxy_count inherits as 1 unless overridden here

Request flow with IP allowlist:

  Client -> [TCP/TLS Accept] -> ConnectInfo<SocketAddr> injected
         -> [TraceLayer]     -> trace span created
         -> [IP Allowlist]   -> select active scope
                                  - current routes: default `[server.ip_allowlist]`
                                  - future GitHub webhook route:
                                    `[server.integrations.github.webhooks.ip_allowlist]`
                                    overlaid on `[server.ip_allowlist]`
                                if path == "/health": pass through
                                else: extract client IP
                                  (ConnectInfo or X-Forwarded-For rightmost-minus-N)
                                  normalize IPv4-mapped IPv6
                                  check against Vec<IpNet>
                                  if miss: warn!(...), return 403
         -> [Cookie/Demo]    -> parse cookies, demo header
         -> [Router]         -> dispatch to demo/real sub-router
         -> [Auth extractor] -> per-handler auth check

Startup validation:
  settings.toml -> parse [server.ip_allowlist]
    -> parse optional [server.integrations.github.webhooks.ip_allowlist]
    -> parse each entry string into IpAllowEntry
       - IP/CIDR -> Literal(IpNet)
       - `github_meta_hooks` -> GitHubMetaHooks
    -> reject `github_meta_hooks` outside the GitHub webhook allowlist
    -> when resolving a GitHub webhook scope containing `GitHubMetaHooks`,
       call `GET /meta`, read `hooks`, parse into IpNet list
    -> for GitHub webhook requests, build the effective scope by overlaying
       `[server.integrations.github.webhooks.ip_allowlist]` on
       `[server.ip_allowlist]`
    -> if Unix socket && active scope has entries && trusted_proxy_count == 0:
       error
    -> resolve to IpAllowlistConfig (Arc, read-once)
    -> pass to build_router_with_options
```

## Implementation Units

- [ ] **Unit 1: Config types and resolution**

**Goal:** Define the settings.toml schema for `[server.ip_allowlist]` plus the GitHub webhook overlay, and wire up typed entry parsing/validation.

**Requirements:** R4, R5, R8, R11, R12, R13, R14, R15, R16, R17

**Dependencies:** None

**Files:**
- Modify: `lib/crates/fabro-types/src/settings/server.rs`
- Modify: `lib/crates/fabro-config/src/resolve/server.rs`
- Modify: `lib/crates/fabro-config/Cargo.toml` (add `ipnet` direct dependency)
- Modify: `lib/crates/fabro-types/Cargo.toml` (add `ipnet` for the resolved type)
- Test: `lib/crates/fabro-config/src/resolve/server.rs` (inline tests)

**Approach:**
- Add `ServerIpAllowlistLayer` for the root `[server.ip_allowlist]` table with `entries: Option<Vec<String>>` and `trusted_proxy_count: Option<u32>`. Apply `#[serde(deny_unknown_fields)]`
- Add `ServerIpAllowlistOverrideLayer` with the same fields for provider-local webhook overlays
- Add `ip_allowlist: Option<ServerIpAllowlistLayer>` to `ServerLayer`
- Add `ip_allowlist: Option<ServerIpAllowlistOverrideLayer>` to `IntegrationWebhooksLayer` so GitHub webhook config can live at `[server.integrations.github.webhooks.ip_allowlist]`
- Add `IpAllowEntry` enum with `Literal(IpNet)` and `GitHubMetaHooks`
- Add `ServerIpAllowlistSettings` with `entries: Vec<IpAllowEntry>` and `trusted_proxy_count: u32` for the global scope
- Add `ServerIpAllowlistOverrideSettings` with `entries: Option<Vec<IpAllowEntry>>` and `trusted_proxy_count: Option<u32>` for GitHub webhook overlay settings
- Default when absent: empty `entries` vec, `trusted_proxy_count: 0`
- Add `resolve_ip_allowlist()` to parse the global scope into `ServerIpAllowlistSettings`
- Add `resolve_ip_allowlist_override()` to parse provider-local overlays into `ServerIpAllowlistOverrideSettings`, preserving omitted fields as `None`
- Extend `resolve_integrations()` to parse `server.integrations.github.webhooks.ip_allowlist` into the resolved GitHub webhook settings
- Reject `github_meta_hooks` outside the GitHub webhook allowlist path
- Add `ip_allowlist` field to `ServerSettings`, and extend the resolved integration webhook settings to carry the optional overlay
- Perform cross-domain validation in `resolve_server()` after `resolve_listen()`, `resolve_ip_allowlist()`, and `resolve_integrations()` return: compute the effective GitHub webhook scope by overlaying the GitHub webhook config on the global allowlist, then check Unix socket + effective scope with non-empty entries + resolved `trusted_proxy_count == 0` → push error

**Patterns to follow:**
- `ServerAuthLayer` / `ServerAuthSettings` in same file for layer/resolved struct pattern
- `IntegrationWebhooksLayer` / `IntegrationWebhooksSettings` in the same file for nested integration-owned config
- `resolve_auth()` in `lib/crates/fabro-config/src/resolve/server.rs` for validation with `ResolveError`

**Test scenarios:**
- Happy path: valid IPv4, IPv6, and CIDR entries parse correctly into `IpAllowEntry::Literal`
- Happy path: absent `[server.ip_allowlist]` section resolves to empty entries (R2)
- Happy path: `trusted_proxy_count` defaults to 0 when not specified
- Happy path: `[server.integrations.github.webhooks.ip_allowlist]` resolves as an optional overlay independent from the global scope
- Happy path: GitHub webhook entries fall back to the global scope when the GitHub block is absent
- Happy path: GitHub webhook entries fall back to the global scope when the GitHub block omits `entries`
- Happy path: `server.integrations.github.webhooks.ip_allowlist.trusted_proxy_count` inherits the global value when omitted
- Happy path: `server.integrations.github.webhooks.ip_allowlist.trusted_proxy_count` overrides the global value when explicitly set
- Happy path: `github_meta_hooks` is accepted in `server.integrations.github.webhooks.ip_allowlist.entries`
- Error path: malformed CIDR string (e.g., `10.0.0.0/33`) produces ResolveError
- Error path: unparseable address (e.g., `not-an-ip`) produces ResolveError
- Error path: `github_meta_hooks` in the default scope produces ResolveError
- Error path: `github_meta_hooks` outside the GitHub webhook allowlist path produces ResolveError
- Error path: Unix socket listener + non-empty allowlist + `trusted_proxy_count: 0` produces ResolveError
- Edge case: empty `entries` list (present but empty) resolves to empty vec (equivalent to no filtering)

**Verification:**
- `cargo nextest run -p fabro-config` passes
- `cargo nextest run -p fabro-types` passes
- Invalid config entries cause resolution errors, not panics

- [ ] **Unit 2: Effective scope resolution, entry expansion, IP matching, and client IP extraction**

**Goal:** Create the core IP matching logic, effective scope overlay logic, dynamic entry expansion, and client IP extraction (from ConnectInfo or proxy headers).

**Requirements:** R4, R5, R6, R7, R14, R17

**Dependencies:** Unit 1 (needs `ServerIpAllowlistSettings`)

**Files:**
- Create: `lib/crates/fabro-server/src/ip_allowlist.rs`
- Modify: `lib/crates/fabro-server/src/lib.rs` (add module declaration)
- Modify: `lib/crates/fabro-server/Cargo.toml` (add `ipnet` direct dependency)
- Test: `lib/crates/fabro-server/src/ip_allowlist.rs` (inline test module)
- Test fixture or mock HTTP support as needed for GitHub `/meta` resolution tests

**Approach:**
- `IpAllowlist` struct wrapping `Vec<IpNet>` with a `contains(&IpAddr) -> bool` method
- Add a helper that computes an effective scope from the global allowlist plus an optional provider-specific overlay:
  - `entries = override.entries.unwrap_or(global.entries.clone())`
  - `trusted_proxy_count = override.trusted_proxy_count.unwrap_or(global.trusted_proxy_count)`
- Add a resolution step that turns the effective scope's `Vec<IpAllowEntry>` into concrete `Vec<IpNet>` for a chosen route:
  - `Literal(IpNet)` passes through directly
  - `GitHubMetaHooks` fetches GitHub `GET /meta`, reads the `hooks` array, and parses each item as `IpNet`
- Use cache-aware startup fetch for `GitHubMetaHooks`: send `If-None-Match` when cache exists, reuse cached ranges on `304`, and fail startup when the source cannot be resolved and no usable cache exists
- Before matching, normalize IPv4-mapped IPv6 (`::ffff:x.x.x.x` → IPv4 equivalent) using `to_canonical()` or manual mapping
- `extract_client_ip(req, trusted_proxy_count) -> Option<IpAddr>` function:
  - If `trusted_proxy_count > 0`: parse `X-Forwarded-For` header, split by comma, take the entry at zero-indexed position `len - 1 - trusted_proxy_count` (skipping N trusted proxy entries from the right to reach the client IP). If `X-Forwarded-For` is absent or has fewer entries than `trusted_proxy_count + 1`, return `None` (fail-closed)
  - If `trusted_proxy_count == 0`: extract `ConnectInfo<SocketAddr>` from request extensions, return `addr.ip()`
- `IpAllowlist::is_empty()` method to efficiently skip checking when no allowlist is configured (R2)
- `IpAllowlistConfig` struct containing `IpAllowlist` + `trusted_proxy_count: u32`, constructed from the effective scope for a chosen route. This is the type used as middleware state (`Arc<IpAllowlistConfig>`) in Units 3, 4, and the later GitHub webhook wiring step

**Patterns to follow:**
- `lib/crates/fabro-server/src/jwt_auth.rs` for module structure and inline test organization

**Test scenarios:**
- Happy path: IPv4 address matches an individual IPv4 entry
- Happy path: IPv4 address matches a CIDR range (e.g., `10.1.2.3` matches `10.0.0.0/8`)
- Happy path: IPv6 address matches an IPv6 CIDR range
- Happy path: effective GitHub webhook scope falls back to global entries when the GitHub overlay is absent
- Happy path: effective GitHub webhook scope falls back to global entries when the GitHub overlay omits `entries`
- Happy path: effective GitHub webhook scope inherits the global `trusted_proxy_count` when the GitHub overlay omits it
- Happy path: effective GitHub webhook scope overrides `trusted_proxy_count` when the GitHub overlay sets it
- Happy path: `GitHubMetaHooks` expands to the `hooks` ranges from a mocked `/meta` response
- Happy path: cached GitHub metadata is reused on `304 Not Modified`
- Happy path: `extract_client_ip` with `trusted_proxy_count: 0` returns ConnectInfo IP
- Happy path: `extract_client_ip` with `trusted_proxy_count: 1` and header `client, proxy1` returns `client` (second-from-right, skipping 1 trusted proxy)
- Happy path: `extract_client_ip` with `trusted_proxy_count: 2` and header `client, proxy1, proxy2` returns `client` (third-from-right, skipping 2 trusted proxies)
- Edge case: IPv4-mapped IPv6 (`::ffff:10.1.2.3`) matches plain IPv4 entry `10.0.0.0/8`
- Edge case: X-Forwarded-For with fewer entries than `trusted_proxy_count` — returns None (fail-closed)
- Edge case: X-Forwarded-For entries with whitespace around commas are trimmed
- Edge case: empty allowlist `is_empty()` returns true
- Edge case: single host IP as `/32` CIDR matches exactly
- Error path: GitHub `/meta` response contains an invalid range — resolution fails closed
- Error path: GitHub `/meta` is unavailable and there is no usable cache — startup fails
- Error path: missing ConnectInfo extension when `trusted_proxy_count: 0` — returns None
- Error path: malformed IP in X-Forwarded-For — returns None (fail-closed)
- Error path: X-Forwarded-For absent when `trusted_proxy_count > 0` — returns None (no X-Real-IP fallback)

**Verification:**
- `cargo nextest run -p fabro-server -- ip_allowlist` passes
- All matching and extraction edge cases covered

- [ ] **Unit 3: IP filter middleware**

**Goal:** Create the Axum middleware function that enforces the IP allowlist on incoming requests.

**Requirements:** R1, R2, R3, R10

**Dependencies:** Unit 2 (needs `IpAllowlist` and `extract_client_ip`)

**Files:**
- Modify: `lib/crates/fabro-server/src/ip_allowlist.rs` (add middleware function)
- Test: `lib/crates/fabro-server/src/ip_allowlist.rs` (middleware integration tests in inline module)

**Approach:**
- `ip_allowlist_middleware` async function with signature compatible with `middleware::from_fn_with_state`:
  - State: `Arc<IpAllowlistConfig>` containing the `IpAllowlist` and `trusted_proxy_count`
  - If allowlist is empty: call `next.run(req).await` immediately (R2 — zero overhead when unconfigured)
  - If request path is `/health`: pass through regardless (R3)
  - Extract client IP using `extract_client_ip()`
  - If IP is in allowlist: pass through
  - If IP is not in allowlist or extraction failed: `warn!(client_ip = %ip, path = %path, "request rejected: IP not in allowlist")`, return 403 response. Log only the parsed `IpAddr`, not raw header values, to prevent log injection
- Return `ApiError::forbidden().into_response()` (generic "Access denied." — does not reveal IP filtering as the mechanism; `.into_response()` needed since `from_fn_with_state` middleware must return `Response`)

**Patterns to follow:**
- `cookie_and_demo_middleware` at `lib/crates/fabro-server/src/server.rs:1994` for `from_fn_with_state` middleware signature and request inspection
- Logging strategy: `warn!` with structured fields per `docs-internal/logging-strategy.md`

**Test scenarios:**
- Happy path: request from allowlisted IP passes through to handler
- Happy path: request to `/health` from any IP passes through (R3)
- Happy path: empty allowlist allows all requests (R2)
- Error path: request from non-allowlisted IP returns 403 with appropriate body
- Error path: request with no extractable IP (missing ConnectInfo, no proxy headers) returns 403
- Integration: middleware runs before auth — a blocked IP never reaches auth extractor
- Edge case: request to `/health/diagnostics` from non-whitelisted IP returns 403 (not exempt)
- Edge case: request to `/health/` (trailing slash) from non-whitelisted IP returns 403 (not exempt — exact path match only)
- Edge case: static asset request from non-whitelisted IP returns 403

**Verification:**
- `cargo nextest run -p fabro-server -- ip_allowlist` passes
- Rejected requests logged at warn level with structured fields

- [ ] **Unit 4: Router and serve integration**

**Goal:** Wire the default-scope IP allowlist middleware into the router and enable ConnectInfo extraction in all serve paths.

**Requirements:** R1, R7, R9

**Dependencies:** Unit 3 (needs middleware function), Unit 1 (needs resolved config)

**Files:**
- Modify: `lib/crates/fabro-server/src/server.rs` (add middleware layer in `build_router_with_options`, update `build_router()` convenience wrapper to pass default empty allowlist)
- Modify: `lib/crates/fabro-server/src/serve.rs` (read allowlist config at startup, pass to router, update TCP serve call to use `into_make_service_with_connect_info`)
- Modify: `lib/crates/fabro-server/src/tls.rs` (inject `ConnectInfo<SocketAddr>` into request extensions in TLS accept loop)
- Test: `lib/crates/fabro-server/src/server.rs` (integration tests in existing test module)

**Approach:**
- In `serve.rs` `serve_command()`: resolve the default-scope IP allowlist config from `resolved_server_settings` once at startup (like `auth_mode` at line 315). Wrap in `Arc<IpAllowlistConfig>` and pass to `build_router_with_options` as a separate parameter (not in `RouterOptions`, to keep it lightweight and `Copy`-compatible — mirrors how `auth_mode` is passed separately)
- In `server.rs` `build_router_with_options()`: apply `middleware::from_fn_with_state(ip_allowlist_config, ip_allowlist_middleware)` as a layer AFTER `trace_layer` but BEFORE `cookie_and_demo_middleware`. Layer ordering (outermost to innermost): TraceLayer → IP allowlist → cookie/demo → router
- In `serve.rs` TCP plain path (line 519): change `axum::serve(listener, router)` to `axum::serve(listener, router.into_make_service_with_connect_info::<SocketAddr>())`
- In `tls.rs` accept loop (line 78): inject `axum::extract::ConnectInfo<SocketAddr>` (the Axum newtype wrapper, NOT a bare `SocketAddr`) as a request extension before calling the router. If the wrong type is inserted, `extract_client_ip` will silently fail and all requests will be rejected
- In `serve.rs` Unix socket path (line 498): no ConnectInfo change needed — the startup validation (Unit 1) ensures `trusted_proxy_count > 0` when using Unix socket with an allowlist, so the middleware will use proxy headers
- IP allowlist config is NOT added to `AppState` or `shared_settings` — it is captured by the middleware closure at construction time, ensuring R9 (no hot-reload)
- Do not wire `server.integrations.github.webhooks.ip_allowlist` here yet unless the GitHub webhook route already exists in the same change; the root middleware should continue to represent the global default scope for current routes

**Patterns to follow:**
- `auth_mode` resolution at `serve.rs:315` for resolve-once-at-startup pattern
- `cookie_and_demo_middleware` layer application at `server.rs:971` for middleware wiring
- `auth_mode` parameter passing for how `build_router_with_options` accepts separate config

**Test scenarios:**
- Happy path: full router with allowlist configured blocks non-allowlisted IP, allows allowlisted IP
- Happy path: full router without allowlist configured allows all IPs
- Happy path: `/health` accessible from any IP when allowlist is configured
- Integration: ConnectInfo extension is available in request when using TCP serve
- Integration: IP filtering works with demo mode — blocked IPs cannot access demo routes
- Integration: existing tests with non-empty allowlist must inject `ConnectInfo<SocketAddr>` into request extensions via `Request::builder().extension(ConnectInfo(...))`, or fail-closed will reject all requests
- Edge case: TLS path injects ConnectInfo correctly so IP filtering works over HTTPS

**Verification:**
- `cargo nextest run -p fabro-server` passes (all existing tests + new integration tests)
- `cargo clippy --workspace -- -D warnings` passes
- `cargo +nightly fmt --check --all` passes

- [ ] **Unit 5: Webhook scope wiring**

**Goal:** When the GitHub webhook endpoint is added, route those requests through the GitHub-specific overlay without changing the TOML schema.

**Requirements:** R1, R7, R9, R13, R14, R15, R16, R17

**Dependencies:** Unit 1 (typed config), Unit 2 (dynamic entry resolution), webhook endpoint implementation

**Files:**
- Modify: `lib/crates/fabro-server/src/server.rs`
- Modify: `lib/crates/fabro-server/src/github_webhooks.rs` and/or webhook handler/router files added by the webhook feature
- Test: GitHub webhook integration tests alongside the webhook endpoint

**Approach:**
- Select the active allowlist scope by route:
  - normal routes use `resolved_server_settings.ip_allowlist`
  - GitHub webhook routes compute an effective scope by overlaying `resolved_server_settings.integrations.github.webhooks.ip_allowlist` on `resolved_server_settings.ip_allowlist`
- Reuse the same middleware and `IpAllowlistConfig` shape; only the chosen scope changes
- If the GitHub webhook scope contains `GitHubMetaHooks`, resolve it at startup the same way as any other typed entry before constructing the webhook middleware state
- Keep the GitHub webhook route's HMAC verification; IP allowlisting is additive defense, not a replacement

**Patterns to follow:**
- Reuse the default-scope middleware wiring rather than introducing a second bespoke path
- Keep route selection explicit near the router definition so reviewers can see which requests use which scope

**Test scenarios:**
- Happy path: GitHub webhook request uses the global allowlist when `server.integrations.github.webhooks.ip_allowlist` is absent
- Happy path: GitHub webhook request uses global entries when the GitHub overlay omits `entries`
- Happy path: GitHub webhook request uses the inherited global `trusted_proxy_count` when the GitHub overlay omits it
- Happy path: GitHub webhook request uses the overlay `trusted_proxy_count` when explicitly configured
- Happy path: `github_meta_hooks` allows a GitHub webhook request from a GitHub hook range
- Error path: GitHub webhook request from a non-allowlisted IP returns 403 before webhook auth/HMAC handling
- Integration: non-webhook routes continue using the global scope even when GitHub webhook IP allowlist config is present

**Verification:**
- GitHub webhook integration tests pass once the endpoint exists
- Existing non-webhook allowlist tests still pass unchanged

## System-Wide Impact

- **Interaction graph:** The default middleware sits between TraceLayer and cookie/demo middleware. It reads `ConnectInfo` extensions (set by `into_make_service_with_connect_info` or manually in TLS path). The GitHub webhook-specific overlay reuses the same machinery once the GitHub webhook route exists
- **Error propagation:** 403 responses from the middleware short-circuit the request pipeline. They appear in TraceLayer output as normal 403 responses. The middleware also emits its own `warn!` log
- **State lifecycle risks:** None — the allowlist is immutable after startup (Arc, no hot-reload). No shared mutable state
- **API surface parity:** The global scope applies uniformly to all current routes (except `/health`). A later GitHub webhook route can opt into the reserved provider-local overlay without changing the global config surface
- **Integration coverage:** Key scenario: full request through TCP listener → ConnectInfo injection → IP filter → handler. This must be tested via `oneshot()` with manually injected `ConnectInfo` extensions
- **Unchanged invariants:** Auth middleware (extractors), demo mode dispatch, SSE streaming, and existing API routes remain unchanged. The GitHub webhook endpoint is still added separately; this plan only reserves and later wires the scope it will use

## Risks & Dependencies

| Risk | Mitigation |
|------|------------|
| `into_make_service_with_connect_info` changes the `axum::serve` return type | Straightforward migration; Axum docs show the pattern. Test all serve paths |
| TLS path ConnectInfo injection uses wrong type (bare `SocketAddr` instead of `ConnectInfo<SocketAddr>`) | Explicitly documented constraint in Unit 4 approach; dedicated test verifies extraction works through TLS path |
| IPv4-mapped IPv6 normalization bugs | Comprehensive test coverage in Unit 2 with explicit mapped-address test cases |
| Existing tests may break if they rely on the serve signature | Tests use `oneshot()` against the router, not `axum::serve`, so they are unaffected |
| `github_meta_hooks` makes startup depend on an external service | Use GitHub's ETag-capable `/meta` endpoint with cached reuse; fail closed only when the keyword is configured and no usable data can be resolved |
| A future GitHub webhook-specific schema could force a TOML migration | Reserve `[server.integrations.github.webhooks.ip_allowlist]` now so the later GitHub webhook feature only consumes existing config rather than renaming it |

## Sources & References

- **Origin document:** [docs/brainstorms/2026-04-15-ip-whitelist-requirements.md](docs/brainstorms/2026-04-15-ip-whitelist-requirements.md)
- Related code: `lib/crates/fabro-server/src/server.rs` (router, middleware)
- Related code: `lib/crates/fabro-server/src/serve.rs` (serve paths)
- Related code: `lib/crates/fabro-server/src/tls.rs` (TLS accept loop)
- Related code: `lib/crates/fabro-types/src/settings/server.rs` (config types)
- Related code: `lib/crates/fabro-config/src/resolve/server.rs` (config resolution)
- External: [ipnet crate](https://docs.rs/ipnet/latest/ipnet/) for CIDR matching
- External: [Axum ConnectInfo](https://docs.rs/axum/0.8/axum/extract/struct.ConnectInfo.html)
- External: [GitHub REST API meta endpoint](https://docs.github.com/en/rest/meta/meta) for `GET /meta` and the `hooks` IP list
- External: [GitHub webhook best practices](https://docs.github.com/en/enterprise-cloud@latest/webhooks/using-webhooks/best-practices-for-using-webhooks) for the recommendation to allow GitHub's current webhook IPs
