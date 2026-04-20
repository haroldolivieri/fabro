# Portkey AI Gateway Integration — Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Enable routing all Fabro LLM traffic through Portkey's AI gateway by setting environment variables.

**Architecture:** A new `portkey.rs` module in `fabro-llm` transforms `ApiCredential` values before they reach the existing adapter pipeline. It overrides `base_url` and injects `x-portkey-*` headers. When no provider API key is set, it creates a credential with a dummy key. No new provider enum, no adapter changes.

**Tech Stack:** Rust, `tracing` for warnings, `fabro-auth::ApiCredential`, `fabro-model::Provider`

**Spec:** `docs/superpowers/specs/2026-04-20-portkey-integration-design.md`

---

## File Map

| Action | File | Responsibility |
|--------|------|----------------|
| Create | `lib/crates/fabro-llm/src/portkey.rs` | `PortkeyConfig` struct, `from_env()`, `apply()`, unit tests |
| Modify | `lib/crates/fabro-llm/src/lib.rs` | Add `pub mod portkey;` |
| Modify | `lib/crates/fabro-llm/src/client.rs:48-141` | Call `PortkeyConfig::from_env()` + `apply()` in `from_env()` |
| Modify | `.env.example` | Add Portkey env vars |
| Create | `docs/integrations/portkey.mdx` | User-facing integration guide |
| Modify | `docs/docs.json` | Add `"integrations/portkey"` to nav |
| Modify | `lib/crates/fabro-llm/tests/integration.rs` | E2E live tests for Portkey |

---

### Task 1: `PortkeyConfig` struct and `from_env()`

**Files:**
- Create: `lib/crates/fabro-llm/src/portkey.rs`

- [ ] **Step 1: Write the failing test — `from_env` returns `None` when required vars are missing**

Add to `lib/crates/fabro-llm/src/portkey.rs`:

```rust
#[cfg(test)]
mod tests {
    use super::*;

    // Helper: clear all PORTKEY_* env vars to isolate tests.
    fn clear_portkey_env() {
        for var in [
            "PORTKEY_URL", "PORTKEY_API_KEY", "PORTKEY_PROVIDER",
            "PORTKEY_PROVIDER_SLUG", "PORTKEY_CONFIG", "PORTKEY_METADATA",
            "PORTKEY_AWS_ACCESS_KEY_ID", "PORTKEY_AWS_SECRET_ACCESS_KEY",
            "PORTKEY_AWS_REGION", "PORTKEY_AWS_SESSION_TOKEN",
        ] {
            std::env::remove_var(var);
        }
    }

    #[test]
    fn from_env_returns_none_when_url_missing() {
        clear_portkey_env();
        std::env::set_var("PORTKEY_API_KEY", "pk-test");
        std::env::set_var("PORTKEY_PROVIDER", "anthropic");
        assert!(PortkeyConfig::from_env().is_none());
    }

    #[test]
    fn from_env_returns_none_when_api_key_missing() {
        clear_portkey_env();
        std::env::set_var("PORTKEY_URL", "https://api.portkey.ai/v1");
        std::env::set_var("PORTKEY_PROVIDER", "anthropic");
        assert!(PortkeyConfig::from_env().is_none());
    }

    #[test]
    fn from_env_returns_none_when_provider_missing() {
        clear_portkey_env();
        std::env::set_var("PORTKEY_URL", "https://api.portkey.ai/v1");
        std::env::set_var("PORTKEY_API_KEY", "pk-test");
        assert!(PortkeyConfig::from_env().is_none());
    }

    #[test]
    fn from_env_returns_none_on_invalid_provider() {
        clear_portkey_env();
        std::env::set_var("PORTKEY_URL", "https://api.portkey.ai/v1");
        std::env::set_var("PORTKEY_API_KEY", "pk-test");
        std::env::set_var("PORTKEY_PROVIDER", "invalid_provider");
        assert!(PortkeyConfig::from_env().is_none());
    }
}
```

- [ ] **Step 2: Run test to verify it fails**

Run: `cargo nextest run -p fabro-llm -- portkey`
Expected: Compilation error — `PortkeyConfig` not defined yet.

- [ ] **Step 3: Write `PortkeyConfig` struct and `from_env()`**

Add to top of `lib/crates/fabro-llm/src/portkey.rs`:

```rust
use std::collections::HashMap;
use std::str::FromStr;

use fabro_model::Provider;
use tracing::warn;

/// AWS credentials for direct Bedrock access via Portkey headers.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AwsCredentials {
    pub access_key_id:     String,
    pub secret_access_key: String,
    pub region:            String,
    pub session_token:     Option<String>,
}

/// Configuration for routing LLM requests through the Portkey AI gateway.
///
/// When enabled, overrides provider base URLs and injects Portkey
/// authentication headers into all outgoing LLM requests.
///
/// Requires `PORTKEY_URL`, `PORTKEY_API_KEY`, and `PORTKEY_PROVIDER`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PortkeyConfig {
    /// Portkey gateway base URL (e.g., `https://api.portkey.ai/v1`).
    pub base_url:      String,
    /// Portkey API key — sent as `x-portkey-api-key`.
    pub api_key:       String,
    /// Target provider — determines which adapter (request format) to use.
    pub provider:      Provider,
    /// Portkey provider slug for `x-portkey-provider` header.
    /// Defaults to `provider.as_str()` when absent.
    pub provider_slug: Option<String>,
    /// Portkey config ID or inline JSON for `x-portkey-config`.
    pub config:        Option<String>,
    /// JSON metadata for `x-portkey-metadata`.
    pub metadata:      Option<String>,
    /// AWS credentials for direct Bedrock access.
    pub aws:           Option<AwsCredentials>,
}

impl PortkeyConfig {
    /// Load Portkey configuration from environment variables.
    ///
    /// Returns `None` if any required variable (`PORTKEY_URL`,
    /// `PORTKEY_API_KEY`, `PORTKEY_PROVIDER`) is absent or if the
    /// provider value is not a recognized provider name.
    #[must_use]
    pub fn from_env() -> Option<Self> {
        let base_url = std::env::var("PORTKEY_URL").ok()?;
        let api_key = std::env::var("PORTKEY_API_KEY").ok()?;

        let provider_str = std::env::var("PORTKEY_PROVIDER").ok()?;
        let provider = match Provider::from_str(&provider_str) {
            Ok(p) => p,
            Err(_) => {
                warn!(
                    provider = %provider_str,
                    "PORTKEY_PROVIDER is not a recognized provider name, \
                     ignoring Portkey configuration"
                );
                return None;
            }
        };

        let provider_slug = std::env::var("PORTKEY_PROVIDER_SLUG").ok();
        let config = std::env::var("PORTKEY_CONFIG").ok();
        let metadata = std::env::var("PORTKEY_METADATA").ok();

        let aws = match (
            std::env::var("PORTKEY_AWS_ACCESS_KEY_ID").ok(),
            std::env::var("PORTKEY_AWS_SECRET_ACCESS_KEY").ok(),
        ) {
            (Some(access_key_id), Some(secret_access_key)) => {
                let region = std::env::var("PORTKEY_AWS_REGION")
                    .unwrap_or_else(|_| "us-east-1".to_string());
                let session_token = std::env::var("PORTKEY_AWS_SESSION_TOKEN").ok();
                Some(AwsCredentials {
                    access_key_id,
                    secret_access_key,
                    region,
                    session_token,
                })
            }
            _ => None,
        };

        Some(Self {
            base_url,
            api_key,
            provider,
            provider_slug,
            config,
            metadata,
            aws,
        })
    }
}
```

- [ ] **Step 4: Run tests to verify they pass**

Run: `cargo nextest run -p fabro-llm -- portkey`
Expected: All 4 tests PASS.

- [ ] **Step 5: Write test — `from_env` parses all fields**

Add to `mod tests`:

```rust
    #[test]
    fn from_env_parses_all_required_fields() {
        clear_portkey_env();
        std::env::set_var("PORTKEY_URL", "https://api.portkey.ai/v1");
        std::env::set_var("PORTKEY_API_KEY", "pk-test-key");
        std::env::set_var("PORTKEY_PROVIDER", "anthropic");

        let config = PortkeyConfig::from_env().unwrap();
        assert_eq!(config.base_url, "https://api.portkey.ai/v1");
        assert_eq!(config.api_key, "pk-test-key");
        assert_eq!(config.provider, Provider::Anthropic);
        assert!(config.provider_slug.is_none());
        assert!(config.config.is_none());
        assert!(config.metadata.is_none());
        assert!(config.aws.is_none());
    }

    #[test]
    fn from_env_parses_optional_fields() {
        clear_portkey_env();
        std::env::set_var("PORTKEY_URL", "https://api.portkey.ai/v1");
        std::env::set_var("PORTKEY_API_KEY", "pk-test-key");
        std::env::set_var("PORTKEY_PROVIDER", "anthropic");
        std::env::set_var("PORTKEY_PROVIDER_SLUG", "@bedrock-sandbox");
        std::env::set_var("PORTKEY_CONFIG", "cfg-xxx");
        std::env::set_var("PORTKEY_METADATA", r#"{"team":"eng"}"#);
        std::env::set_var("PORTKEY_AWS_ACCESS_KEY_ID", "AKIAIOSFODNN7EXAMPLE");
        std::env::set_var("PORTKEY_AWS_SECRET_ACCESS_KEY", "wJalrXUtnFEMI/K7MDENG");
        std::env::set_var("PORTKEY_AWS_REGION", "eu-west-1");
        std::env::set_var("PORTKEY_AWS_SESSION_TOKEN", "FwoGZXIvY...");

        let config = PortkeyConfig::from_env().unwrap();
        assert_eq!(config.provider_slug.as_deref(), Some("@bedrock-sandbox"));
        assert_eq!(config.config.as_deref(), Some("cfg-xxx"));
        assert_eq!(config.metadata.as_deref(), Some(r#"{"team":"eng"}"#));

        let aws = config.aws.unwrap();
        assert_eq!(aws.access_key_id, "AKIAIOSFODNN7EXAMPLE");
        assert_eq!(aws.secret_access_key, "wJalrXUtnFEMI/K7MDENG");
        assert_eq!(aws.region, "eu-west-1");
        assert_eq!(aws.session_token.as_deref(), Some("FwoGZXIvY..."));
    }

    #[test]
    fn from_env_collects_aws_only_when_both_keys_present() {
        clear_portkey_env();
        std::env::set_var("PORTKEY_URL", "https://api.portkey.ai/v1");
        std::env::set_var("PORTKEY_API_KEY", "pk-test");
        std::env::set_var("PORTKEY_PROVIDER", "anthropic");
        std::env::set_var("PORTKEY_AWS_ACCESS_KEY_ID", "AKIA...");
        // No SECRET — should not collect AWS
        let config = PortkeyConfig::from_env().unwrap();
        assert!(config.aws.is_none());
    }
```

- [ ] **Step 6: Run tests**

Run: `cargo nextest run -p fabro-llm -- portkey`
Expected: All 7 tests PASS.

- [ ] **Step 7: Commit**

```bash
git add lib/crates/fabro-llm/src/portkey.rs
git commit -m "feat(fabro-llm): add PortkeyConfig struct and from_env()"
```

---

### Task 2: `PortkeyConfig::apply()` — header injection

**Files:**
- Modify: `lib/crates/fabro-llm/src/portkey.rs`

- [ ] **Step 1: Write the failing test — `apply` creates credential when none exists**

Add to `mod tests` in `portkey.rs`:

```rust
    use fabro_auth::{ApiCredential, ApiKeyHeader};

    fn empty_credential(provider: Provider) -> ApiCredential {
        ApiCredential {
            provider,
            auth_header:   ApiKeyHeader::Bearer("real-key".to_string()),
            extra_headers: HashMap::new(),
            base_url:      None,
            codex_mode:    false,
            org_id:        None,
            project_id:    None,
        }
    }

    fn portkey_config_anthropic() -> PortkeyConfig {
        PortkeyConfig {
            base_url:      "https://api.portkey.ai/v1".to_string(),
            api_key:       "pk-test".to_string(),
            provider:      Provider::Anthropic,
            provider_slug: None,
            config:        None,
            metadata:      None,
            aws:           None,
        }
    }

    #[test]
    fn apply_creates_credential_when_none_exists() {
        let config = portkey_config_anthropic();
        let mut credentials: Vec<ApiCredential> = Vec::new();

        config.apply(&mut credentials);

        assert_eq!(credentials.len(), 1);
        assert_eq!(credentials[0].provider, Provider::Anthropic);
        assert_eq!(
            credentials[0].base_url.as_deref(),
            Some("https://api.portkey.ai/v1")
        );
        assert_eq!(
            credentials[0].extra_headers.get("x-portkey-api-key"),
            Some(&"pk-test".to_string())
        );
        assert_eq!(
            credentials[0].extra_headers.get("x-portkey-provider"),
            Some(&"anthropic".to_string())
        );
    }
```

- [ ] **Step 2: Run test to verify it fails**

Run: `cargo nextest run -p fabro-llm -- apply_creates`
Expected: Compilation error — `apply` not defined yet.

- [ ] **Step 3: Write `apply()` and `build_headers()`**

Add to `impl PortkeyConfig` in `portkey.rs`:

```rust
    /// Build the Portkey headers to inject into credentials.
    fn build_headers(&self) -> HashMap<String, String> {
        let mut headers = HashMap::new();

        headers.insert(
            "x-portkey-api-key".to_string(),
            self.api_key.clone(),
        );

        let slug = self
            .provider_slug
            .as_deref()
            .unwrap_or_else(|| self.provider.as_str());
        headers.insert("x-portkey-provider".to_string(), slug.to_string());

        if let Some(config) = &self.config {
            headers.insert("x-portkey-config".to_string(), config.clone());
        }
        if let Some(metadata) = &self.metadata {
            headers.insert("x-portkey-metadata".to_string(), metadata.clone());
        }
        if let Some(aws) = &self.aws {
            headers.insert(
                "x-portkey-aws-access-key-id".to_string(),
                aws.access_key_id.clone(),
            );
            headers.insert(
                "x-portkey-aws-secret-access-key".to_string(),
                aws.secret_access_key.clone(),
            );
            headers.insert(
                "x-portkey-aws-region".to_string(),
                aws.region.clone(),
            );
            if let Some(token) = &aws.session_token {
                headers.insert(
                    "x-portkey-aws-session-token".to_string(),
                    token.clone(),
                );
            }
        }

        headers
    }

    /// Create a dummy auth header appropriate for the provider.
    fn dummy_auth_header(provider: Provider) -> ApiKeyHeader {
        match provider {
            Provider::Anthropic => ApiKeyHeader::Custom {
                name:  "x-api-key".to_string(),
                value: "pk-portkey-dummy".to_string(),
            },
            _ => ApiKeyHeader::Bearer("pk-portkey-dummy".to_string()),
        }
    }

    /// Apply Portkey configuration to a set of credentials.
    ///
    /// For the target provider: overrides `base_url` and injects Portkey
    /// headers into `extra_headers`. If no credential exists for the
    /// target provider, creates one with a dummy auth key (Portkey
    /// handles upstream authentication).
    pub fn apply(&self, credentials: &mut Vec<ApiCredential>) {
        let headers = self.build_headers();

        let existing = credentials
            .iter_mut()
            .find(|c| c.provider == self.provider);

        match existing {
            Some(credential) => {
                credential.base_url = Some(self.base_url.clone());
                for (key, value) in &headers {
                    credential.extra_headers.insert(key.clone(), value.clone());
                }
            }
            None => {
                credentials.push(ApiCredential {
                    provider:      self.provider,
                    auth_header:   Self::dummy_auth_header(self.provider),
                    extra_headers: headers,
                    base_url:      Some(self.base_url.clone()),
                    codex_mode:    false,
                    org_id:        None,
                    project_id:    None,
                });
            }
        }
    }
```

- [ ] **Step 4: Run test to verify it passes**

Run: `cargo nextest run -p fabro-llm -- apply_creates`
Expected: PASS.

- [ ] **Step 5: Write remaining `apply` tests**

Add to `mod tests`:

```rust
    #[test]
    fn apply_modifies_existing_credential() {
        let config = portkey_config_anthropic();
        let mut credentials = vec![empty_credential(Provider::Anthropic)];

        config.apply(&mut credentials);

        assert_eq!(credentials.len(), 1);
        assert_eq!(
            credentials[0].base_url.as_deref(),
            Some("https://api.portkey.ai/v1")
        );
        assert_eq!(
            credentials[0].extra_headers.get("x-portkey-api-key"),
            Some(&"pk-test".to_string())
        );
        // Original auth header is preserved.
        assert_eq!(
            credentials[0].auth_header,
            ApiKeyHeader::Bearer("real-key".to_string())
        );
    }

    #[test]
    fn apply_preserves_existing_extra_headers() {
        let config = portkey_config_anthropic();
        let mut credential = empty_credential(Provider::Anthropic);
        credential
            .extra_headers
            .insert("ChatGPT-Account-Id".to_string(), "acct-123".to_string());
        credential
            .extra_headers
            .insert("originator".to_string(), "fabro".to_string());
        let mut credentials = vec![credential];

        config.apply(&mut credentials);

        // Portkey headers added.
        assert!(credentials[0].extra_headers.contains_key("x-portkey-api-key"));
        // Original headers preserved.
        assert_eq!(
            credentials[0].extra_headers.get("ChatGPT-Account-Id"),
            Some(&"acct-123".to_string())
        );
        assert_eq!(
            credentials[0].extra_headers.get("originator"),
            Some(&"fabro".to_string())
        );
    }

    #[test]
    fn apply_sets_provider_slug_from_config() {
        let config = PortkeyConfig {
            provider_slug: Some("@bedrock-sandbox".to_string()),
            ..portkey_config_anthropic()
        };
        let mut credentials: Vec<ApiCredential> = Vec::new();

        config.apply(&mut credentials);

        assert_eq!(
            credentials[0].extra_headers.get("x-portkey-provider"),
            Some(&"@bedrock-sandbox".to_string())
        );
    }

    #[test]
    fn apply_defaults_provider_slug_to_provider_name() {
        let config = portkey_config_anthropic();
        let mut credentials: Vec<ApiCredential> = Vec::new();

        config.apply(&mut credentials);

        assert_eq!(
            credentials[0].extra_headers.get("x-portkey-provider"),
            Some(&"anthropic".to_string())
        );
    }

    #[test]
    fn apply_injects_config_header() {
        let config = PortkeyConfig {
            config: Some("cfg-xxx".to_string()),
            ..portkey_config_anthropic()
        };
        let mut credentials: Vec<ApiCredential> = Vec::new();

        config.apply(&mut credentials);

        assert_eq!(
            credentials[0].extra_headers.get("x-portkey-config"),
            Some(&"cfg-xxx".to_string())
        );
    }

    #[test]
    fn apply_injects_metadata_header() {
        let config = PortkeyConfig {
            metadata: Some(r#"{"team":"eng"}"#.to_string()),
            ..portkey_config_anthropic()
        };
        let mut credentials: Vec<ApiCredential> = Vec::new();

        config.apply(&mut credentials);

        assert_eq!(
            credentials[0].extra_headers.get("x-portkey-metadata"),
            Some(&r#"{"team":"eng"}"#.to_string())
        );
    }

    #[test]
    fn apply_injects_aws_headers() {
        let config = PortkeyConfig {
            aws: Some(AwsCredentials {
                access_key_id:     "AKIA...".to_string(),
                secret_access_key: "secret".to_string(),
                region:            "eu-west-1".to_string(),
                session_token:     Some("token123".to_string()),
            }),
            ..portkey_config_anthropic()
        };
        let mut credentials: Vec<ApiCredential> = Vec::new();

        config.apply(&mut credentials);

        let h = &credentials[0].extra_headers;
        assert_eq!(h.get("x-portkey-aws-access-key-id"), Some(&"AKIA...".to_string()));
        assert_eq!(h.get("x-portkey-aws-secret-access-key"), Some(&"secret".to_string()));
        assert_eq!(h.get("x-portkey-aws-region"), Some(&"eu-west-1".to_string()));
        assert_eq!(h.get("x-portkey-aws-session-token"), Some(&"token123".to_string()));
    }

    #[test]
    fn apply_skips_aws_session_token_when_absent() {
        let config = PortkeyConfig {
            aws: Some(AwsCredentials {
                access_key_id:     "AKIA...".to_string(),
                secret_access_key: "secret".to_string(),
                region:            "eu-west-1".to_string(),
                session_token:     None,
            }),
            ..portkey_config_anthropic()
        };
        let mut credentials: Vec<ApiCredential> = Vec::new();

        config.apply(&mut credentials);

        let h = &credentials[0].extra_headers;
        assert!(h.contains_key("x-portkey-aws-access-key-id"));
        assert!(!h.contains_key("x-portkey-aws-session-token"));
    }

    #[test]
    fn apply_does_not_touch_other_credentials() {
        let config = portkey_config_anthropic();
        let mut credentials = vec![
            empty_credential(Provider::Anthropic),
            empty_credential(Provider::OpenAi),
        ];

        config.apply(&mut credentials);

        // Anthropic: modified.
        assert!(credentials[0].extra_headers.contains_key("x-portkey-api-key"));
        // OpenAI: untouched.
        assert!(credentials[1].extra_headers.is_empty());
        assert!(credentials[1].base_url.is_none());
    }
```

- [ ] **Step 6: Run all tests**

Run: `cargo nextest run -p fabro-llm -- portkey`
Expected: All 17 tests PASS.

- [ ] **Step 7: Commit**

```bash
git add lib/crates/fabro-llm/src/portkey.rs
git commit -m "feat(fabro-llm): add PortkeyConfig::apply() with full header injection"
```

---

### Task 3: Scenario integration tests

**Files:**
- Modify: `lib/crates/fabro-llm/src/portkey.rs`

These tests validate the full credential transformation for each documented scenario (A–E from the spec).

- [ ] **Step 1: Write scenario tests**

Add to `mod tests` in `portkey.rs`:

```rust
    // --- Scenario integration tests ---

    #[test]
    fn scenario_a_direct_provider() {
        // PORTKEY_PROVIDER=anthropic, no slug override.
        let config = portkey_config_anthropic();
        let mut credentials: Vec<ApiCredential> = Vec::new();

        config.apply(&mut credentials);

        assert_eq!(credentials.len(), 1);
        assert_eq!(credentials[0].provider, Provider::Anthropic);
        assert_eq!(credentials[0].base_url.as_deref(), Some("https://api.portkey.ai/v1"));
        assert_eq!(
            credentials[0].extra_headers.get("x-portkey-provider"),
            Some(&"anthropic".to_string())
        );
        // Dummy key created.
        assert_eq!(
            credentials[0].auth_header,
            ApiKeyHeader::Custom {
                name:  "x-api-key".to_string(),
                value: "pk-portkey-dummy".to_string(),
            }
        );
    }

    #[test]
    fn scenario_b_bedrock_model_catalog() {
        // PORTKEY_PROVIDER=anthropic, PORTKEY_PROVIDER_SLUG=@bedrock-sandbox.
        let config = PortkeyConfig {
            provider_slug: Some("@bedrock-sandbox".to_string()),
            ..portkey_config_anthropic()
        };
        let mut credentials: Vec<ApiCredential> = Vec::new();

        config.apply(&mut credentials);

        assert_eq!(credentials[0].provider, Provider::Anthropic);
        assert_eq!(
            credentials[0].extra_headers.get("x-portkey-provider"),
            Some(&"@bedrock-sandbox".to_string())
        );
        // No AWS headers — credentials managed by Portkey Model Catalog.
        assert!(!credentials[0].extra_headers.contains_key("x-portkey-aws-access-key-id"));
    }

    #[test]
    fn scenario_c_bedrock_direct_aws() {
        // PORTKEY_PROVIDER=anthropic, PORTKEY_PROVIDER_SLUG=bedrock, + AWS creds.
        let config = PortkeyConfig {
            provider_slug: Some("bedrock".to_string()),
            aws: Some(AwsCredentials {
                access_key_id:     "AKIA...".to_string(),
                secret_access_key: "secret".to_string(),
                region:            "eu-west-1".to_string(),
                session_token:     None,
            }),
            ..portkey_config_anthropic()
        };
        let mut credentials: Vec<ApiCredential> = Vec::new();

        config.apply(&mut credentials);

        assert_eq!(
            credentials[0].extra_headers.get("x-portkey-provider"),
            Some(&"bedrock".to_string())
        );
        assert_eq!(
            credentials[0].extra_headers.get("x-portkey-aws-access-key-id"),
            Some(&"AKIA...".to_string())
        );
    }

    #[test]
    fn scenario_d_config_routing() {
        // PORTKEY_PROVIDER=anthropic, PORTKEY_CONFIG=cfg-xxx.
        let config = PortkeyConfig {
            config: Some("cfg-xxx".to_string()),
            ..portkey_config_anthropic()
        };
        let mut credentials: Vec<ApiCredential> = Vec::new();

        config.apply(&mut credentials);

        assert_eq!(
            credentials[0].extra_headers.get("x-portkey-config"),
            Some(&"cfg-xxx".to_string())
        );
        // Provider slug still present for Portkey fallback.
        assert!(credentials[0].extra_headers.contains_key("x-portkey-provider"));
    }

    #[test]
    fn scenario_e_openai_through_portkey() {
        let config = PortkeyConfig {
            provider: Provider::OpenAi,
            ..portkey_config_anthropic()
        };
        let mut credentials: Vec<ApiCredential> = Vec::new();

        config.apply(&mut credentials);

        assert_eq!(credentials[0].provider, Provider::OpenAi);
        assert_eq!(
            credentials[0].auth_header,
            ApiKeyHeader::Bearer("pk-portkey-dummy".to_string())
        );
        assert_eq!(
            credentials[0].extra_headers.get("x-portkey-provider"),
            Some(&"openai".to_string())
        );
    }

    #[test]
    fn scenario_e_gemini_through_portkey() {
        let config = PortkeyConfig {
            provider: Provider::Gemini,
            ..portkey_config_anthropic()
        };
        let mut credentials: Vec<ApiCredential> = Vec::new();

        config.apply(&mut credentials);

        assert_eq!(credentials[0].provider, Provider::Gemini);
        assert_eq!(
            credentials[0].extra_headers.get("x-portkey-provider"),
            Some(&"gemini".to_string())
        );
    }

    #[test]
    fn scenario_existing_api_key_preserved() {
        // User has ANTHROPIC_API_KEY set AND Portkey configured.
        let config = portkey_config_anthropic();
        let mut credentials = vec![ApiCredential {
            provider:      Provider::Anthropic,
            auth_header:   ApiKeyHeader::Custom {
                name:  "x-api-key".to_string(),
                value: "sk-ant-real-key".to_string(),
            },
            extra_headers: HashMap::new(),
            base_url:      Some("https://api.anthropic.com/v1".to_string()),
            codex_mode:    false,
            org_id:        None,
            project_id:    None,
        }];

        config.apply(&mut credentials);

        // Real key preserved — Portkey forwards it.
        assert_eq!(
            credentials[0].auth_header,
            ApiKeyHeader::Custom {
                name:  "x-api-key".to_string(),
                value: "sk-ant-real-key".to_string(),
            }
        );
        // But base_url overridden to Portkey.
        assert_eq!(
            credentials[0].base_url.as_deref(),
            Some("https://api.portkey.ai/v1")
        );
        // And portkey headers injected.
        assert!(credentials[0].extra_headers.contains_key("x-portkey-api-key"));
    }
```

- [ ] **Step 2: Run tests**

Run: `cargo nextest run -p fabro-llm -- scenario`
Expected: All 7 scenario tests PASS.

- [ ] **Step 3: Commit**

```bash
git add lib/crates/fabro-llm/src/portkey.rs
git commit -m "test(fabro-llm): add scenario integration tests for Portkey routing modes"
```

---

### Task 4: Wire into `Client::from_env()`

**Files:**
- Modify: `lib/crates/fabro-llm/src/lib.rs:1`
- Modify: `lib/crates/fabro-llm/src/client.rs:140-141`

- [ ] **Step 1: Add module declaration**

In `lib/crates/fabro-llm/src/lib.rs`, add after `pub mod provider;` (line 6):

```rust
pub mod portkey;
```

- [ ] **Step 2: Add Portkey integration to `from_env()`**

In `lib/crates/fabro-llm/src/client.rs`, add the following two lines between line 140 (`}` closing the `INCEPTION_API_KEY` block) and line 141 (`Self::from_credentials(credentials).await`):

```rust
        if let Some(portkey) = crate::portkey::PortkeyConfig::from_env() {
            portkey.apply(&mut credentials);
        }
```

- [ ] **Step 3: Verify compilation**

Run: `cargo build -p fabro-llm`
Expected: Compiles with no errors and no warnings.

- [ ] **Step 4: Run full test suite**

Run: `cargo nextest run -p fabro-llm`
Expected: All existing tests pass, plus all 24 portkey tests pass.

- [ ] **Step 5: Commit**

```bash
git add lib/crates/fabro-llm/src/lib.rs lib/crates/fabro-llm/src/client.rs
git commit -m "feat(fabro-llm): wire PortkeyConfig into Client::from_env()"
```

---

### Task 5: Update `.env.example`

**Files:**
- Modify: `.env.example`

- [ ] **Step 1: Add Portkey env vars**

Append to the end of `.env.example` (after the `FABRO_DOMAIN=` line):

```env

# Portkey AI gateway (optional — routes LLM traffic through Portkey when set)
# Docs: https://fabro.sh/docs/integrations/portkey
PORTKEY_URL=
PORTKEY_API_KEY=
PORTKEY_PROVIDER=

# Optional Portkey settings:
# PORTKEY_PROVIDER_SLUG=         # Override x-portkey-provider header (e.g. @bedrock-sandbox)
# PORTKEY_CONFIG=                # Portkey config ID or inline JSON
# PORTKEY_METADATA=              # JSON metadata for observability

# Direct AWS Bedrock credentials (alternative to Portkey Model Catalog):
# PORTKEY_AWS_ACCESS_KEY_ID=
# PORTKEY_AWS_SECRET_ACCESS_KEY=
# PORTKEY_AWS_REGION=
# PORTKEY_AWS_SESSION_TOKEN=
```

- [ ] **Step 2: Commit**

```bash
git add .env.example
git commit -m "docs: add Portkey env vars to .env.example"
```

---

### Task 6: Integration documentation — `docs/integrations/portkey.mdx`

**Files:**
- Create: `docs/integrations/portkey.mdx`
- Modify: `docs/docs.json`

- [ ] **Step 1: Create the doc**

Create `docs/integrations/portkey.mdx`:

````mdx
---
title: "Portkey AI Gateway"
description: "Route Fabro's LLM traffic through Portkey for observability, cost tracking, and provider routing"
---

Fabro can route all LLM requests through [Portkey](https://portkey.ai), an AI gateway that provides observability, cost tracking, fallback routing, and access to providers like AWS Bedrock and Azure OpenAI. When enabled, Portkey sits between Fabro and your LLM provider — Fabro sends requests to Portkey's gateway, and Portkey forwards them to the configured upstream provider.

No code changes are needed. Set three environment variables and Fabro routes through Portkey automatically.

## Prerequisites

- A [Portkey](https://portkey.ai) account with an API key
- At least one provider configured in your Portkey dashboard (direct API key, Model Catalog provider, or a routing config)

## Setup paths

Choose the path that matches your Portkey configuration. Each path lists the environment variables to set and how to configure model IDs in your workflow stylesheets.

### Path A: Direct provider through Portkey

Route requests to a provider's own API through Portkey for observability and cost tracking. Simplest setup.

```bash title=".env"
PORTKEY_URL=https://api.portkey.ai/v1
PORTKEY_API_KEY=pk-xxx
PORTKEY_PROVIDER=anthropic
```

Use standard model IDs in your stylesheet:

```css
* { model: claude-sonnet-4-6; }
```

<Note>
If your provider API key is already set (e.g. `ANTHROPIC_API_KEY`), Fabro sends it as the auth header and Portkey forwards it. If the key is **not** set, Fabro creates a dummy key — Portkey handles authentication via your dashboard configuration.
</Note>

### Path B: AWS Bedrock through Portkey Model Catalog

Route requests to AWS Bedrock using a pre-configured provider slug in Portkey's Model Catalog. AWS credentials are stored in Portkey's dashboard, so no local AWS keys are needed.

```bash title=".env"
PORTKEY_URL=https://api.portkey.ai/v1
PORTKEY_API_KEY=pk-xxx
PORTKEY_PROVIDER=anthropic
PORTKEY_PROVIDER_SLUG=@bedrock-sandbox
```

`PORTKEY_PROVIDER=anthropic` tells Fabro to use the Anthropic Messages API format (which Bedrock Claude models accept). `PORTKEY_PROVIDER_SLUG=@bedrock-sandbox` tells Portkey to route to your Bedrock Model Catalog provider.

**Model IDs must use Bedrock inference profile format** — region-prefixed identifiers instead of standard Anthropic names:

```css
* { model: eu.anthropic.claude-sonnet-4-6; }
.code { model: eu.anthropic.claude-opus-4-6-v1; }
```

Common Bedrock inference profile IDs for Claude models:

| Model | EU | US | Global |
|-------|----|----|--------|
| Claude Sonnet 4.6 | `eu.anthropic.claude-sonnet-4-6` | `us.anthropic.claude-sonnet-4-6` | `global.anthropic.claude-sonnet-4-6` |
| Claude Opus 4.6 | `eu.anthropic.claude-opus-4-6-v1` | `us.anthropic.claude-opus-4-6-v1` | `global.anthropic.claude-opus-4-6-v1` |
| Claude Haiku 4.5 | `eu.anthropic.claude-haiku-4-5-20251001-v1:0` | `us.anthropic.claude-haiku-4-5-20251001-v1:0` | `global.anthropic.claude-haiku-4-5-20251001-v1:0` |
| Claude Opus 4.5 | `eu.anthropic.claude-opus-4-5-20251101-v1:0` | `us.anthropic.claude-opus-4-5-20251101-v1:0` | `global.anthropic.claude-opus-4-5-20251101-v1:0` |

Full list: [AWS Bedrock cross-region inference profiles](https://docs.aws.amazon.com/bedrock/latest/userguide/inference-profiles-support.html)

### Path C: AWS Bedrock with direct AWS credentials

Route to Bedrock by passing AWS credentials through Portkey headers, without configuring a Model Catalog provider.

```bash title=".env"
PORTKEY_URL=https://api.portkey.ai/v1
PORTKEY_API_KEY=pk-xxx
PORTKEY_PROVIDER=anthropic
PORTKEY_PROVIDER_SLUG=bedrock
PORTKEY_AWS_ACCESS_KEY_ID=AKIA...
PORTKEY_AWS_SECRET_ACCESS_KEY=xxx
PORTKEY_AWS_REGION=eu-west-1
# PORTKEY_AWS_SESSION_TOKEN=    # Optional: for STS/assumed-role access
```

Uses the same Bedrock inference profile model IDs as Path B.

### Path D: Config-based routing (fallbacks, load balancing)

Use a Portkey config for advanced routing strategies.

```bash title=".env"
PORTKEY_URL=https://api.portkey.ai/v1
PORTKEY_API_KEY=pk-xxx
PORTKEY_PROVIDER=anthropic
PORTKEY_CONFIG=cfg-xxx
```

The config defines fallback chains, load balancing weights, or conditional routing rules. Create configs in the [Portkey dashboard](https://app.portkey.ai). `PORTKEY_PROVIDER` still determines the request format — set it to match the primary provider in your config.

### Path E: OpenAI or Gemini through Portkey

Works identically to Path A with a different provider value.

```bash title=".env (OpenAI)"
PORTKEY_URL=https://api.portkey.ai/v1
PORTKEY_API_KEY=pk-xxx
PORTKEY_PROVIDER=openai
```

```bash title=".env (Gemini)"
PORTKEY_URL=https://api.portkey.ai/v1
PORTKEY_API_KEY=pk-xxx
PORTKEY_PROVIDER=gemini
```

For Azure OpenAI, set `PORTKEY_PROVIDER=openai` and `PORTKEY_PROVIDER_SLUG=@azure-prod` (your Azure Model Catalog provider slug).

## Environment variable reference

| Variable | Required | Description |
|----------|----------|-------------|
| `PORTKEY_URL` | Yes | Gateway base URL |
| `PORTKEY_API_KEY` | Yes | Portkey API key |
| `PORTKEY_PROVIDER` | Yes | Provider enum (`anthropic`, `openai`, `gemini`, `kimi`, `zai`, `minimax`, `inception`) — determines request format |
| `PORTKEY_PROVIDER_SLUG` | No | `x-portkey-provider` header value. Defaults to `PORTKEY_PROVIDER`. Set when the Portkey routing target differs from the adapter (e.g. `@bedrock-sandbox`, `@azure-prod`). |
| `PORTKEY_CONFIG` | No | Config ID or inline JSON for fallbacks/load balancing |
| `PORTKEY_AWS_ACCESS_KEY_ID` | No | AWS access key for direct Bedrock (Path C) |
| `PORTKEY_AWS_SECRET_ACCESS_KEY` | No | AWS secret key |
| `PORTKEY_AWS_REGION` | No | AWS region (defaults to `us-east-1`) |
| `PORTKEY_AWS_SESSION_TOKEN` | No | AWS STS session token |
| `PORTKEY_METADATA` | No | JSON metadata for Portkey observability |

## Known limitations

- **All-or-nothing routing.** When Portkey is configured, all requests for the specified provider go through Portkey. You cannot route some models through Portkey and others directly. For selective routing, use `PORTKEY_CONFIG` with conditional routing rules.
- **Single provider per configuration.** `PORTKEY_PROVIDER` specifies one adapter type. Multi-provider setups (e.g. Anthropic + OpenAI through Portkey) require a `PORTKEY_CONFIG` that handles routing internally.
- **No model ID translation.** Fabro passes the model ID from your stylesheet verbatim. When targeting Bedrock, you must use Bedrock inference profile IDs, not standard Anthropic model names.

## Troubleshooting

**"PORTKEY_PROVIDER is not a recognized provider name"** — The value must be one of: `anthropic`, `openai`, `gemini`, `kimi`, `zai`, `minimax`, `inception`. Check for typos.

**Requests fail with 401 from Portkey** — Verify `PORTKEY_API_KEY` is correct. Check the Portkey dashboard logs for details.

**Requests fail with "model not found" from Bedrock** — You're likely using a standard Anthropic model ID instead of a Bedrock inference profile ID. Use the region-prefixed format: `eu.anthropic.claude-sonnet-4-6`.

**Requests fail with 403 from Bedrock** — Your AWS credentials (via Model Catalog or direct `PORTKEY_AWS_*` headers) lack the `bedrock:InvokeModel` and `bedrock:GetInferenceProfile` IAM permissions.

**Portkey env vars are set but requests go directly to the provider** — All three required variables (`PORTKEY_URL`, `PORTKEY_API_KEY`, `PORTKEY_PROVIDER`) must be set. If any is missing, Portkey integration is silently disabled.

## Further reading

<Columns cols={2}>
  <Card title="Portkey Docs" icon="book" href="https://portkey.ai/docs">
    Portkey documentation — configs, provider setup, observability.
  </Card>
  <Card title="Bedrock Models" icon="cloud" href="https://docs.aws.amazon.com/bedrock/latest/userguide/inference-profiles-support.html">
    AWS Bedrock cross-region inference profile model IDs.
  </Card>
</Columns>
````

- [ ] **Step 2: Add to docs nav**

In `docs/docs.json`, find the Integrations group (`navigation.tabs[0].groups[6]`) and add `"integrations/portkey"` after `"integrations/brave-search"`:

The `pages` array should become:
```json
"pages": [
  "integrations/github",
  "integrations/daytona",
  "integrations/slack",
  "integrations/brave-search",
  "integrations/portkey"
]
```

- [ ] **Step 3: Commit**

```bash
git add docs/integrations/portkey.mdx docs/docs.json
git commit -m "docs: add Portkey AI gateway integration guide"
```

---

### Task 7: E2E live tests

**Files:**
- Modify: `lib/crates/fabro-llm/tests/integration.rs`

These tests require `PORTKEY_URL`, `PORTKEY_API_KEY`, and `PORTKEY_PROVIDER` set in `.env`. They are live-only (no twin mock for Portkey).

- [ ] **Step 1: Add Portkey e2e test**

Add to `lib/crates/fabro-llm/tests/integration.rs`:

```rust
#[fabro_macros::e2e_test(live("PORTKEY_API_KEY"))]
async fn portkey_anthropic_complete() {
    let portkey_url =
        std::env::var("PORTKEY_URL").expect("PORTKEY_URL must be set for Portkey e2e");
    let portkey_api_key =
        std::env::var("PORTKEY_API_KEY").expect("PORTKEY_API_KEY must be set for Portkey e2e");
    let provider_slug = std::env::var("PORTKEY_PROVIDER_SLUG").ok();

    let mut headers = std::collections::HashMap::new();
    headers.insert("x-portkey-api-key".to_string(), portkey_api_key);

    let slug = provider_slug
        .as_deref()
        .unwrap_or("anthropic");
    headers.insert("x-portkey-provider".to_string(), slug.to_string());

    let adapter = AnthropicAdapter::new("pk-portkey-dummy")
        .with_base_url(portkey_url)
        .with_default_headers(headers);

    // Use a model appropriate for the provider slug. Default to standard
    // Anthropic; Bedrock users should set PORTKEY_TEST_MODEL.
    let model = std::env::var("PORTKEY_TEST_MODEL")
        .unwrap_or_else(|_| "claude-haiku-4-5".to_string());
    let request = make_request(&model);
    let response = adapter.complete(&request).await.unwrap();

    assert!(
        !response.text().is_empty(),
        "response text should not be empty"
    );
    assert_eq!(response.finish_reason, FinishReason::Stop);
    assert!(response.usage.input_tokens > 0);
    assert!(response.usage.output_tokens > 0);
}
```

- [ ] **Step 2: Run e2e test (requires Portkey credentials in `.env`)**

Run: `set -a && source .env && set +a && cargo nextest run -p fabro-llm --profile e2e --run-ignored only -- portkey`
Expected: PASS (if Portkey credentials are configured).

- [ ] **Step 3: Commit**

```bash
git add lib/crates/fabro-llm/tests/integration.rs
git commit -m "test(fabro-llm): add Portkey e2e live test"
```

---

### Task 8: Final verification

**Files:** None — verification only.

- [ ] **Step 1: Run full `fabro-llm` test suite**

Run: `cargo nextest run -p fabro-llm`
Expected: All tests pass including 24 portkey unit tests.

- [ ] **Step 2: Run workspace build**

Run: `cargo build --workspace`
Expected: Clean build, no warnings.

- [ ] **Step 3: Run clippy**

Run: `cargo +nightly-2026-04-14 clippy --workspace --all-targets -- -D warnings`
Expected: No warnings.

- [ ] **Step 4: Run format check**

Run: `cargo +nightly-2026-04-14 fmt --check --all`
Expected: No formatting issues.

- [ ] **Step 5: Commit any fixes from linting**

If clippy or fmt flagged issues, fix and commit:

```bash
git add -A
git commit -m "style: fix clippy/fmt issues in portkey module"
```
