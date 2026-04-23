use async_trait::async_trait;
use fabro_model::Provider;

use crate::{ApiCredential, ResolveError};

#[derive(Debug)]
pub struct ResolvedCredentials {
    pub credentials: Vec<ApiCredential>,
    pub auth_issues: Vec<(Provider, ResolveError)>,
}

#[async_trait]
pub trait CredentialSource: Send + Sync {
    async fn resolve(&self) -> anyhow::Result<ResolvedCredentials>;

    async fn configured_providers(&self) -> Vec<Provider>;
}

#[must_use]
pub fn auth_issue_message(provider: Provider, err: &ResolveError) -> String {
    match err {
        ResolveError::NotConfigured(_) => {
            format!("{} is not configured", provider.display_name())
        }
        ResolveError::RefreshFailed { source, .. } => format!(
            "{} requires re-authentication: {}",
            provider.display_name(),
            source
        ),
        ResolveError::RefreshTokenMissing(_) => format!(
            "{} requires re-authentication: refresh token missing",
            provider.display_name()
        ),
    }
}

#[cfg(test)]
mod tests {
    use fabro_model::Provider;

    use super::auth_issue_message;
    use crate::ResolveError;

    #[test]
    fn auth_issue_message_formats_refresh_token_missing() {
        let message = auth_issue_message(
            Provider::OpenAi,
            &ResolveError::RefreshTokenMissing(Provider::OpenAi),
        );

        assert_eq!(
            message,
            "OpenAI requires re-authentication: refresh token missing"
        );
    }
}
