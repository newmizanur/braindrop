use serde::{Deserialize, Serialize};
use ts_rs::TS;

use crate::models::{
    items::{ItemPriority, ItemType},
    user_settings::{self, SettingsPatch, Theme},
};

/// The user's persisted preferences. This type deliberately has no field for
/// credentials, tokens or integration secrets.
#[derive(Clone, Debug, Serialize, Deserialize, TS)]
#[ts(export, export_to = "../frontend/src/bindings/")]
pub struct SettingsResponse {
    pub timezone: String,
    pub theme: Theme,
    pub default_item_type: ItemType,
    pub default_priority: ItemPriority,
    #[ts(type = "number")]
    pub week_start: i64,
}

impl From<&user_settings::Model> for SettingsResponse {
    fn from(settings: &user_settings::Model) -> Self {
        Self {
            timezone: settings.timezone.clone(),
            theme: settings.theme(),
            default_item_type: settings.default_item_type(),
            default_priority: settings.default_priority(),
            week_start: settings.week_start,
        }
    }
}

impl From<user_settings::Model> for SettingsResponse {
    fn from(settings: user_settings::Model) -> Self {
        Self::from(&settings)
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
}

impl UpdateSettingsParams {
    /// Validate and convert to the model's patch type.
    ///
    /// # Errors
    ///
    /// Returns a message for an unknown timezone or an out-of-range week start.
    pub fn into_patch(self) -> Result<SettingsPatch, String> {
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

        Ok(SettingsPatch {
            timezone,
            theme: self.theme,
            default_item_type: self.default_item_type,
            default_priority: self.default_priority,
            week_start: self.week_start,
        })
    }
}
