use loco_rs::schema::*;
use sea_orm_migration::prelude::*;

#[derive(DeriveMigrationName)]
pub struct Migration;

#[derive(DeriveIden)]
enum UserSettings {
    Table,
    UserId,
}

#[async_trait::async_trait]
impl MigrationTrait for Migration {
    async fn up(&self, m: &SchemaManager) -> Result<(), DbErr> {
        create_table(
            m,
            "user_settings",
            &[
                ("id", ColType::PkAuto),
                ("pid", ColType::UuidUniq),
                ("timezone", ColType::StringWithDefault("UTC".to_string())),
                ("theme", ColType::StringWithDefault("auto".to_string())),
                (
                    "default_item_type",
                    ColType::StringWithDefault("task".to_string()),
                ),
                (
                    "default_priority",
                    ColType::StringWithDefault("normal".to_string()),
                ),
                ("week_start", ColType::BigIntegerWithDefault(1)),
            ],
            &[("user", "")],
        )
        .await?;

        // One settings row per user. The unique index also serves the lookup by
        // user id, so no separate plain index is needed.
        m.create_index(
            Index::create()
                .name("idx_user_settings_user_id")
                .table(UserSettings::Table)
                .col(UserSettings::UserId)
                .unique()
                .to_owned(),
        )
        .await?;

        Ok(())
    }

    async fn down(&self, m: &SchemaManager) -> Result<(), DbErr> {
        drop_table(m, "user_settings").await
    }
}
