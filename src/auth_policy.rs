//! Authentication policy read from the application's `settings:` block.
//!
//! Loco interpolates environment variables into the YAML configuration, so the
//! controller and tasks read the policy from `ctx.config` rather than calling
//! `std::env::var` themselves. That keeps configuration in one place and makes
//! it testable by booting the app with a different environment.

use loco_rs::{config::Config, Error};
use serde::Deserialize;

/// The complete application settings block.
#[derive(Debug, Clone, Default, Deserialize)]
#[serde(default)]
pub struct AppSettings {
    pub auth: AuthSettings,
}

/// Authentication policy configured under `settings.auth`.
#[derive(Debug, Clone, Default, Deserialize)]
#[serde(default)]
pub struct AuthSettings {
    /// Whether visitors may create their own accounts.
    pub allow_registration: bool,
    /// Whether an account must verify its email before logging in.
    pub require_email_verification: bool,
}

/// Resolved authentication policy used by controllers and tasks.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct AuthPolicy {
    pub allow_registration: bool,
    pub require_email_verification: bool,
}

impl AuthPolicy {
    /// Read the policy from the application configuration.
    ///
    /// # Errors
    ///
    /// Returns an error when the `settings:` block is present but malformed.
    pub fn from_config(config: &Config) -> Result<Self, Error> {
        let settings: AppSettings = config.settings()?;
        Ok(Self::from(settings.auth))
    }
}

impl From<AuthSettings> for AuthPolicy {
    fn from(settings: AuthSettings) -> Self {
        Self {
            allow_registration: settings.allow_registration,
            require_email_verification: settings.require_email_verification,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::{AppSettings, AuthPolicy};
    use serde_json::json;

    #[test]
    fn defaults_to_closed_registration() {
        let settings: AppSettings = serde_json::from_value(json!({})).expect("empty settings");
        let policy = AuthPolicy::from(settings.auth);

        assert!(!policy.allow_registration);
        assert!(!policy.require_email_verification);
    }

    #[test]
    fn reads_the_configured_policy() {
        let settings: AppSettings = serde_json::from_value(json!({
            "auth": {
                "allow_registration": true,
                "require_email_verification": true
            }
        }))
        .expect("valid settings");

        let policy = AuthPolicy::from(settings.auth);

        assert!(policy.allow_registration);
        assert!(policy.require_email_verification);
    }
}
