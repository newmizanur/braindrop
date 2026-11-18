//! Typed access to the `settings.ai` configuration block.
//!
//! Everything here is server-side configuration. Users may choose a model and
//! whether to use their own key, but they can never point BrainDrop at an
//! arbitrary provider URL.

use std::time::Duration;

use loco_rs::{config::Config, Error};
use serde::{Deserialize, Deserializer};

use crate::ai::error::AiError;

/// YAML renders an unset environment variable as `key:` — which serde reads as
/// `null`, not `""`. Treat null as an empty string for the optional AI values so
/// an installation without credentials boots normally.
fn string_or_empty<'de, D>(deserializer: D) -> Result<String, D::Error>
where
    D: Deserializer<'de>,
{
    Ok(Option::<String>::deserialize(deserializer)?.unwrap_or_default())
}

/// The AI settings read from `config/<env>.yaml`.
#[derive(Clone, Debug, Deserialize)]
#[serde(default)]
pub struct AiSettings {
    /// Server-wide switch. A user can still disable AI for their own account.
    pub enabled: bool,
    /// Server-provided OpenRouter key. Empty means "not configured".
    #[serde(deserialize_with = "string_or_empty")]
    pub api_key: String,
    /// Default model used when the account does not choose one.
    #[serde(deserialize_with = "string_or_empty")]
    pub model: String,
    /// OpenRouter-compatible API root. Never user-configurable.
    #[serde(deserialize_with = "string_or_empty")]
    pub base_url: String,
    pub timeout_seconds: u64,
    pub max_input_chars: usize,
    /// Maximum provider response size, in bytes.
    pub max_output_chars: usize,
    pub max_items: usize,
    pub rate_limit_per_minute: u32,
    /// Base64-encoded 32-byte key used to seal personal provider keys.
    #[serde(deserialize_with = "string_or_empty")]
    pub encryption_key: String,
}

impl Default for AiSettings {
    fn default() -> Self {
        Self {
            enabled: false,
            api_key: String::new(),
            model: String::new(),
            base_url: "https://openrouter.ai/api/v1".to_string(),
            timeout_seconds: 30,
            max_input_chars: 8_000,
            max_output_chars: 64_000,
            max_items: 20,
            rate_limit_per_minute: 10,
            encryption_key: String::new(),
        }
    }
}

#[derive(Clone, Debug, Default, Deserialize)]
#[serde(default)]
struct AiConfigRoot {
    ai: AiSettings,
}

impl AiSettings {
    /// Read the AI block from the application configuration.
    ///
    /// # Errors
    ///
    /// Returns an error when the `settings:` block is present but malformed.
    pub fn from_config(config: &Config) -> Result<Self, Error> {
        let root: AiConfigRoot = config.settings()?;
        Ok(root.ai)
    }

    /// The configured server key, if it is non-empty.
    #[must_use]
    pub fn server_key(&self) -> Option<&str> {
        let key = self.api_key.trim();
        (!key.is_empty()).then_some(key)
    }

    /// The configured default model, if it is non-empty.
    #[must_use]
    pub fn server_model(&self) -> Option<&str> {
        let model = self.model.trim();
        (!model.is_empty()).then_some(model)
    }

    /// A validated, trailing-slash-free API root.
    ///
    /// # Errors
    ///
    /// Returns `NotConfigured` when the configured URL is not an absolute
    /// `http(s)` URL with a host.
    pub fn endpoint(&self) -> Result<String, AiError> {
        let raw = self.base_url.trim().trim_end_matches('/');
        if raw.is_empty() {
            return Err(AiError::NotConfigured(
                "the server AI endpoint is empty".to_string(),
            ));
        }
        let url = reqwest::Url::parse(raw).map_err(|_| {
            AiError::NotConfigured("the server AI endpoint is not a valid URL".to_string())
        })?;
        if !matches!(url.scheme(), "http" | "https") || url.host_str().is_none() {
            return Err(AiError::NotConfigured(
                "the server AI endpoint must be an absolute http(s) URL".to_string(),
            ));
        }
        Ok(raw.to_string())
    }

    /// Whether the server itself has everything it needs to serve AI requests.
    #[must_use]
    pub fn server_available(&self) -> bool {
        self.enabled
            && self.server_key().is_some()
            && self.server_model().is_some()
            && self.endpoint().is_ok()
    }

    #[must_use]
    pub fn timeout(&self) -> Duration {
        Duration::from_secs(self.timeout_seconds.clamp(1, 120))
    }

    #[must_use]
    pub fn input_limit(&self) -> usize {
        self.max_input_chars.clamp(1, 100_000)
    }

    #[must_use]
    pub fn output_limit(&self) -> usize {
        self.max_output_chars.clamp(1_024, 1_000_000)
    }

    #[must_use]
    pub fn item_limit(&self) -> usize {
        self.max_items.clamp(1, 50)
    }

    #[must_use]
    pub fn rate_limit(&self) -> u32 {
        self.rate_limit_per_minute.clamp(1, 1_000)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn defaults_are_safe_and_bounded() {
        let settings = AiSettings::default();
        assert!(!settings.enabled);
        assert!(settings.server_key().is_none());
        assert!(settings.server_model().is_none());
        assert_eq!(settings.timeout(), Duration::from_secs(30));
        assert_eq!(settings.input_limit(), 8_000);
        assert_eq!(settings.output_limit(), 64_000);
        assert_eq!(settings.item_limit(), 20);
        assert_eq!(settings.rate_limit(), 10);
    }

    #[test]
    fn values_are_clamped_to_safe_ranges() {
        let settings = AiSettings {
            timeout_seconds: 10_000,
            max_input_chars: 0,
            max_output_chars: 1,
            max_items: 1_000,
            rate_limit_per_minute: 0,
            ..AiSettings::default()
        };
        assert_eq!(settings.timeout(), Duration::from_secs(120));
        assert_eq!(settings.input_limit(), 1);
        assert_eq!(settings.output_limit(), 1_024);
        assert_eq!(settings.item_limit(), 50);
        assert_eq!(settings.rate_limit(), 1);
    }

    #[test]
    fn endpoint_rejects_relative_and_non_http_urls() {
        let mut settings = AiSettings {
            base_url: "not a url".to_string(),
            ..AiSettings::default()
        };
        assert!(settings.endpoint().is_err());

        settings.base_url = "ftp://example.com/api".to_string();
        assert!(settings.endpoint().is_err());

        settings.base_url = "https://openrouter.ai/api/v1/".to_string();
        assert_eq!(settings.endpoint().unwrap(), "https://openrouter.ai/api/v1");
    }

    #[test]
    fn server_availability_requires_enabled_key_model_and_endpoint() {
        let mut settings = AiSettings {
            enabled: true,
            api_key: "sk-test".to_string(),
            model: "provider/model".to_string(),
            ..AiSettings::default()
        };
        assert!(settings.server_available());

        settings.enabled = false;
        assert!(!settings.server_available());

        settings.enabled = true;
        settings.api_key.clear();
        assert!(!settings.server_available());
    }
}
