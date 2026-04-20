# Portkey AI Gateway Integration

**Status:** Draft for review
**Date:** 2026-04-20
**Author:** Haroldo Olivieri (with Claude)

## Problem

Fabro's LLM client connects directly to provider APIs (Anthropic, OpenAI, Gemini, etc.). Organizations that use Portkey as an AI gateway — for observability, cost tracking, fallback routing, or to access models through AWS Bedrock or Azure — cannot route Fabro's requests through Portkey without forking the LLM client.

## Goals

- Any Fabro user can route all LLM traffic through Portkey by setting environment variables.
- Zero impact on users who do not set Portkey variables — existing behavior is unchanged.
- Support the three common Portkey routing modes: provider slugs, configs, and direct AWS Bedrock credentials.
- Clear documentation with distinct setup paths per routing mode.
- Comprehensive test coverage for all configuration scenarios.

## Non-goals (v1)

- Per-provider opt-out (e.g., Anthropic through Portkey but OpenAI direct). Use `PORTKEY_CONFIG` for selective routing.
- Per-request provider switching or virtual key rotation.
- New `Provider::Portkey` enum variant — Portkey is a transparent gateway, not a model provider.
- Model ID translation — users supply the correct model ID for their target provider (e.g., Bedrock inference profile IDs).
- `x-portkey-virtual-key` support — deprecated by Portkey in favor of provider slugs and configs.

## Architecture overview

Portkey integration is a **credential transformation layer** in `fabro-llm`. It does not add a new provider adapter. Instead, it modifies `ApiCredential` values before they reach `from_credentials()`, overriding `base_url` and injecting `x-portkey-*` headers. The model resolution flow is unchanged:

```
Stylesheet → ModelHandle → Catalog → Provider::Anthropic → AnthropicAdapter
                                                              ↑
                                              base_url = PORTKEY_URL
                                              x-portkey-* headers injected
                                              auth key = dummy (Portkey handles auth)
```

When Portkey is configured but no provider API key is set (e.g., no `ANTHROPIC_API_KEY`), the integration creates a credential with a dummy auth key for the provider specified by `PORTKEY_PROVIDER`. This matches how Portkey works in practice: the gateway handles upstream authentication, so the original provider key is not needed.

## Environment variables

Three variables are **required** when using Portkey. All others are optional.

| Variable | Required | Description |
|----------|----------|-------------|
| `PORTKEY_URL` | Yes | Gateway base URL (e.g., `https://api.portkey.ai/v1`) |
| `PORTKEY_API_KEY` | Yes | Portkey API key — sent as `x-portkey-api-key` header |
| `PORTKEY_PROVIDER` | Yes | Provider enum value (`anthropic`, `openai`, `gemini`, `kimi`, `zai`, `minimax`, `inception`) — determines which adapter and request format to use |
| `PORTKEY_PROVIDER_SLUG` | No | Value for `x-portkey-provider` header (e.g., `@bedrock-sandbox`, `@azure-prod`). Defaults to `PORTKEY_PROVIDER` when omitted. Only needed when the Portkey routing target differs from the adapter type (Bedrock, Azure). |
| `PORTKEY_CONFIG` | No | Portkey config ID or inline JSON — enables fallbacks, load balancing, conditional routing |
| `PORTKEY_AWS_ACCESS_KEY_ID` | No | AWS access key for direct Bedrock access without a Portkey Model Catalog provider |
| `PORTKEY_AWS_SECRET_ACCESS_KEY` | No | AWS secret key |
| `PORTKEY_AWS_REGION` | No | AWS region (e.g., `eu-west-1`) |
| `PORTKEY_AWS_SESSION_TOKEN` | No | AWS STS session token for federated/assumed-role access |
| `PORTKEY_METADATA` | No | JSON string attached to requests for observability (e.g., `{"team":"eng"}`) |

`PortkeyConfig::from_env()` returns `None` if any of the three required variables is absent. Users without Portkey see zero behavior change.

## Routing scenarios

### Scenario A: Direct provider through Portkey (simplest)

Route requests to a provider's own API through Portkey for observability and cost tracking.

```env
PORTKEY_URL=https://api.portkey.ai/v1
PORTKEY_API_KEY=pk-xxx
PORTKEY_PROVIDER=anthropic
```

- Adapter: Anthropic (Messages API format)
- `x-portkey-provider` header: `anthropic`
- Model IDs: standard Anthropic names (`claude-sonnet-4-6`, `claude-opus-4-6`)
- Provider API key: optional — if `ANTHROPIC_API_KEY` is set it flows as the auth header; if absent, a dummy key is used and Portkey handles auth via its Model Catalog

### Scenario B: AWS Bedrock through Portkey Model Catalog

Route requests to Bedrock using a pre-configured provider in Portkey's Model Catalog. The Model Catalog provider stores AWS credentials, so no local AWS keys are needed.

```env
PORTKEY_URL=https://api.portkey.ai/v1
PORTKEY_API_KEY=pk-xxx
PORTKEY_PROVIDER=anthropic
PORTKEY_PROVIDER_SLUG=@bedrock-sandbox
```

- Adapter: Anthropic (Bedrock Claude uses the same Messages API format)
- `x-portkey-provider` header: `@bedrock-sandbox`
- Model IDs: **Bedrock inference profile IDs** — use the region-prefixed format in your workflow stylesheet:
  - `eu.anthropic.claude-sonnet-4-6` (EU)
  - `us.anthropic.claude-sonnet-4-6` (US)
  - `global.anthropic.claude-sonnet-4-6` (Global)
  - Full list: [AWS Bedrock inference profiles](https://docs.aws.amazon.com/bedrock/latest/userguide/inference-profiles-support.html)

Stylesheet example:
```css
* { model: eu.anthropic.claude-sonnet-4-6; }
.code { model: eu.anthropic.claude-opus-4-6-v1; }
```

### Scenario C: AWS Bedrock with direct AWS credentials

Route to Bedrock by passing AWS credentials through Portkey headers, without configuring a Model Catalog provider.

```env
PORTKEY_URL=https://api.portkey.ai/v1
PORTKEY_API_KEY=pk-xxx
PORTKEY_PROVIDER=anthropic
PORTKEY_PROVIDER_SLUG=bedrock
PORTKEY_AWS_ACCESS_KEY_ID=AKIA...
PORTKEY_AWS_SECRET_ACCESS_KEY=xxx
PORTKEY_AWS_REGION=eu-west-1
```

- Same adapter and model ID format as Scenario B
- AWS credentials flow as `x-portkey-aws-*` headers
- Optional `PORTKEY_AWS_SESSION_TOKEN` for STS/federated access

### Scenario D: Config-based routing (fallbacks, load balancing)

Use a Portkey config for advanced routing strategies across multiple providers.

```env
PORTKEY_URL=https://api.portkey.ai/v1
PORTKEY_API_KEY=pk-xxx
PORTKEY_PROVIDER=anthropic
PORTKEY_CONFIG=cfg-xxx
```

- The config defines fallback chains, load balancing weights, or conditional routing rules
- `PORTKEY_PROVIDER` still determines the adapter/request format
- `PORTKEY_PROVIDER_SLUG` is typically not needed — the config handles provider selection
- Configs are created in the Portkey dashboard and referenced by ID, or passed as inline JSON

### Scenario E: OpenAI or Gemini through Portkey

Works identically to Scenario A with a different provider value.

```env
# OpenAI through Portkey
PORTKEY_URL=https://api.portkey.ai/v1
PORTKEY_API_KEY=pk-xxx
PORTKEY_PROVIDER=openai
```

```env
# Gemini through Portkey
PORTKEY_URL=https://api.portkey.ai/v1
PORTKEY_API_KEY=pk-xxx
PORTKEY_PROVIDER=gemini
```

- `x-portkey-provider` header matches the provider name (`openai`, `gemini`)
- Standard model IDs for each provider
- For Azure OpenAI: set `PORTKEY_PROVIDER=openai`, `PORTKEY_PROVIDER_SLUG=@azure-prod`

## File changes

### New: `lib/crates/fabro-llm/src/portkey.rs`

Top-level module in `fabro-llm` (not in `providers/`) because it transforms credentials, not adapter behavior.

```rust
use std::collections::HashMap;

use fabro_auth::ApiCredential;
use fabro_model::Provider;

/// Configuration for routing LLM requests through the Portkey AI gateway.
///
/// When enabled, overrides provider base URLs and injects Portkey
/// authentication headers into all outgoing LLM requests. Portkey
/// handles upstream provider authentication and routing.
///
/// Set `PORTKEY_URL`, `PORTKEY_API_KEY`, and `PORTKEY_PROVIDER` to enable.
pub struct PortkeyConfig {
    /// Portkey gateway base URL.
    pub base_url:       String,
    /// Portkey API key — sent as `x-portkey-api-key`.
    pub api_key:        String,
    /// Target provider — determines which adapter (request format) to use.
    pub provider:       Provider,
    /// Portkey provider slug for `x-portkey-provider` header.
    /// Defaults to `provider.as_str()` when absent.
    pub provider_slug:  Option<String>,
    /// Portkey config ID or inline JSON for `x-portkey-config`.
    pub config:         Option<String>,
    /// JSON metadata for `x-portkey-metadata`.
    pub metadata:       Option<String>,
    /// AWS credentials for direct Bedrock access via Portkey.
    pub aws:            Option<AwsCredentials>,
}

pub struct AwsCredentials {
    pub access_key_id:     String,
    pub secret_access_key: String,
    pub region:            String,
    pub session_token:     Option<String>,
}
```

**`from_env() -> Option<Self>`**

Returns `None` if any of `PORTKEY_URL`, `PORTKEY_API_KEY`, or `PORTKEY_PROVIDER` is absent. `PORTKEY_PROVIDER` is parsed via `Provider::from_str()` — returns `None` with a `tracing::warn!` if the value is not a recognized provider name.

AWS credentials are collected into `Some(AwsCredentials)` only if both `PORTKEY_AWS_ACCESS_KEY_ID` and `PORTKEY_AWS_SECRET_ACCESS_KEY` are present.

**`apply(&self, credentials: &mut Vec<ApiCredential>)`**

1. **Find or create the target credential.** Search `credentials` for one matching `self.provider`. If not found, create a new `ApiCredential` with:
   - `provider`: `self.provider`
   - `auth_header`: `ApiKeyHeader::Custom { name: "x-api-key", value: "pk-dummy" }` for Anthropic, `ApiKeyHeader::Bearer("pk-dummy")` for others
   - `base_url`: `None`
   - `extra_headers`: empty
   - `codex_mode`: `false`
   - `org_id`, `project_id`: `None`

2. **Override base URL.** Set `credential.base_url = Some(self.base_url.clone())`.

3. **Inject headers into `extra_headers`** (INSERT, not replace — preserves existing entries):
   - `x-portkey-api-key`: always
   - `x-portkey-provider`: `self.provider_slug` if set, else `self.provider.as_str()`
   - `x-portkey-config`: if `self.config` is set
   - `x-portkey-metadata`: if `self.metadata` is set
   - `x-portkey-aws-access-key-id`, `x-portkey-aws-secret-access-key`, `x-portkey-aws-region`, `x-portkey-aws-session-token`: if `self.aws` is set

### Modified: `lib/crates/fabro-llm/src/lib.rs`

Add `pub mod portkey;`.

### Modified: `lib/crates/fabro-llm/src/client.rs`

Two lines added in `from_env()`, after building the `credentials` vec, before `Self::from_credentials(credentials).await`:

```rust
if let Some(portkey) = crate::portkey::PortkeyConfig::from_env() {
    portkey.apply(&mut credentials);
}
```

No changes to `from_credentials()`. The existing credential → adapter pipeline handles everything because:
- Native adapters (Anthropic, OpenAI, Gemini) read `credential.base_url` via `with_base_url()` and `credential.extra_headers` via `with_default_headers()`
- OpenAiCompatible adapters (Kimi, Zai, Minimax, Inception) read `credential.base_url` as a constructor arg via `unwrap_or_else()` and `credential.extra_headers` via `with_default_headers()`

### Modified: `.env.example`

```env
# Portkey AI gateway (optional — routes LLM traffic through Portkey when set)
# Docs: https://fabro.sh/docs/integrations/portkey
PORTKEY_URL=
PORTKEY_API_KEY=
PORTKEY_PROVIDER=

# Optional Portkey settings:
PORTKEY_PROVIDER_SLUG=
PORTKEY_CONFIG=
PORTKEY_METADATA=

# Direct AWS Bedrock credentials (alternative to Portkey Model Catalog):
PORTKEY_AWS_ACCESS_KEY_ID=
PORTKEY_AWS_SECRET_ACCESS_KEY=
PORTKEY_AWS_REGION=
PORTKEY_AWS_SESSION_TOKEN=
```

### New: `docs/integrations/portkey.mdx`

Mintlify-format integration doc following the existing pattern (`brave-search.mdx`, `daytona.mdx`). Structure:

1. **Introduction** — what Portkey is, why you'd use it with Fabro
2. **Prerequisites** — Portkey account, at least one provider configured
3. **Setup paths** — five scenarios (A through E) as described above, each with:
   - When to use this path
   - Environment variables to set
   - Stylesheet model ID format
   - Verify-it-works step
4. **Model IDs for Bedrock** — explanation that Bedrock requires inference profile IDs, table of common Claude models with their EU/US/Global IDs, link to AWS reference page
5. **Environment variable reference** — full table with required/optional, descriptions
6. **Known limitations** — all-or-nothing routing, no per-request switching
7. **Troubleshooting** — common errors: missing required vars, wrong provider slug, Bedrock IAM permissions, model ID format mismatch

### Modified: `docs/docs.json`

Add `"integrations/portkey"` to the Integrations group pages array, after `"integrations/slack"`.

## Test plan

### Unit tests in `portkey.rs`

| Test | What it validates |
|------|-------------------|
| `from_env_returns_none_when_url_missing` | `PORTKEY_API_KEY` + `PORTKEY_PROVIDER` set but no `PORTKEY_URL` → `None` |
| `from_env_returns_none_when_api_key_missing` | `PORTKEY_URL` + `PORTKEY_PROVIDER` set but no `PORTKEY_API_KEY` → `None` |
| `from_env_returns_none_when_provider_missing` | `PORTKEY_URL` + `PORTKEY_API_KEY` set but no `PORTKEY_PROVIDER` → `None` |
| `from_env_returns_none_on_invalid_provider` | `PORTKEY_PROVIDER=invalid` → `None` with warning |
| `from_env_parses_all_required_fields` | All three required vars set → `Some` with correct values |
| `from_env_parses_optional_fields` | All vars set including slug, config, metadata, AWS → correct struct |
| `from_env_collects_aws_only_when_both_keys_present` | Only `ACCESS_KEY_ID` without `SECRET` → `aws` is `None` |
| `apply_creates_credential_when_none_exists` | Empty credentials vec → one credential created for target provider with dummy key |
| `apply_modifies_existing_credential` | Existing Anthropic credential → `base_url` and `extra_headers` updated |
| `apply_preserves_existing_extra_headers` | Credential with codex-mode headers → Portkey headers added alongside, originals preserved |
| `apply_sets_provider_slug_from_env` | `PORTKEY_PROVIDER_SLUG=@bedrock-sandbox` → `x-portkey-provider: @bedrock-sandbox` |
| `apply_defaults_provider_slug_to_provider_name` | No `PORTKEY_PROVIDER_SLUG` → `x-portkey-provider: anthropic` |
| `apply_injects_config_header` | `PORTKEY_CONFIG=cfg-xxx` → `x-portkey-config: cfg-xxx` |
| `apply_injects_metadata_header` | `PORTKEY_METADATA={"team":"eng"}` → `x-portkey-metadata: {"team":"eng"}` |
| `apply_injects_aws_headers` | All AWS vars set → four `x-portkey-aws-*` headers present |
| `apply_skips_aws_session_token_when_absent` | No session token → only three AWS headers |
| `apply_does_not_touch_other_credentials` | Anthropic + OpenAI credentials, Portkey targets Anthropic → OpenAI credential unchanged |

### Scenario integration tests in `portkey.rs`

| Test | Scenario | What it validates |
|------|----------|-------------------|
| `scenario_a_direct_provider` | A | Provider slug defaults to `anthropic`, base_url overridden, dummy key created |
| `scenario_b_bedrock_model_catalog` | B | Custom slug `@bedrock-sandbox` in header, Anthropic adapter, dummy key |
| `scenario_c_bedrock_direct_aws` | C | Slug `bedrock`, AWS headers injected, Anthropic adapter |
| `scenario_d_config_routing` | D | Config header set, provider slug still present, Anthropic adapter |
| `scenario_e_openai_through_portkey` | E (OpenAI) | OpenAI adapter created with dummy bearer key, slug `openai` |
| `scenario_e_gemini_through_portkey` | E (Gemini) | Gemini adapter created with dummy bearer key, slug `gemini` |
| `scenario_existing_api_key_preserved` | A with key | `ANTHROPIC_API_KEY` set → real key used as auth, Portkey headers still injected |

### E2E tests in `lib/crates/fabro-llm/tests/integration.rs`

| Test | What it validates |
|------|-------------------|
| `portkey_anthropic_complete` | `#[e2e_test(live("PORTKEY_API_KEY"))]` — full request/response cycle through Portkey to Anthropic |
| `portkey_bedrock_complete` | `#[e2e_test(live("PORTKEY_API_KEY"))]` — full request/response cycle through Portkey to Bedrock with custom slug |

These require `PORTKEY_URL`, `PORTKEY_API_KEY`, `PORTKEY_PROVIDER`, and optionally `PORTKEY_PROVIDER_SLUG` to be set in `.env`. They are live-only tests (no twin mode, since we don't have a Portkey mock).

## What this does NOT change

- No new `Provider` enum variants in `fabro-model`
- No model catalog changes
- No changes to `from_credentials()` or any provider adapter
- No model ID translation logic — users provide the right ID for their target
- No per-request provider switching (v1)
- No `x-portkey-virtual-key` support (deprecated by Portkey)
