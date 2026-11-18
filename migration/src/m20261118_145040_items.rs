use loco_rs::schema::*;
use sea_orm_migration::prelude::*;

#[derive(DeriveMigrationName)]
pub struct Migration;

#[derive(DeriveIden)]
enum Items {
    Table,
    UserId,
    ItemType,
    Status,
    Priority,
    Category,
    DueAt,
    RemindAt,
    CreatedAt,
    CompletedAt,
}

#[async_trait::async_trait]
impl MigrationTrait for Migration {
    async fn up(&self, m: &SchemaManager) -> Result<(), DbErr> {
        create_table(
            m,
            "items",
            &[
                ("id", ColType::PkAuto),
                ("pid", ColType::UuidUniq),
                ("item_type", ColType::String),
                ("title", ColType::String),
                ("content", ColType::TextNull),
                ("category", ColType::StringNull),
                ("tags", ColType::JsonNull),
                ("status", ColType::String),
                ("priority", ColType::String),
                ("due_at", ColType::TimestampWithTimeZoneNull),
                ("remind_at", ColType::TimestampWithTimeZoneNull),
                ("completed_at", ColType::TimestampWithTimeZoneNull),
                ("archived_at", ColType::TimestampWithTimeZoneNull),
            ],
            &[("user", "")],
        )
        .await?;

        // Every query is scoped to the owner; the composite indexes keep the
        // common list/filter/sort shapes from scanning the whole table.
        for (name, cols) in [
            ("idx_items_user_id", vec![Items::UserId]),
            ("idx_items_user_status", vec![Items::UserId, Items::Status]),
            ("idx_items_user_due_at", vec![Items::UserId, Items::DueAt]),
            (
                "idx_items_user_remind_at",
                vec![Items::UserId, Items::RemindAt],
            ),
            (
                "idx_items_user_created_at",
                vec![Items::UserId, Items::CreatedAt],
            ),
            (
                "idx_items_user_completed_at",
                vec![Items::UserId, Items::CompletedAt],
            ),
            ("idx_items_user_type", vec![Items::UserId, Items::ItemType]),
            (
                "idx_items_user_priority",
                vec![Items::UserId, Items::Priority],
            ),
            (
                "idx_items_user_category",
                vec![Items::UserId, Items::Category],
            ),
        ] {
            let mut index = Index::create();
            index.name(name).table(Items::Table);
            for col in cols {
                index.col(col);
            }
            m.create_index(index.to_owned()).await?;
        }

        Ok(())
    }

    async fn down(&self, m: &SchemaManager) -> Result<(), DbErr> {
        drop_table(m, "items").await
    }
}
