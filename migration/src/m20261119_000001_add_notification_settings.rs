use loco_rs::schema::*;
use sea_orm_migration::prelude::*;

#[derive(DeriveMigrationName)]
pub struct Migration;

#[async_trait::async_trait]
impl MigrationTrait for Migration {
    async fn up(&self, m: &SchemaManager) -> Result<(), DbErr> {
        // Notification preferences live with the rest of the user's settings.
        // Every switch defaults to off: notifications are opt-in.
        add_column(
            m,
            "user_settings",
            "notifications_enabled",
            ColType::BooleanWithDefault(false),
        )
        .await?;
        add_column(
            m,
            "user_settings",
            "telegram_enabled",
            ColType::BooleanWithDefault(false),
        )
        .await?;
        add_column(
            m,
            "user_settings",
            "telegram_chat_id",
            ColType::StringWithDefault(String::new()),
        )
        .await?;
        // The bot token is sealed with the deployment's encryption key. The
        // plaintext value never reaches the database or an API response.
        add_column(
            m,
            "user_settings",
            "telegram_bot_token_encrypted",
            ColType::TextNull,
        )
        .await?;
        add_column(
            m,
            "user_settings",
            "discord_enabled",
            ColType::BooleanWithDefault(false),
        )
        .await?;
        add_column(
            m,
            "user_settings",
            "discord_webhook_encrypted",
            ColType::TextNull,
        )
        .await?;
        add_column(
            m,
            "user_settings",
            "reminders_enabled",
            ColType::BooleanWithDefault(false),
        )
        .await?;
        add_column(
            m,
            "user_settings",
            "daily_digest_enabled",
            ColType::BooleanWithDefault(false),
        )
        .await?;
        add_column(
            m,
            "user_settings",
            "daily_digest_time",
            ColType::StringWithDefault("09:00".to_string()),
        )
        .await?;
        add_column(
            m,
            "user_settings",
            "overdue_notifications_enabled",
            ColType::BooleanWithDefault(false),
        )
        .await?;

        Ok(())
    }

    async fn down(&self, m: &SchemaManager) -> Result<(), DbErr> {
        remove_column(m, "user_settings", "overdue_notifications_enabled").await?;
        remove_column(m, "user_settings", "daily_digest_time").await?;
        remove_column(m, "user_settings", "daily_digest_enabled").await?;
        remove_column(m, "user_settings", "reminders_enabled").await?;
        remove_column(m, "user_settings", "discord_webhook_encrypted").await?;
        remove_column(m, "user_settings", "discord_enabled").await?;
        remove_column(m, "user_settings", "telegram_bot_token_encrypted").await?;
        remove_column(m, "user_settings", "telegram_chat_id").await?;
        remove_column(m, "user_settings", "telegram_enabled").await?;
        remove_column(m, "user_settings", "notifications_enabled").await?;
        Ok(())
    }
}
