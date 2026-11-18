//! Typed access to the `settings.notifications` configuration block.
//!
//! These are server-wide limits, not user preferences. Users may enable a
//! channel and store credentials, but they can never point the app at an
//! arbitrary host: Telegram always talks to the configured official API and
//! Discord webhooks are validated against the official hosts.

use std::time::Duration;

use loco_rs::{config::Config, Error};
use serde::{Deserialize, Deserializer};

use crate::{
    ai::{config::AiSettings, credentials::SecretBox},
    notifications::error::NotificationError,
};

fn string_or_empty<'de, D>(deserializer: D) -> Result<String, D::Error>
where
    D: Deserializer<'de>,
{
    Ok(Option::<String>::deserialize(deserializer)?.unwrap_or_default())
}

/// The notification settings read from `config/<env>.yaml`.
///
/// Deliberately does not derive `Debug`: it holds the credential encryption key.
#[derive(Clone, Deserialize)]
#[serde(default)]
pub struct NotificationSettings {
    /// Base64-encoded 32-byte key used to seal channel credentials. When empty
    /// the AI encryption key is used, so a deployment only needs one key.
    #[serde(deserialize_with = "string_or_empty")]
    pub encryption_key: String,
    /// Official Telegram Bot API root. Never user-configurable.
    #[serde(deserialize_with = "string_or_empty")]
    pub telegram_api_base: String,
    /// Official Discord API root. Only used to describe the destination; the
    /// webhook URL itself is validated separately.
    #[serde(deserialize_with = "string_or_empty")]
    pub discord_api_base: String,
    pub timeout_seconds: u64,
    pub max_attempts: u32,
    pub retry_base_seconds: u64,
    pub retry_max_seconds: u64,
    /// Deliveries left in `processing` longer than this are assumed abandoned.
    pub stale_processing_minutes: i64,
    /// How long finished delivery history is kept.
    pub history_retention_days: i64,
    pub test_rate_limit_per_minute: u32,
    /// How far in the past a reminder may be and still be sent after downtime.
    pub reminder_grace_hours: i64,
    /// The digest is only sent within this many minutes after its local time.
    pub digest_window_minutes: i64,
    /// Maximum number of items listed in a digest or overdue message.
    pub max_message_items: usize,
    pub telegram_max_chars: usize,
    pub discord_max_chars: usize,
    /// Maximum deliveries one recovery pass enqueues.
    pub recovery_batch: u64,
}

impl Default for NotificationSettings {
    fn default() -> Self {
        Self {
            encryption_key: String::new(),
            telegram_api_base: "https://api.telegram.org".to_string(),
            discord_api_base: "https://discord.com".to_string(),
            timeout_seconds: 10,
            max_attempts: 5,
            retry_base_seconds: 30,
            retry_max_seconds: 3_600,
            stale_processing_minutes: 15,
            history_retention_days: 30,
            test_rate_limit_per_minute: 5,
            reminder_grace_hours: 24,
            digest_window_minutes: 120,
            max_message_items: 10,
            telegram_max_chars: 4_096,
            discord_max_chars: 2_000,
            recovery_batch: 100,
        }
    }
}

#[derive(Clone, Default, Deserialize)]
#[serde(default)]
struct NotificationConfigRoot {
    notifications: NotificationSettings,
}

impl NotificationSettings {
    /// Read the notification block from the application configuration.
    ///
    /// # Errors
    ///
    /// Returns an error when the `settings:` block is present but malformed.
    pub fn from_config(config: &Config) -> Result<Self, Error> {
        let root: NotificationConfigRoot = config.settings()?;
        Ok(root.notifications)
    }

    /// Build the credential cipher, falling back to the AI encryption key.
    ///
    /// # Errors
    ///
    /// Returns an error when the configured key is not valid base64 or is not
    /// exactly 32 bytes.
    pub fn secret_box(&self, ai: &AiSettings) -> Result<Option<SecretBox>, NotificationError> {
        let key = if self.encryption_key.trim().is_empty() {
            ai.encryption_key.trim()
        } else {
            self.encryption_key.trim()
        };
        SecretBox::from_base64(key)
            .map_err(|error| NotificationError::Encryption(error.to_string()))
    }

    #[must_use]
    pub fn timeout(&self) -> Duration {
        Duration::from_secs(self.timeout_seconds.clamp(1, 60))
    }

    #[must_use]
    pub fn max_attempts(&self) -> i64 {
        i64::from(self.max_attempts.clamp(1, 10))
    }

    #[must_use]
    pub fn retry_base(&self) -> Duration {
        Duration::from_secs(self.retry_base_seconds.clamp(1, 3_600))
    }

    #[must_use]
    pub fn retry_max(&self) -> Duration {
        Duration::from_secs(self.retry_max_seconds.clamp(1, 86_400))
    }

    #[must_use]
    pub fn stale_processing(&self) -> Duration {
        Duration::from_secs(
            u64::try_from(self.stale_processing_minutes.clamp(1, 1_440)).unwrap_or(900) * 60,
        )
    }

    #[must_use]
    pub fn retention_days(&self) -> i64 {
        self.history_retention_days.clamp(1, 3_650)
    }

    #[must_use]
    pub fn test_rate_limit(&self) -> u32 {
        self.test_rate_limit_per_minute.clamp(1, 60)
    }

    #[must_use]
    pub fn reminder_grace(&self) -> chrono::Duration {
        chrono::Duration::hours(self.reminder_grace_hours.clamp(1, 24 * 30))
    }

    #[must_use]
    pub fn digest_window(&self) -> chrono::Duration {
        chrono::Duration::minutes(self.digest_window_minutes.clamp(1, 24 * 60))
    }

    #[must_use]
    pub fn message_items(&self) -> usize {
        self.max_message_items.clamp(1, 50)
    }

    #[must_use]
    pub fn telegram_limit(&self) -> usize {
        self.telegram_max_chars.clamp(64, 4_096)
    }

    #[must_use]
    pub fn discord_limit(&self) -> usize {
        self.discord_max_chars.clamp(64, 2_000)
    }

    #[must_use]
    pub fn recovery_batch(&self) -> u64 {
        self.recovery_batch.clamp(1, 1_000)
    }

    /// The official Telegram API root, without a trailing slash.
    #[must_use]
    pub fn telegram_base(&self) -> &str {
        self.telegram_api_base.trim().trim_end_matches('/')
    }

    /// The official Discord API root, without a trailing slash.
    #[must_use]
    pub fn discord_base(&self) -> &str {
        self.discord_api_base.trim().trim_end_matches('/')
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn defaults_are_safe_and_bounded() {
        let settings = NotificationSettings::default();
        assert_eq!(settings.timeout(), Duration::from_secs(10));
        assert_eq!(settings.max_attempts(), 5);
        assert_eq!(settings.retry_base(), Duration::from_secs(30));
        assert_eq!(settings.retry_max(), Duration::from_secs(3_600));
        assert_eq!(settings.retention_days(), 30);
        assert_eq!(settings.test_rate_limit(), 5);
        assert_eq!(settings.telegram_limit(), 4_096);
        assert_eq!(settings.discord_limit(), 2_000);
        assert_eq!(settings.telegram_base(), "https://api.telegram.org");
    }

    #[test]
    fn values_are_clamped_to_safe_ranges() {
        let settings = NotificationSettings {
            timeout_seconds: 0,
            max_attempts: 0,
            retry_base_seconds: 0,
            retry_max_seconds: 0,
            history_retention_days: 0,
            test_rate_limit_per_minute: 0,
            telegram_max_chars: 0,
            discord_max_chars: 0,
            ..NotificationSettings::default()
        };
        assert_eq!(settings.timeout(), Duration::from_secs(1));
        assert_eq!(settings.max_attempts(), 1);
        assert_eq!(settings.retry_base(), Duration::from_secs(1));
        assert_eq!(settings.retry_max(), Duration::from_secs(1));
        assert_eq!(settings.retention_days(), 1);
        assert_eq!(settings.test_rate_limit(), 1);
        assert_eq!(settings.telegram_limit(), 64);
        assert_eq!(settings.discord_limit(), 64);
    }
}
