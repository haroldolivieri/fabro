use std::collections::HashMap;
use std::str::FromStr;

use fabro_auth::{ApiCredential, ApiKeyHeader};
use fabro_model::Provider;
use tracing::warn;

// ---------------------------------------------------------------------------
// AwsCredentials
// ---------------------------------------------------------------------------

/// AWS credentials for Portkey's AWS Bedrock passthrough.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AwsCredentials {
    pub access_key_id:     String,
    pub secret_access_key: String,
    pub region:            String,
    pub session_token:     Option<String>,
}

// ---------------------------------------------------------------------------
// PortkeyConfig
// ---------------------------------------------------------------------------

/// Configuration for routing requests through the Portkey AI gateway.
///
/// Constructed from environment variables via [`PortkeyConfig::from_env`].
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PortkeyConfig {
    pub base_url:      String,         // PORTKEY_URL
    pub api_key:       String,         // PORTKEY_API_KEY
    pub provider:      Provider,       // PORTKEY_PROVIDER (mandatory Provider enum)
    pub provider_slug: Option<String>, // PORTKEY_PROVIDER_SLUG
    pub config:        Option<String>, // PORTKEY_CONFIG
    pub metadata:      Option<String>, // PORTKEY_METADATA
    pub aws:           Option<AwsCredentials>,
}

impl PortkeyConfig {
    /// Load `PortkeyConfig` using a custom key-lookup function.
    ///
    /// `lookup` is called with env var names and returns `Some(value)` when
    /// found. Use this to read from vault, injected test maps, or any other
    /// source. [`PortkeyConfig::from_env`] delegates here with `std::env::var`.
    ///
    /// Returns `None` if any required variable is missing or invalid.
    #[must_use]
    pub fn from_lookup(lookup: impl Fn(&str) -> Option<String>) -> Option<Self> {
        let base_url = lookup("PORTKEY_URL")?;
        let api_key  = lookup("PORTKEY_API_KEY")?;

        let provider_str = lookup("PORTKEY_PROVIDER")?;
        let provider = match Provider::from_str(&provider_str) {
            Ok(p) => p,
            Err(e) => {
                warn!(value = %provider_str, error = %e, "PORTKEY_PROVIDER is not a valid provider");
                return None;
            }
        };

        let provider_slug = lookup("PORTKEY_PROVIDER_SLUG");
        let config        = lookup("PORTKEY_CONFIG");
        let metadata      = lookup("PORTKEY_METADATA");

        let aws = match (
            lookup("PORTKEY_AWS_ACCESS_KEY_ID"),
            lookup("PORTKEY_AWS_SECRET_ACCESS_KEY"),
        ) {
            (Some(access_key_id), Some(secret_access_key)) => {
                let region        = lookup("PORTKEY_AWS_REGION").unwrap_or_else(|| "us-east-1".to_string());
                let session_token = lookup("PORTKEY_AWS_SESSION_TOKEN");
                Some(AwsCredentials { access_key_id, secret_access_key, region, session_token })
            }
            _ => None,
        };

        Some(Self { base_url, api_key, provider, provider_slug, config, metadata, aws })
    }

    /// Load `PortkeyConfig` from environment variables.
    ///
    /// Delegates to [`PortkeyConfig::from_lookup`] with `std::env::var`.
    #[must_use]
    pub fn from_env() -> Option<Self> {
        Self::from_lookup(|k| std::env::var(k).ok())
    }

    /// Build the Portkey-specific HTTP headers that must be injected into
    /// every outbound request routed through the gateway.
    fn build_headers(&self) -> HashMap<String, String> {
        let mut headers = HashMap::new();

        headers.insert("x-portkey-api-key".to_string(), self.api_key.clone());

        let provider_value = self
            .provider_slug
            .as_deref()
            .unwrap_or_else(|| self.provider.as_str())
            .to_string();
        headers.insert("x-portkey-provider".to_string(), provider_value);

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
            headers.insert("x-portkey-aws-region".to_string(), aws.region.clone());
            if let Some(session_token) = &aws.session_token {
                headers.insert(
                    "x-portkey-aws-session-token".to_string(),
                    session_token.clone(),
                );
            }
        }

        headers
    }

    /// Build a dummy auth header for a provider.
    ///
    /// Portkey acts as the real auth layer; the underlying provider key is
    /// replaced with a sentinel value that signals "routed via Portkey".
    fn dummy_auth_header(provider: Provider) -> ApiKeyHeader {
        match provider {
            Provider::Anthropic => ApiKeyHeader::Custom {
                name:  "x-api-key".to_string(),
                value: "pk-portkey-dummy".to_string(),
            },
            _ => ApiKeyHeader::Bearer("pk-portkey-dummy".to_string()),
        }
    }

    /// Inject Portkey gateway headers into the credential matching
    /// `self.provider`.
    ///
    /// If no matching credential exists a new one is created with a dummy
    /// auth key.  The Portkey headers are *inserted* into `extra_headers`
    /// (existing keys are preserved).
    pub fn apply(&self, credentials: &mut Vec<ApiCredential>) {
        let portkey_headers = self.build_headers();

        if let Some(credential) = credentials.iter_mut().find(|c| c.provider == self.provider) {
            credential.base_url = Some(self.base_url.clone());
            for (key, value) in portkey_headers {
                credential.extra_headers.insert(key, value);
            }
        } else {
            credentials.push(ApiCredential {
                provider:      self.provider,
                auth_header:   Self::dummy_auth_header(self.provider),
                extra_headers: portkey_headers,
                base_url:      Some(self.base_url.clone()),
                codex_mode:    false,
                org_id:        None,
                project_id:    None,
            });
        }
    }
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;

    /// Remove all PORTKEY_* and AWS credential env vars so tests start clean.
    fn clear_portkey_env() {
        for var in &[
            "PORTKEY_URL",
            "PORTKEY_API_KEY",
            "PORTKEY_PROVIDER",
            "PORTKEY_PROVIDER_SLUG",
            "PORTKEY_CONFIG",
            "PORTKEY_METADATA",
            "PORTKEY_AWS_ACCESS_KEY_ID",
            "PORTKEY_AWS_SECRET_ACCESS_KEY",
            "PORTKEY_AWS_REGION",
            "PORTKEY_AWS_SESSION_TOKEN",
        ] {
            std::env::remove_var(var);
        }
    }

    fn empty_credential(provider: Provider) -> ApiCredential {
        ApiCredential {
            provider,
            auth_header: ApiKeyHeader::Bearer("real-key".to_string()),
            extra_headers: HashMap::new(),
            base_url: None,
            codex_mode: false,
            org_id: None,
            project_id: None,
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

    // -----------------------------------------------------------------------
    // from_env — failing cases
    // -----------------------------------------------------------------------

    #[test]
    fn from_env_returns_none_when_url_missing() {
        clear_portkey_env();
        std::env::set_var("PORTKEY_API_KEY", "key");
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
        std::env::set_var("PORTKEY_API_KEY", "key");
        assert!(PortkeyConfig::from_env().is_none());
    }

    #[test]
    fn from_env_returns_none_on_invalid_provider() {
        clear_portkey_env();
        std::env::set_var("PORTKEY_URL", "https://api.portkey.ai/v1");
        std::env::set_var("PORTKEY_API_KEY", "key");
        std::env::set_var("PORTKEY_PROVIDER", "not_a_real_provider");
        assert!(PortkeyConfig::from_env().is_none());
    }

    // -----------------------------------------------------------------------
    // from_env — success cases
    // -----------------------------------------------------------------------

    #[test]
    fn from_env_parses_all_required_fields() {
        clear_portkey_env();
        std::env::set_var("PORTKEY_URL", "https://api.portkey.ai/v1");
        std::env::set_var("PORTKEY_API_KEY", "pk-test-key");
        std::env::set_var("PORTKEY_PROVIDER", "openai");

        let cfg = PortkeyConfig::from_env().expect("should return Some");
        assert_eq!(cfg.base_url, "https://api.portkey.ai/v1");
        assert_eq!(cfg.api_key, "pk-test-key");
        assert_eq!(cfg.provider, Provider::OpenAi);
        assert!(cfg.provider_slug.is_none());
        assert!(cfg.config.is_none());
        assert!(cfg.metadata.is_none());
        assert!(cfg.aws.is_none());
    }

    #[test]
    fn from_env_parses_optional_fields() {
        clear_portkey_env();
        std::env::set_var("PORTKEY_URL", "https://api.portkey.ai/v1");
        std::env::set_var("PORTKEY_API_KEY", "pk-test-key");
        std::env::set_var("PORTKEY_PROVIDER", "anthropic");
        std::env::set_var("PORTKEY_PROVIDER_SLUG", "my-anthropic-slug");
        std::env::set_var("PORTKEY_CONFIG", "pc-config-abc");
        std::env::set_var("PORTKEY_METADATA", r#"{"user":"test"}"#);
        std::env::set_var("PORTKEY_AWS_ACCESS_KEY_ID", "AKIAIOSFODNN7EXAMPLE");
        std::env::set_var(
            "PORTKEY_AWS_SECRET_ACCESS_KEY",
            "wJalrXUtnFEMI/K7MDENG/bPxRfiCYEXAMPLEKEY",
        );
        std::env::set_var("PORTKEY_AWS_REGION", "eu-west-1");
        std::env::set_var("PORTKEY_AWS_SESSION_TOKEN", "session-tok");

        let cfg = PortkeyConfig::from_env().expect("should return Some");
        assert_eq!(cfg.provider, Provider::Anthropic);
        assert_eq!(cfg.provider_slug.as_deref(), Some("my-anthropic-slug"));
        assert_eq!(cfg.config.as_deref(), Some("pc-config-abc"));
        assert_eq!(cfg.metadata.as_deref(), Some(r#"{"user":"test"}"#));

        let aws = cfg.aws.expect("aws should be Some");
        assert_eq!(aws.access_key_id, "AKIAIOSFODNN7EXAMPLE");
        assert_eq!(
            aws.secret_access_key,
            "wJalrXUtnFEMI/K7MDENG/bPxRfiCYEXAMPLEKEY"
        );
        assert_eq!(aws.region, "eu-west-1");
        assert_eq!(aws.session_token.as_deref(), Some("session-tok"));
    }

    #[test]
    fn from_env_collects_aws_only_when_both_keys_present() {
        clear_portkey_env();
        std::env::set_var("PORTKEY_URL", "https://api.portkey.ai/v1");
        std::env::set_var("PORTKEY_API_KEY", "pk-test-key");
        std::env::set_var("PORTKEY_PROVIDER", "anthropic");
        // Only set access key, omit secret key
        std::env::set_var("PORTKEY_AWS_ACCESS_KEY_ID", "AKIAIOSFODNN7EXAMPLE");

        let cfg = PortkeyConfig::from_env().expect("should return Some");
        assert!(
            cfg.aws.is_none(),
            "aws should be None when secret is missing"
        );
    }

    // -----------------------------------------------------------------------
    // apply — new credential creation
    // -----------------------------------------------------------------------

    #[test]
    fn apply_creates_credential_when_none_exists() {
        let cfg = portkey_config_anthropic();
        let mut credentials: Vec<ApiCredential> = vec![];

        cfg.apply(&mut credentials);

        assert_eq!(credentials.len(), 1);
        let cred = &credentials[0];

        // Provider matches
        assert_eq!(cred.provider, Provider::Anthropic);

        // Dummy auth key was used (Custom header for Anthropic)
        assert_eq!(cred.auth_header, ApiKeyHeader::Custom {
            name:  "x-api-key".to_string(),
            value: "pk-portkey-dummy".to_string(),
        });

        // base_url set to Portkey URL
        assert_eq!(cred.base_url.as_deref(), Some("https://api.portkey.ai/v1"));

        // Portkey headers injected
        assert_eq!(
            cred.extra_headers
                .get("x-portkey-api-key")
                .map(String::as_str),
            Some("pk-test")
        );
        assert_eq!(
            cred.extra_headers
                .get("x-portkey-provider")
                .map(String::as_str),
            Some("anthropic")
        );
    }

    // -----------------------------------------------------------------------
    // apply — modifying an existing credential
    // -----------------------------------------------------------------------

    #[test]
    fn apply_modifies_existing_credential() {
        let cfg = portkey_config_anthropic();
        let original_header = ApiKeyHeader::Custom {
            name:  "x-api-key".to_string(),
            value: "real-anthropic-key".to_string(),
        };
        let mut credentials = vec![ApiCredential {
            provider:      Provider::Anthropic,
            auth_header:   original_header.clone(),
            extra_headers: HashMap::new(),
            base_url:      Some("https://old-url.example.com".to_string()),
            codex_mode:    false,
            org_id:        None,
            project_id:    None,
        }];

        cfg.apply(&mut credentials);

        assert_eq!(credentials.len(), 1);
        let cred = &credentials[0];

        // base_url overridden to Portkey URL
        assert_eq!(cred.base_url.as_deref(), Some("https://api.portkey.ai/v1"));

        // original auth_header preserved
        assert_eq!(cred.auth_header, original_header);

        // Portkey headers added
        assert!(cred.extra_headers.contains_key("x-portkey-api-key"));
        assert!(cred.extra_headers.contains_key("x-portkey-provider"));
    }

    #[test]
    fn apply_preserves_existing_extra_headers() {
        let cfg = portkey_config_anthropic();
        let mut existing_headers = HashMap::new();
        existing_headers.insert("ChatGPT-Account-Id".to_string(), "acct-123".to_string());
        existing_headers.insert("originator".to_string(), "fabro".to_string());

        let mut credentials = vec![ApiCredential {
            provider:      Provider::Anthropic,
            auth_header:   ApiKeyHeader::Bearer("real-key".to_string()),
            extra_headers: existing_headers,
            base_url:      None,
            codex_mode:    false,
            org_id:        None,
            project_id:    None,
        }];

        cfg.apply(&mut credentials);

        let cred = &credentials[0];

        // Original headers preserved
        assert_eq!(
            cred.extra_headers
                .get("ChatGPT-Account-Id")
                .map(String::as_str),
            Some("acct-123")
        );
        assert_eq!(
            cred.extra_headers.get("originator").map(String::as_str),
            Some("fabro")
        );

        // Portkey headers also present
        assert!(cred.extra_headers.contains_key("x-portkey-api-key"));
        assert!(cred.extra_headers.contains_key("x-portkey-provider"));
    }

    // -----------------------------------------------------------------------
    // apply — provider slug
    // -----------------------------------------------------------------------

    #[test]
    fn apply_sets_provider_slug_from_config() {
        let cfg = PortkeyConfig {
            provider_slug: Some("@bedrock-sandbox".to_string()),
            ..portkey_config_anthropic()
        };
        let mut credentials: Vec<ApiCredential> = vec![];
        cfg.apply(&mut credentials);

        assert_eq!(
            credentials[0]
                .extra_headers
                .get("x-portkey-provider")
                .map(String::as_str),
            Some("@bedrock-sandbox")
        );
    }

    #[test]
    fn apply_defaults_provider_slug_to_provider_name() {
        let cfg = portkey_config_anthropic(); // provider_slug = None
        let mut credentials: Vec<ApiCredential> = vec![];
        cfg.apply(&mut credentials);

        assert_eq!(
            credentials[0]
                .extra_headers
                .get("x-portkey-provider")
                .map(String::as_str),
            Some("anthropic")
        );
    }

    // -----------------------------------------------------------------------
    // apply — optional headers
    // -----------------------------------------------------------------------

    #[test]
    fn apply_injects_config_header() {
        let cfg = PortkeyConfig {
            config: Some("cfg-xxx".to_string()),
            ..portkey_config_anthropic()
        };
        let mut credentials: Vec<ApiCredential> = vec![];
        cfg.apply(&mut credentials);

        assert_eq!(
            credentials[0]
                .extra_headers
                .get("x-portkey-config")
                .map(String::as_str),
            Some("cfg-xxx")
        );
    }

    #[test]
    fn apply_injects_metadata_header() {
        let cfg = PortkeyConfig {
            metadata: Some(r#"{"user":"alice"}"#.to_string()),
            ..portkey_config_anthropic()
        };
        let mut credentials: Vec<ApiCredential> = vec![];
        cfg.apply(&mut credentials);

        assert_eq!(
            credentials[0]
                .extra_headers
                .get("x-portkey-metadata")
                .map(String::as_str),
            Some(r#"{"user":"alice"}"#)
        );
    }

    // -----------------------------------------------------------------------
    // apply — AWS headers
    // -----------------------------------------------------------------------

    #[test]
    fn apply_injects_aws_headers() {
        let cfg = PortkeyConfig {
            aws: Some(AwsCredentials {
                access_key_id:     "AKID".to_string(),
                secret_access_key: "SECRET".to_string(),
                region:            "us-west-2".to_string(),
                session_token:     Some("TOKEN".to_string()),
            }),
            ..portkey_config_anthropic()
        };
        let mut credentials: Vec<ApiCredential> = vec![];
        cfg.apply(&mut credentials);

        let headers = &credentials[0].extra_headers;
        assert_eq!(
            headers
                .get("x-portkey-aws-access-key-id")
                .map(String::as_str),
            Some("AKID")
        );
        assert_eq!(
            headers
                .get("x-portkey-aws-secret-access-key")
                .map(String::as_str),
            Some("SECRET")
        );
        assert_eq!(
            headers.get("x-portkey-aws-region").map(String::as_str),
            Some("us-west-2")
        );
        assert_eq!(
            headers
                .get("x-portkey-aws-session-token")
                .map(String::as_str),
            Some("TOKEN")
        );
    }

    #[test]
    fn apply_skips_aws_session_token_when_absent() {
        let cfg = PortkeyConfig {
            aws: Some(AwsCredentials {
                access_key_id:     "AKID".to_string(),
                secret_access_key: "SECRET".to_string(),
                region:            "us-east-1".to_string(),
                session_token:     None,
            }),
            ..portkey_config_anthropic()
        };
        let mut credentials: Vec<ApiCredential> = vec![];
        cfg.apply(&mut credentials);

        let headers = &credentials[0].extra_headers;
        assert!(headers.contains_key("x-portkey-aws-access-key-id"));
        assert!(headers.contains_key("x-portkey-aws-secret-access-key"));
        assert!(headers.contains_key("x-portkey-aws-region"));
        assert!(
            !headers.contains_key("x-portkey-aws-session-token"),
            "session token header should be absent when token is None"
        );
    }

    // -----------------------------------------------------------------------
    // apply — does not touch unrelated credentials
    // -----------------------------------------------------------------------

    #[test]
    fn apply_does_not_touch_other_credentials() {
        let cfg = portkey_config_anthropic(); // targets Anthropic
        let openai_cred = empty_credential(Provider::OpenAi);
        let anthropic_cred = empty_credential(Provider::Anthropic);
        let mut credentials = vec![anthropic_cred, openai_cred.clone()];

        cfg.apply(&mut credentials);

        // OpenAI credential is unchanged
        let openai_after = credentials
            .iter()
            .find(|c| c.provider == Provider::OpenAi)
            .expect("openai credential should still be present");
        assert_eq!(*openai_after, openai_cred);

        // Anthropic credential was modified
        let anthropic_after = credentials
            .iter()
            .find(|c| c.provider == Provider::Anthropic)
            .expect("anthropic credential should be present");
        assert!(
            anthropic_after
                .extra_headers
                .contains_key("x-portkey-api-key")
        );
    }

    // --- Scenario integration tests ---

    #[test]
    fn scenario_a_direct_provider() {
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
            credentials[0].extra_headers.get("x-portkey-provider"),
            Some(&"anthropic".to_string())
        );
        assert_eq!(credentials[0].auth_header, ApiKeyHeader::Custom {
            name:  "x-api-key".to_string(),
            value: "pk-portkey-dummy".to_string(),
        });
    }

    #[test]
    fn scenario_b_bedrock_model_catalog() {
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
        assert!(
            !credentials[0]
                .extra_headers
                .contains_key("x-portkey-aws-access-key-id")
        );
    }

    #[test]
    fn scenario_c_bedrock_direct_aws() {
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
            credentials[0]
                .extra_headers
                .get("x-portkey-aws-access-key-id"),
            Some(&"AKIA...".to_string())
        );
    }

    #[test]
    fn scenario_d_config_routing() {
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
        assert!(
            credentials[0]
                .extra_headers
                .contains_key("x-portkey-provider")
        );
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
        assert_eq!(credentials[0].auth_header, ApiKeyHeader::Custom {
            name:  "x-api-key".to_string(),
            value: "sk-ant-real-key".to_string(),
        });
        assert_eq!(
            credentials[0].base_url.as_deref(),
            Some("https://api.portkey.ai/v1")
        );
        assert!(
            credentials[0]
                .extra_headers
                .contains_key("x-portkey-api-key")
        );
    }

    // -----------------------------------------------------------------------
    // from_lookup tests
    // -----------------------------------------------------------------------

    #[test]
    fn from_lookup_returns_none_when_url_missing() {
        let lookup = |k: &str| match k {
            "PORTKEY_API_KEY" => Some("pk-test".to_string()),
            "PORTKEY_PROVIDER" => Some("anthropic".to_string()),
            _ => None,
        };
        assert!(PortkeyConfig::from_lookup(lookup).is_none());
    }

    #[test]
    fn from_lookup_parses_all_required_fields() {
        let lookup = |k: &str| match k {
            "PORTKEY_URL"      => Some("https://api.portkey.ai/v1".to_string()),
            "PORTKEY_API_KEY"  => Some("pk-test".to_string()),
            "PORTKEY_PROVIDER" => Some("anthropic".to_string()),
            _                  => None,
        };
        let config = PortkeyConfig::from_lookup(lookup).unwrap();
        assert_eq!(config.base_url, "https://api.portkey.ai/v1");
        assert_eq!(config.api_key, "pk-test");
        assert_eq!(config.provider, Provider::Anthropic);
        assert!(config.provider_slug.is_none());
        assert!(config.aws.is_none());
    }

    #[test]
    fn from_lookup_parses_optional_fields() {
        let lookup = |k: &str| match k {
            "PORTKEY_URL"                    => Some("https://api.portkey.ai/v1".to_string()),
            "PORTKEY_API_KEY"                => Some("pk-key".to_string()),
            "PORTKEY_PROVIDER"               => Some("anthropic".to_string()),
            "PORTKEY_PROVIDER_SLUG"          => Some("@bedrock-sandbox".to_string()),
            "PORTKEY_CONFIG"                 => Some("cfg-abc".to_string()),
            "PORTKEY_METADATA"               => Some(r#"{"team":"eng"}"#.to_string()),
            "PORTKEY_AWS_ACCESS_KEY_ID"      => Some("AKIA...".to_string()),
            "PORTKEY_AWS_SECRET_ACCESS_KEY"  => Some("secret".to_string()),
            "PORTKEY_AWS_REGION"             => Some("eu-west-1".to_string()),
            _                                => None,
        };
        let config = PortkeyConfig::from_lookup(lookup).unwrap();
        assert_eq!(config.provider_slug.as_deref(), Some("@bedrock-sandbox"));
        assert_eq!(config.config.as_deref(), Some("cfg-abc"));
        assert_eq!(config.aws.as_ref().unwrap().region, "eu-west-1");
    }

    #[test]
    fn from_env_delegates_to_from_lookup() {
        clear_portkey_env();
        std::env::set_var("PORTKEY_URL", "https://api.portkey.ai/v1");
        std::env::set_var("PORTKEY_API_KEY", "pk-env");
        std::env::set_var("PORTKEY_PROVIDER", "openai");
        let config = PortkeyConfig::from_env().unwrap();
        assert_eq!(config.api_key, "pk-env");
        assert_eq!(config.provider, Provider::OpenAi);
    }
}
