use loco_rs::prelude::*;
use serde::{Deserialize, Serialize};
use ts_rs::TS;
use uuid::Uuid;

pub use super::_entities::user_settings::{ActiveModel, Column, Entity, Model};
use crate::models::{
    items::{ItemPriority, ItemType},
    notification_deliveries::NotificationChannel,
};
pub type UserSettings = Entity;

/// The user's preferred colour scheme. `Auto` follows the operating system.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize, TS)]
#[serde(rename_all = "lowercase")]
#[ts(export)]
pub enum Theme {
    Light,
    Dark,
    Auto,
}

impl Theme {
    #[must_use]
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::Light => "light",
            Self::Dark => "dark",
            Self::Auto => "auto",
        }
    }

    #[must_use]
    pub fn parse(value: &str) -> Option<Self> {
        match value {
            "light" => Some(Self::Light),
            "dark" => Some(Self::Dark),
            "auto" => Some(Self::Auto),
            _ => None,
        }
    }
}

/// The defaults a newly created settings row gets.
pub const DEFAULT_TIMEZONE: &str = "UTC";
pub const DEFAULT_THEME: Theme = Theme::Auto;
pub const DEFAULT_ITEM_TYPE: ItemType = ItemType::Task;
pub const DEFAULT_PRIORITY: ItemPriority = ItemPriority::Normal;
pub const DEFAULT_WEEK_START: i64 = 1;
pub const DEFAULT_DIGEST_TIME: &str = "09:00";

/// A partial settings update.
#[derive(Clone, Debug, Default)]
pub struct SettingsPatch {
    pub timezone: Option<String>,
    pub theme: Option<Theme>,
    pub default_item_type: Option<ItemType>,
    pub default_priority: Option<ItemPriority>,
    pub week_start: Option<i64>,
    pub ai_enabled: Option<bool>,
    pub ai_use_server_key: Option<bool>,
    /// An empty string means "use the server default model".
    pub ai_model: Option<String>,
    /// `None` leaves the stored key alone; `Some(None)` removes it and
    /// `Some(Some(envelope))` replaces it with a newly sealed value. The
    /// plaintext key never reaches this type.
    pub ai_api_key_encrypted: Option<Option<String>>,
    pub notifications_enabled: Option<bool>,
    pub telegram_enabled: Option<bool>,
    pub telegram_chat_id: Option<String>,
    /// Same write-only contract as `ai_api_key_encrypted`.
    pub telegram_bot_token_encrypted: Option<Option<String>>,
    pub discord_enabled: Option<bool>,
    /// Same write-only contract as `ai_api_key_encrypted`.
    pub discord_webhook_encrypted: Option<Option<String>>,
    pub reminders_enabled: Option<bool>,
    pub daily_digest_enabled: Option<bool>,
    pub daily_digest_time: Option<String>,
    pub overdue_notifications_enabled: Option<bool>,
}

#[async_trait::async_trait]
impl ActiveModelBehavior for ActiveModel {
    async fn before_save<C>(self, _db: &C, insert: bool) -> std::result::Result<Self, DbErr>
    where
        C: ConnectionTrait,
    {
        if !insert && self.updated_at.is_unchanged() {
            let mut this = self;
            this.updated_at = sea_orm::ActiveValue::Set(chrono::Utc::now().into());
            Ok(this)
        } else {
            Ok(self)
        }
    }
}

impl Model {
    /// Find the settings row for a user, if it exists.
    ///
    /// # Errors
    ///
    /// Returns a database error when the query fails.
    pub async fn find_by_user(db: &DatabaseConnection, user_id: i64) -> ModelResult<Option<Self>> {
        Entity::find()
            .filter(Column::UserId.eq(user_id))
            .one(db)
            .await
            .map_err(ModelError::from)
    }

    /// Return the user's settings, creating a default row on first access.
    ///
    /// # Errors
    ///
    /// Returns a database error when the read or insert fails.
    pub async fn get_or_create(db: &DatabaseConnection, user_id: i64) -> ModelResult<Self> {
        if let Some(settings) = Self::find_by_user(db, user_id).await? {
            return Ok(settings);
        }

        let inserted = ActiveModel {
            pid: ActiveValue::set(Uuid::new_v4()),
            user_id: ActiveValue::set(user_id),
            timezone: ActiveValue::set(DEFAULT_TIMEZONE.to_string()),
            theme: ActiveValue::set(DEFAULT_THEME.as_str().to_string()),
            default_item_type: ActiveValue::set(DEFAULT_ITEM_TYPE.as_str().to_string()),
            default_priority: ActiveValue::set(DEFAULT_PRIORITY.as_str().to_string()),
            week_start: ActiveValue::set(DEFAULT_WEEK_START),
            ai_enabled: ActiveValue::set(true),
            ai_use_server_key: ActiveValue::set(true),
            ai_model: ActiveValue::set(String::new()),
            ai_api_key_encrypted: ActiveValue::set(None),
            notifications_enabled: ActiveValue::set(false),
            telegram_enabled: ActiveValue::set(false),
            telegram_chat_id: ActiveValue::set(String::new()),
            telegram_bot_token_encrypted: ActiveValue::set(None),
            discord_enabled: ActiveValue::set(false),
            discord_webhook_encrypted: ActiveValue::set(None),
            reminders_enabled: ActiveValue::set(false),
            daily_digest_enabled: ActiveValue::set(false),
            daily_digest_time: ActiveValue::set(DEFAULT_DIGEST_TIME.to_string()),
            overdue_notifications_enabled: ActiveValue::set(false),
            ..Default::default()
        }
        .insert(db)
        .await;

        match inserted {
            Ok(settings) => Ok(settings),
            // Two concurrent first requests can both miss the row; the unique
            // index lets one win. Re-read so both callers get a usable model.
            Err(error) => match Self::find_by_user(db, user_id).await? {
                Some(settings) => Ok(settings),
                None => Err(ModelError::from(error)),
            },
        }
    }

    /// Apply a partial update to the user's settings.
    ///
    /// # Errors
    ///
    /// Returns a database error when the update fails.
    pub async fn update_for_user(
        db: &DatabaseConnection,
        user_id: i64,
        patch: SettingsPatch,
    ) -> ModelResult<Self> {
        let settings = Self::get_or_create(db, user_id).await?;
        let mut active = settings.into_active_model();

        if let Some(timezone) = patch.timezone {
            active.timezone = ActiveValue::set(timezone);
        }
        if let Some(theme) = patch.theme {
            active.theme = ActiveValue::set(theme.as_str().to_string());
        }
        if let Some(item_type) = patch.default_item_type {
            active.default_item_type = ActiveValue::set(item_type.as_str().to_string());
        }
        if let Some(priority) = patch.default_priority {
            active.default_priority = ActiveValue::set(priority.as_str().to_string());
        }
        if let Some(week_start) = patch.week_start {
            active.week_start = ActiveValue::set(week_start);
        }
        if let Some(ai_enabled) = patch.ai_enabled {
            active.ai_enabled = ActiveValue::set(ai_enabled);
        }
        if let Some(ai_use_server_key) = patch.ai_use_server_key {
            active.ai_use_server_key = ActiveValue::set(ai_use_server_key);
        }
        if let Some(ai_model) = patch.ai_model {
            active.ai_model = ActiveValue::set(ai_model);
        }
        if let Some(ai_api_key_encrypted) = patch.ai_api_key_encrypted {
            active.ai_api_key_encrypted = ActiveValue::set(ai_api_key_encrypted);
        }
        if let Some(enabled) = patch.notifications_enabled {
            active.notifications_enabled = ActiveValue::set(enabled);
        }
        if let Some(enabled) = patch.telegram_enabled {
            active.telegram_enabled = ActiveValue::set(enabled);
        }
        if let Some(chat_id) = patch.telegram_chat_id {
            active.telegram_chat_id = ActiveValue::set(chat_id);
        }
        if let Some(token) = patch.telegram_bot_token_encrypted {
            active.telegram_bot_token_encrypted = ActiveValue::set(token);
        }
        if let Some(enabled) = patch.discord_enabled {
            active.discord_enabled = ActiveValue::set(enabled);
        }
        if let Some(webhook) = patch.discord_webhook_encrypted {
            active.discord_webhook_encrypted = ActiveValue::set(webhook);
        }
        if let Some(enabled) = patch.reminders_enabled {
            active.reminders_enabled = ActiveValue::set(enabled);
        }
        if let Some(enabled) = patch.daily_digest_enabled {
            active.daily_digest_enabled = ActiveValue::set(enabled);
        }
        if let Some(time) = patch.daily_digest_time {
            active.daily_digest_time = ActiveValue::set(time);
        }
        if let Some(enabled) = patch.overdue_notifications_enabled {
            active.overdue_notifications_enabled = ActiveValue::set(enabled);
        }

        active.update(db).await.map_err(ModelError::from)
    }

    /// Whether this account has a personal provider key stored (not whether it
    /// can be decrypted — that needs the server's encryption key).
    #[must_use]
    pub fn has_personal_key(&self) -> bool {
        self.ai_api_key_encrypted.is_some()
    }

    /// The timezone used for day boundaries, falling back to UTC.
    #[must_use]
    pub fn timezone(&self) -> chrono_tz::Tz {
        crate::time::timezone_or_utc(&self.timezone)
    }

    /// The parsed theme, falling back to `Auto`.
    #[must_use]
    pub fn theme(&self) -> Theme {
        Theme::parse(&self.theme).unwrap_or(DEFAULT_THEME)
    }

    /// The parsed default item type, falling back to `Task`.
    #[must_use]
    pub fn default_item_type(&self) -> ItemType {
        ItemType::parse(&self.default_item_type).unwrap_or(DEFAULT_ITEM_TYPE)
    }

    /// The parsed default priority, falling back to `Normal`.
    #[must_use]
    pub fn default_priority(&self) -> ItemPriority {
        ItemPriority::parse(&self.default_priority).unwrap_or(DEFAULT_PRIORITY)
    }

    /// Whether notifications are switched on for this account.
    #[must_use]
    pub fn notifications_enabled(&self) -> bool {
        self.notifications_enabled
    }

    /// Whether a Telegram bot token is stored. The token itself is never read
    /// here; decryption happens only in the notification dispatcher.
    #[must_use]
    pub fn telegram_configured(&self) -> bool {
        self.telegram_bot_token_encrypted.is_some() && !self.telegram_chat_id.trim().is_empty()
    }

    /// Whether a Discord webhook is stored.
    #[must_use]
    pub fn discord_configured(&self) -> bool {
        self.discord_webhook_encrypted.is_some()
    }

    /// Whether a provider can be used: enabled and fully configured.
    #[must_use]
    pub fn channel_ready(&self, channel: NotificationChannel) -> bool {
        match channel {
            NotificationChannel::Telegram => self.telegram_enabled && self.telegram_configured(),
            NotificationChannel::Discord => self.discord_enabled && self.discord_configured(),
        }
    }

    /// The providers that are enabled and configured, in a stable order.
    #[must_use]
    pub fn enabled_channels(&self) -> Vec<NotificationChannel> {
        [NotificationChannel::Telegram, NotificationChannel::Discord]
            .into_iter()
            .filter(|channel| self.channel_ready(*channel))
            .collect()
    }

    /// The local delivery time for the daily digest, falling back to 09:00.
    #[must_use]
    pub fn daily_digest_time(&self) -> chrono::NaiveTime {
        crate::time::parse_local_time(&self.daily_digest_time)
            .unwrap_or_else(|| crate::time::parse_local_time(DEFAULT_DIGEST_TIME).unwrap())
    }
}

impl Entity {}
