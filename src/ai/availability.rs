//! Resolving which credential and model a request should use.
//!
//! The server key is preferred when the account asks for it, but a personal key
//! is a safe fallback when the server is not configured. Neither path ever
//! exposes the key outside this module.

use std::{fmt, time::Duration};

use serde::{Deserialize, Serialize};
use ts_rs::TS;

use crate::{
    ai::{config::AiSettings, credentials::SecretBox, error::AiError},
    models::user_settings,
};

/// Where the API key for a request came from.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize, TS)]
#[serde(rename_all = "lowercase")]
#[ts(export)]
pub enum AiCredentialSource {
    Server,
    Personal,
}

/// A request-ready AI configuration. `Debug` is redacted.
#[derive(Clone)]
pub struct ResolvedAi {
    pub api_key: String,
    pub model: String,
    pub base_url: String,
    pub timeout: Duration,
    pub max_output_chars: usize,
    pub max_items: usize,
    pub source: AiCredentialSource,
}

impl fmt::Debug for ResolvedAi {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("ResolvedAi")
            .field("api_key", &"<redacted>")
            .field("model", &self.model)
            .field("base_url", &self.base_url)
            .field("timeout", &self.timeout)
            .field("max_output_chars", &self.max_output_chars)
            .field("max_items", &self.max_items)
            .field("source", &self.source)
            .finish()
    }
}

/// The answer to "can this account use AI right now?".
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum AiAvailability {
    Available {
        model: String,
        source: AiCredentialSource,
    },
    Disabled(String),
    NotConfigured(String),
}

impl AiAvailability {
    #[must_use]
    pub fn is_available(&self) -> bool {
        matches!(self, Self::Available { .. })
    }

    #[must_use]
    pub fn reason(&self) -> Option<&str> {
        match self {
            Self::Available { .. } => None,
            Self::Disabled(reason) | Self::NotConfigured(reason) => Some(reason),
        }
    }

    #[must_use]
    pub fn model(&self) -> Option<&str> {
        match self {
            Self::Available { model, .. } => Some(model),
            Self::Disabled(_) | Self::NotConfigured(_) => None,
        }
    }

    #[must_use]
    pub fn source(&self) -> Option<AiCredentialSource> {
        match self {
            Self::Available { source, .. } => Some(*source),
            Self::Disabled(_) | Self::NotConfigured(_) => None,
        }
    }
}

/// Decide whether AI is usable and which model would be called.
#[must_use]
pub fn availability(config: &AiSettings, settings: &user_settings::Model) -> AiAvailability {
    if !config.enabled {
        return AiAvailability::Disabled("AI is disabled on this server.".to_string());
    }
    if !settings.ai_enabled {
        return AiAvailability::Disabled("AI is disabled for this account.".to_string());
    }

    let Some(model) = selected_model(config, settings) else {
        return AiAvailability::NotConfigured("No AI model is configured.".to_string());
    };
    if config.endpoint().is_err() {
        return AiAvailability::NotConfigured(
            "The server AI endpoint is not a valid URL.".to_string(),
        );
    }

    let has_personal_key = settings.ai_api_key_encrypted.is_some();
    if settings.ai_use_server_key && config.server_key().is_some() {
        return AiAvailability::Available {
            model,
            source: AiCredentialSource::Server,
        };
    }
    if has_personal_key {
        return personal_availability(config, settings, model);
    }
    if settings.ai_use_server_key {
        AiAvailability::NotConfigured(
            "Server AI credentials are not configured. Add a personal key or contact the administrator."
                .to_string(),
        )
    } else {
        AiAvailability::NotConfigured(
            "Add a personal OpenRouter key or switch to server credentials.".to_string(),
        )
    }
}

/// Build the request-ready configuration, or explain why it cannot be built.
///
/// # Errors
///
/// Returns `Disabled`/`NotConfigured` when no usable credential exists, or
/// `Encryption` when the stored personal key cannot be opened.
pub fn resolve(
    config: &AiSettings,
    settings: &user_settings::Model,
) -> Result<ResolvedAi, AiError> {
    let availability = availability(config, settings);
    let (model, source) = match availability {
        AiAvailability::Available { model, source } => (model, source),
        AiAvailability::Disabled(reason) => return Err(AiError::Disabled(reason)),
        AiAvailability::NotConfigured(reason) => return Err(AiError::NotConfigured(reason)),
    };

    let api_key = match source {
        AiCredentialSource::Server => config.server_key().ok_or_else(|| {
            AiError::NotConfigured("Server AI credentials are not configured.".to_string())
        })?,
        AiCredentialSource::Personal => {
            let envelope = settings.ai_api_key_encrypted.as_deref().ok_or_else(|| {
                AiError::NotConfigured("No personal OpenRouter key is stored.".to_string())
            })?;
            let secret = SecretBox::from_base64(&config.encryption_key)?.ok_or_else(|| {
                AiError::NotConfigured(
                    "Credential encryption is not configured on this server.".to_string(),
                )
            })?;
            return finish(config, secret.decrypt(envelope)?, model, source);
        }
    };

    finish(config, api_key.to_string(), model, source)
}

fn finish(
    config: &AiSettings,
    api_key: String,
    model: String,
    source: AiCredentialSource,
) -> Result<ResolvedAi, AiError> {
    let api_key = api_key.trim().to_string();
    if api_key.is_empty() {
        return Err(AiError::NotConfigured(
            "The selected AI credential is empty.".to_string(),
        ));
    }
    Ok(ResolvedAi {
        api_key,
        model,
        base_url: config.endpoint()?,
        timeout: config.timeout(),
        max_output_chars: config.output_limit(),
        max_items: config.item_limit(),
        source,
    })
}

fn selected_model(config: &AiSettings, settings: &user_settings::Model) -> Option<String> {
    let user = settings.ai_model.trim();
    if !user.is_empty() {
        return Some(user.to_string());
    }
    config.server_model().map(str::to_string)
}

fn personal_availability(
    config: &AiSettings,
    settings: &user_settings::Model,
    model: String,
) -> AiAvailability {
    if config.encryption_key.trim().is_empty() {
        return AiAvailability::NotConfigured(
            "Credential encryption is not configured on this server.".to_string(),
        );
    }
    let secret = match SecretBox::from_base64(&config.encryption_key) {
        Ok(Some(secret)) => secret,
        Ok(None) => {
            return AiAvailability::NotConfigured(
                "Credential encryption is not configured on this server.".to_string(),
            )
        }
        Err(_) => {
            return AiAvailability::NotConfigured(
                "Credential encryption is misconfigured on this server.".to_string(),
            )
        }
    };
    let envelope = settings.ai_api_key_encrypted.as_deref().unwrap_or_default();
    match secret.decrypt(envelope) {
        Ok(key) if !key.trim().is_empty() => AiAvailability::Available {
            model,
            source: AiCredentialSource::Personal,
        },
        Ok(_) => AiAvailability::NotConfigured(
            "The stored personal key is empty; replace it.".to_string(),
        ),
        Err(_) => AiAvailability::NotConfigured(
            "The stored personal key could not be decrypted; replace it.".to_string(),
        ),
    }
}

#[cfg(test)]
mod tests {
    use chrono::Utc;
    use uuid::Uuid;

    use super::*;

    fn settings(ai_enabled: bool, use_server_key: bool, key: Option<&str>) -> user_settings::Model {
        let now = Utc::now().fixed_offset();
        user_settings::Model {
            created_at: now,
            updated_at: now,
            id: 1,
            pid: Uuid::new_v4(),
            timezone: "UTC".to_string(),
            theme: "auto".to_string(),
            default_item_type: "task".to_string(),
            default_priority: "normal".to_string(),
            week_start: 1,
            user_id: 1,
            ai_enabled,
            ai_use_server_key: use_server_key,
            ai_model: String::new(),
            ai_api_key_encrypted: key.map(str::to_string),
            notifications_enabled: false,
            telegram_enabled: false,
            telegram_chat_id: String::new(),
            telegram_bot_token_encrypted: None,
            discord_enabled: false,
            discord_webhook_encrypted: None,
            reminders_enabled: false,
            daily_digest_enabled: false,
            daily_digest_time: "09:00".to_string(),
            overdue_notifications_enabled: false,
        }
    }

    fn config() -> AiSettings {
        AiSettings {
            enabled: true,
            api_key: "sk-server".to_string(),
            model: "server/model".to_string(),
            encryption_key: base64::Engine::encode(
                &base64::engine::general_purpose::STANDARD,
                [7u8; 32],
            ),
            ..AiSettings::default()
        }
    }

    #[test]
    fn server_key_is_used_when_available() {
        let settings = settings(true, true, None);
        let availability = availability(&config(), &settings);
        assert!(availability.is_available());
        assert_eq!(availability.model(), Some("server/model"));
        assert_eq!(availability.source(), Some(AiCredentialSource::Server));

        let resolved = resolve(&config(), &settings).unwrap();
        assert_eq!(resolved.api_key, "sk-server");
    }

    #[test]
    fn personal_key_falls_back_when_the_server_has_no_key() {
        let mut config = config();
        config.api_key.clear();
        let secret = SecretBox::from_base64(&config.encryption_key)
            .unwrap()
            .unwrap();
        let envelope = secret.encrypt("sk-personal").unwrap();
        let settings = settings(true, true, Some(&envelope));

        let resolved = resolve(&config, &settings).unwrap();
        assert_eq!(resolved.api_key, "sk-personal");
        assert_eq!(resolved.source, AiCredentialSource::Personal);
    }

    #[test]
    fn a_disabled_account_reports_a_reason() {
        let settings = settings(false, true, None);
        let availability = availability(&config(), &settings);
        assert!(!availability.is_available());
        assert!(availability.reason().unwrap().contains("account"));
    }

    #[test]
    fn a_user_model_overrides_the_server_model() {
        let mut settings = settings(true, true, None);
        settings.ai_model = "personal/model".to_string();
        let availability = availability(&config(), &settings);
        assert_eq!(availability.model(), Some("personal/model"));
    }

    #[test]
    fn an_undecryptable_personal_key_is_not_available() {
        let mut config = config();
        config.api_key.clear();
        let settings = settings(true, false, Some("v1:not-a-real-envelope"));
        let availability = availability(&config, &settings);
        assert!(!availability.is_available());
        assert!(availability.reason().unwrap().contains("decrypted"));
    }
}
