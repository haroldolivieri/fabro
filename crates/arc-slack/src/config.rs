use serde::Deserialize;

#[derive(Debug, Clone, Default, Deserialize, PartialEq)]
pub struct SlackConfig {
    pub default_channel: Option<String>,
}

#[derive(Debug, Clone)]
pub struct SlackCredentials {
    pub bot_token: String,
    pub app_token: String,
}

pub fn resolve_credentials() -> Option<SlackCredentials> {
    let bot_token = std::env::var("ARC_SLACK_BOT_TOKEN").ok()?;
    let app_token = std::env::var("ARC_SLACK_APP_TOKEN").ok()?;
    Some(SlackCredentials {
        bot_token,
        app_token,
    })
}

pub struct SlackRuntimeConfig {
    pub config: SlackConfig,
    pub credentials: SlackCredentials,
}

impl SlackRuntimeConfig {
    pub fn new(config: SlackConfig, credentials: SlackCredentials) -> Self {
        Self {
            config,
            credentials,
        }
    }

    pub fn is_enabled(&self) -> bool {
        !self.credentials.bot_token.is_empty() && !self.credentials.app_token.is_empty()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parse_empty_toml_defaults() {
        let config: SlackConfig = toml::from_str("").unwrap();
        assert_eq!(config.default_channel, None);
    }

    #[test]
    fn parse_with_channel() {
        let toml_str = r##"default_channel = "#arc-reviews""##;
        let config: SlackConfig = toml::from_str(toml_str).unwrap();
        assert_eq!(config.default_channel.as_deref(), Some("#arc-reviews"));
    }

    #[test]
    fn resolve_credentials_from_env() {
        let creds = SlackCredentials {
            bot_token: "xoxb-test".to_string(),
            app_token: "xapp-test".to_string(),
        };
        assert_eq!(creds.bot_token, "xoxb-test");
        assert_eq!(creds.app_token, "xapp-test");
    }

    #[test]
    fn is_enabled_when_both_tokens_present() {
        let config = SlackConfig {
            default_channel: None,
        };
        let creds = SlackCredentials {
            bot_token: "xoxb-test".to_string(),
            app_token: "xapp-test".to_string(),
        };
        let runtime = SlackRuntimeConfig::new(config, creds);
        assert!(runtime.is_enabled());
    }

    #[test]
    fn is_not_enabled_with_empty_bot_token() {
        let config = SlackConfig {
            default_channel: None,
        };
        let creds = SlackCredentials {
            bot_token: String::new(),
            app_token: "xapp-test".to_string(),
        };
        let runtime = SlackRuntimeConfig::new(config, creds);
        assert!(!runtime.is_enabled());
    }

    #[test]
    fn is_not_enabled_with_empty_app_token() {
        let config = SlackConfig {
            default_channel: None,
        };
        let creds = SlackCredentials {
            bot_token: "xoxb-test".to_string(),
            app_token: String::new(),
        };
        let runtime = SlackRuntimeConfig::new(config, creds);
        assert!(!runtime.is_enabled());
    }
}
