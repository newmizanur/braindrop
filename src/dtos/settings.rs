use serde::{Deserialize, Serialize};
use ts_rs::TS;

use crate::{
    ai::{
        availability::{self, AiCredentialSource},
        config::AiSettings,
        credentials::SecretBox,
    },
    dtos::ai::AiStatus,
    models::{
        items::{ItemPriority, ItemType},
        user_settings::{self, SettingsPatch, Theme},
    },
    notifications::{config::NotificationSettings, validate as notification_validate},
};

/// The user's persisted preferences. This type deliberately has no field for
/// credentials, tokens or integration secrets: only whether a personal key is
/// stored and where the next request's credential would come from.
#[derive(Clone, Debug, Serialize, Deserialize, TS)]
#[ts(export, export_to = "../frontend/src/bindings/")]
pub struct SettingsResponse {
    pub timezone: String,
    pub theme: Theme,
    pub default_item_type: ItemType,
    pub default_priority: ItemPriority,
    #[ts(type = "number")]
    pub week_start: i64,
    pub ai: AiSettingsResponse,
    pub notifications: NotificationSettingsResponse,
}

/// The AI preferences and their resolved status. The status is computed
/// server-side so the browser never has to guess whether a key exists.
#[derive(Clone, Debug, Serialize, Deserialize, TS)]
#[ts(export, export_to = "../frontend/src/bindings/")]
pub struct AiSettingsResponse {
    /// The account-level switch.
    pub enabled: bool,
    /// Whether this account prefers the server key over a personal one.
    pub use_server_key: bool,
    /// The account's model override; empty means "use the server default".
    pub model: String,
    /// Whether a personal key is stored. The key itself is never returned.
    pub has_personal_key: bool,
    /// `available`, `disabled` or `not_configured`.
    pub status: AiStatus,
    /// A safe, human-readable explanation when AI is not available.
    pub status_reason: Option<String>,
    /// The model a request would use right now, if AI is available.
    pub active_model: Option<String>,
    /// `server` or `personal`, if AI is available.
    pub credential_source: Option<AiCredentialSource>,
    /// The server's default model, shown as a placeholder in the UI.
    pub server_model: Option<String>,
}

impl SettingsResponse {
    #[must_use]
    pub fn new(settings: &user_settings::Model, config: &AiSettings) -> Self {
        Self {
            timezone: settings.timezone.clone(),
            theme: settings.theme(),
            default_item_type: settings.default_item_type(),
            default_priority: settings.default_priority(),
            week_start: settings.week_start,
            ai: AiSettingsResponse::new(settings, config),
            notifications: NotificationSettingsResponse::new(settings),
        }
    }
}

/// Notification preferences and channel status. Credentials are represented by
/// booleans only; the stored token and webhook never cross this boundary.
#[derive(Clone, Debug, Serialize, Deserialize, TS)]
#[ts(export, export_to = "../frontend/src/bindings/")]
pub struct NotificationSettingsResponse {
    pub enabled: bool,
    pub telegram: TelegramSettingsResponse,
    pub discord: DiscordSettingsResponse,
    pub reminders_enabled: bool,
    pub daily_digest_enabled: bool,
    pub daily_digest_time: String,
    pub overdue_notifications_enabled: bool,
}

#[derive(Clone, Debug, Serialize, Deserialize, TS)]
#[ts(export, export_to = "../frontend/src/bindings/")]
pub struct TelegramSettingsResponse {
    pub enabled: bool,
    /// A token and a chat id are both stored.
    pub configured: bool,
    /// A token is stored. The token itself is never returned.
    pub token_set: bool,
    /// The destination chat. Not a secret, so it is safe to show and edit.
    pub chat_id: String,
}

#[derive(Clone, Debug, Serialize, Deserialize, TS)]
#[ts(export, export_to = "../frontend/src/bindings/")]
pub struct DiscordSettingsResponse {
    pub enabled: bool,
    /// A webhook is stored.
    pub configured: bool,
    /// A webhook is stored. The URL itself is never returned.
    pub webhook_set: bool,
}

impl NotificationSettingsResponse {
    #[must_use]
    pub fn new(settings: &user_settings::Model) -> Self {
        Self {
            enabled: settings.notifications_enabled(),
            telegram: TelegramSettingsResponse {
                enabled: settings.telegram_enabled,
                configured: settings.telegram_configured(),
                token_set: settings.telegram_bot_token_encrypted.is_some(),
                chat_id: settings.telegram_chat_id.clone(),
            },
            discord: DiscordSettingsResponse {
                enabled: settings.discord_enabled,
                configured: settings.discord_configured(),
                webhook_set: settings.discord_webhook_encrypted.is_some(),
            },
            reminders_enabled: settings.reminders_enabled,
            daily_digest_enabled: settings.daily_digest_enabled,
            daily_digest_time: settings.daily_digest_time.clone(),
            overdue_notifications_enabled: settings.overdue_notifications_enabled,
        }
    }
}

impl AiSettingsResponse {
    #[must_use]
    pub fn new(settings: &user_settings::Model, config: &AiSettings) -> Self {
        let availability = availability::availability(config, settings);
        let status = match &availability {
            availability::AiAvailability::Available { .. } => AiStatus::Available,
            availability::AiAvailability::Disabled(_) => AiStatus::Disabled,
            availability::AiAvailability::NotConfigured(_) => AiStatus::NotConfigured,
        };

        Self {
            enabled: settings.ai_enabled,
            use_server_key: settings.ai_use_server_key,
            model: settings.ai_model.clone(),
            has_personal_key: settings.has_personal_key(),
            status,
            status_reason: availability.reason().map(str::to_string),
            active_model: availability.model().map(str::to_string),
            credential_source: availability.source(),
            server_model: config.server_model().map(str::to_string),
        }
    }
}

/// A partial settings update.
///
/// Deliberately does not derive `Debug`: it carries write-only credentials.
#[derive(Clone, Default, Deserialize, Serialize, TS)]
#[ts(export, export_to = "../frontend/src/bindings/")]
pub struct UpdateSettingsParams {
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub timezone: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub theme: Option<Theme>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub default_item_type: Option<ItemType>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub default_priority: Option<ItemPriority>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[ts(type = "number | null")]
    pub week_start: Option<i64>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub ai_enabled: Option<bool>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub ai_use_server_key: Option<bool>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub ai_model: Option<String>,
    /// Write-only. A non-empty value replaces the stored key; an empty string
    /// removes it. The value is never echoed back.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub ai_api_key: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub notifications_enabled: Option<bool>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub telegram_enabled: Option<bool>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub telegram_chat_id: Option<String>,
    /// Write-only, like `ai_api_key`: a non-empty value replaces the token and
    /// an empty string removes it.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub telegram_bot_token: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub discord_enabled: Option<bool>,
    /// Write-only, like `ai_api_key`: a non-empty value replaces the webhook
    /// and an empty string removes it.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub discord_webhook: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub reminders_enabled: Option<bool>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub daily_digest_enabled: Option<bool>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub daily_digest_time: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub overdue_notifications_enabled: Option<bool>,
}

const MAX_MODEL_CHARS: usize = 200;
const MAX_API_KEY_CHARS: usize = 512;
const MAX_TELEGRAM_CHAT_CHARS: usize = 64;

impl UpdateSettingsParams {
    /// Validate and convert to the model's patch type.
    ///
    /// # Errors
    ///
    /// Returns a safe message for an unknown timezone, an out-of-range week
    /// start, an invalid model name, or an unusable personal key.
    pub fn into_patch(
        self,
        ai: &AiSettings,
        notifications: &NotificationSettings,
    ) -> Result<SettingsPatch, String> {
        let timezone = match self.timezone {
            Some(timezone) => {
                let timezone = timezone.trim().to_string();
                if crate::time::parse_timezone(&timezone).is_none() {
                    return Err(format!("unknown timezone: {timezone}"));
                }
                Some(timezone)
            }
            None => None,
        };
        if let Some(week_start) = self.week_start {
            if !(0..=6).contains(&week_start) {
                return Err("week_start must be between 0 and 6".to_string());
            }
        }

        let ai_model = match self.ai_model {
            Some(model) => Some(validate_model(&model)?),
            None => None,
        };

        let ai_api_key_encrypted = match self.ai_api_key {
            Some(key) => Some(seal_key(&key, ai)?),
            None => None,
        };

        let telegram_chat_id = match self.telegram_chat_id {
            Some(value) => {
                let value = value.trim().to_string();
                if value.is_empty() {
                    Some(String::new())
                } else {
                    if value.chars().count() > MAX_TELEGRAM_CHAT_CHARS {
                        return Err(format!(
                            "telegram_chat_id must be at most {MAX_TELEGRAM_CHAT_CHARS} characters"
                        ));
                    }
                    notification_validate::telegram_chat_id(&value)
                        .map_err(|error| error.to_string())?;
                    Some(value)
                }
            }
            None => None,
        };

        let telegram_bot_token_encrypted = match self.telegram_bot_token {
            Some(token) => Some(seal_notification_secret(
                &token,
                ai,
                notifications,
                SecretKind::Telegram,
            )?),
            None => None,
        };
        let discord_webhook_encrypted = match self.discord_webhook {
            Some(webhook) => Some(seal_notification_secret(
                &webhook,
                ai,
                notifications,
                SecretKind::Discord,
            )?),
            None => None,
        };

        let daily_digest_time = match self.daily_digest_time {
            Some(value) => {
                let parsed = crate::time::parse_local_time(&value).ok_or_else(|| {
                    "daily_digest_time must be a local time such as 09:00".to_string()
                })?;
                Some(parsed.format("%H:%M").to_string())
            }
            None => None,
        };

        Ok(SettingsPatch {
            timezone,
            theme: self.theme,
            default_item_type: self.default_item_type,
            default_priority: self.default_priority,
            week_start: self.week_start,
            ai_enabled: self.ai_enabled,
            ai_use_server_key: self.ai_use_server_key,
            ai_model,
            ai_api_key_encrypted,
            notifications_enabled: self.notifications_enabled,
            telegram_enabled: self.telegram_enabled,
            telegram_chat_id,
            telegram_bot_token_encrypted,
            discord_enabled: self.discord_enabled,
            discord_webhook_encrypted,
            reminders_enabled: self.reminders_enabled,
            daily_digest_enabled: self.daily_digest_enabled,
            daily_digest_time,
            overdue_notifications_enabled: self.overdue_notifications_enabled,
        })
    }
}

enum SecretKind {
    Telegram,
    Discord,
}

/// Validate and seal a channel credential, or clear it when the submitted value
/// is empty. The plaintext never leaves this function.
fn seal_notification_secret(
    value: &str,
    ai: &AiSettings,
    config: &NotificationSettings,
    kind: SecretKind,
) -> Result<Option<String>, String> {
    let value = value.trim();
    if value.is_empty() {
        return Ok(None);
    }
    match kind {
        SecretKind::Telegram => {
            notification_validate::telegram_bot_token(value).map_err(|error| error.to_string())?;
        }
        SecretKind::Discord => {
            notification_validate::discord_webhook(value).map_err(|error| error.to_string())?;
        }
    }

    let secret = config
        .secret_box(ai)
        .map_err(|error| error.to_string())?
        .ok_or_else(|| {
            "notifications are unavailable: the server has no encryption key".to_string()
        })?;
    secret.encrypt(value).map(Some).map_err(|e| e.to_string())
}

fn validate_model(model: &str) -> Result<String, String> {
    let model = model.trim().to_string();
    if model.chars().count() > MAX_MODEL_CHARS {
        return Err(format!(
            "ai_model must be at most {MAX_MODEL_CHARS} characters"
        ));
    }
    if model.chars().any(char::is_control) {
        return Err("ai_model contains invalid characters".to_string());
    }
    Ok(model)
}

/// Seal a personal key, or clear it when the submitted value is empty.
fn seal_key(key: &str, config: &AiSettings) -> Result<Option<String>, String> {
    let key = key.trim();
    if key.is_empty() {
        return Ok(None);
    }
    if key.chars().count() > MAX_API_KEY_CHARS {
        return Err(format!(
            "ai_api_key must be at most {MAX_API_KEY_CHARS} characters"
        ));
    }
    if key.chars().any(char::is_whitespace) {
        return Err("ai_api_key must not contain whitespace".to_string());
    }

    let secret = SecretBox::from_base64(&config.encryption_key)
        .map_err(|error| error.to_string())?
        .ok_or_else(|| {
            "personal API keys are unavailable: the server has no AI_ENCRYPTION_KEY".to_string()
        })?;
    secret.encrypt(key).map(Some).map_err(|e| e.to_string())
}
