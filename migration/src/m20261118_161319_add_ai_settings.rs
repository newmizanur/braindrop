use loco_rs::schema::*;
use sea_orm_migration::prelude::*;

#[derive(DeriveMigrationName)]
pub struct Migration;

#[async_trait::async_trait]
impl MigrationTrait for Migration {
    async fn up(&self, m: &SchemaManager) -> Result<(), DbErr> {
        // AI preferences live with the rest of the user's settings. The
        // personal API key is never stored in plain text: `ai_api_key_encrypted`
        // holds an AES-GCM envelope produced with the deployment's key.
        add_column(
            m,
            "user_settings",
            "ai_enabled",
            ColType::BooleanWithDefault(true),
        )
        .await?;
        add_column(
            m,
            "user_settings",
            "ai_use_server_key",
            ColType::BooleanWithDefault(true),
        )
        .await?;
        add_column(
            m,
            "user_settings",
            "ai_model",
            ColType::StringWithDefault(String::new()),
        )
        .await?;
        add_column(
            m,
            "user_settings",
            "ai_api_key_encrypted",
            ColType::TextNull,
        )
        .await?;

        Ok(())
    }

    async fn down(&self, m: &SchemaManager) -> Result<(), DbErr> {
        remove_column(m, "user_settings", "ai_api_key_encrypted").await?;
        remove_column(m, "user_settings", "ai_model").await?;
        remove_column(m, "user_settings", "ai_use_server_key").await?;
        remove_column(m, "user_settings", "ai_enabled").await?;
        Ok(())
    }
}
