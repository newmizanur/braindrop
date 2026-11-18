use loco_rs::prelude::*;
use serde::{Deserialize, Serialize};
use ts_rs::TS;
use uuid::Uuid;

pub use super::_entities::user_settings::{ActiveModel, Column, Entity, Model};
use crate::models::items::{ItemPriority, ItemType};
pub type UserSettings = Entity;

/// The user's preferred colour scheme. `Auto` follows the operating system.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize, TS)]
#[serde(rename_all = "lowercase")]
#[ts(export, export_to = "../frontend/src/bindings/")]
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

/// A partial settings update.
#[derive(Clone, Debug, Default)]
pub struct SettingsPatch {
    pub timezone: Option<String>,
    pub theme: Option<Theme>,
    pub default_item_type: Option<ItemType>,
    pub default_priority: Option<ItemPriority>,
    pub week_start: Option<i64>,
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

        active.update(db).await.map_err(ModelError::from)
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
}

impl Entity {}
