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
#[derive(Clone, Debug, Default, Deserialize, Serialize, TS)]
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
}

const MAX_MODEL_CHARS: usize = 200;
const MAX_API_KEY_CHARS: usize = 512;

impl UpdateSettingsParams {
    /// Validate and convert to the model's patch type.
    ///
    /// # Errors
    ///
    /// Returns a safe message for an unknown timezone, an out-of-range week
    /// start, an invalid model name, or an unusable personal key.
    pub fn into_patch(self, config: &AiSettings) -> Result<SettingsPatch, String> {
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
            Some(key) => Some(seal_key(&key, config)?),
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
        })
    }
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
