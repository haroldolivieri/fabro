use std::str::FromStr;

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
    pub base_url:      String,          // PORTKEY_URL
    pub api_key:       String,          // PORTKEY_API_KEY
    pub provider:      Provider,        // PORTKEY_PROVIDER (mandatory Provider enum)
    pub provider_slug: Option<String>,  // PORTKEY_PROVIDER_SLUG
    pub config:        Option<String>,  // PORTKEY_CONFIG
    pub metadata:      Option<String>,  // PORTKEY_METADATA
    pub aws:           Option<AwsCredentials>,
}

impl PortkeyConfig {
    /// Build a `PortkeyConfig` from environment variables.
    ///
    /// Returns `None` if any required variable is missing or invalid:
    /// - `PORTKEY_URL`
    /// - `PORTKEY_API_KEY`
    /// - `PORTKEY_PROVIDER` (must be a valid [`Provider`] string)
    ///
    /// AWS credentials are included only when both
    /// `AWS_ACCESS_KEY_ID` and `AWS_SECRET_ACCESS_KEY` are set.
    /// `AWS_DEFAULT_REGION` defaults to `"us-east-1"` when omitted.
    #[must_use]
    pub fn from_env() -> Option<Self> {
        let base_url = std::env::var("PORTKEY_URL").ok()?;
        let api_key  = std::env::var("PORTKEY_API_KEY").ok()?;

        let provider_str = std::env::var("PORTKEY_PROVIDER").ok()?;
        let provider = match Provider::from_str(&provider_str) {
            Ok(p) => p,
            Err(e) => {
                warn!(value = %provider_str, error = %e, "PORTKEY_PROVIDER is not a valid provider");
                return None;
            }
        };

        let provider_slug = std::env::var("PORTKEY_PROVIDER_SLUG").ok();
        let config        = std::env::var("PORTKEY_CONFIG").ok();
        let metadata      = std::env::var("PORTKEY_METADATA").ok();

        let aws = match (
            std::env::var("AWS_ACCESS_KEY_ID").ok(),
            std::env::var("AWS_SECRET_ACCESS_KEY").ok(),
        ) {
            (Some(access_key_id), Some(secret_access_key)) => {
                let region = std::env::var("AWS_DEFAULT_REGION")
                    .unwrap_or_else(|_| "us-east-1".to_string());
                let session_token = std::env::var("AWS_SESSION_TOKEN").ok();
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
            "AWS_ACCESS_KEY_ID",
            "AWS_SECRET_ACCESS_KEY",
            "AWS_DEFAULT_REGION",
            "AWS_SESSION_TOKEN",
        ] {
            std::env::remove_var(var);
        }
    }

    // -----------------------------------------------------------------------
    // Step 1 — failing cases
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
    // Step 5 — success cases
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
        std::env::set_var("AWS_ACCESS_KEY_ID", "AKIAIOSFODNN7EXAMPLE");
        std::env::set_var("AWS_SECRET_ACCESS_KEY", "wJalrXUtnFEMI/K7MDENG/bPxRfiCYEXAMPLEKEY");
        std::env::set_var("AWS_DEFAULT_REGION", "eu-west-1");
        std::env::set_var("AWS_SESSION_TOKEN", "session-tok");

        let cfg = PortkeyConfig::from_env().expect("should return Some");
        assert_eq!(cfg.provider, Provider::Anthropic);
        assert_eq!(cfg.provider_slug.as_deref(), Some("my-anthropic-slug"));
        assert_eq!(cfg.config.as_deref(), Some("pc-config-abc"));
        assert_eq!(cfg.metadata.as_deref(), Some(r#"{"user":"test"}"#));

        let aws = cfg.aws.expect("aws should be Some");
        assert_eq!(aws.access_key_id, "AKIAIOSFODNN7EXAMPLE");
        assert_eq!(aws.secret_access_key, "wJalrXUtnFEMI/K7MDENG/bPxRfiCYEXAMPLEKEY");
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
        std::env::set_var("AWS_ACCESS_KEY_ID", "AKIAIOSFODNN7EXAMPLE");

        let cfg = PortkeyConfig::from_env().expect("should return Some");
        assert!(cfg.aws.is_none(), "aws should be None when secret is missing");
    }
}
