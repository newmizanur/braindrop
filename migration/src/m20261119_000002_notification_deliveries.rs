use loco_rs::schema::*;
use sea_orm_migration::prelude::*;

#[derive(DeriveMigrationName)]
pub struct Migration;

#[derive(DeriveIden)]
enum NotificationDeliveries {
    Table,
    UserId,
    ItemId,
    Channel,
    Status,
    NextAttemptAt,
    ScheduledAt,
    UpdatedAt,
}

#[async_trait::async_trait]
impl MigrationTrait for Migration {
    async fn up(&self, m: &SchemaManager) -> Result<(), DbErr> {
        // A durable record of every notification the app intends to send. The
        // unique deduplication key is what makes repeated scheduler scans safe:
        // the second insert conflicts instead of sending twice.
        create_table(
            m,
            "notification_deliveries",
            &[
                ("id", ColType::PkAuto),
                ("user_id", ColType::BigInteger),
                ("item_id", ColType::BigIntegerNull),
                ("channel", ColType::String),
                ("notification_type", ColType::String),
                ("deduplication_key", ColType::StringUniq),
                ("scheduled_at", ColType::TimestampWithTimeZone),
                ("status", ColType::StringWithDefault("pending".to_string())),
                ("attempts", ColType::BigIntegerWithDefault(0)),
                ("next_attempt_at", ColType::TimestampWithTimeZoneNull),
                ("sent_at", ColType::TimestampWithTimeZoneNull),
                ("last_error", ColType::TextNull),
            ],
            // `user` cascades; `item?` is nullable and becomes NULL when the
            // item is deleted so the delivery history survives.
            &[("user", ""), ("item?", "")],
        )
        .await?;

        // The worker polls due pending rows; the recovery scan looks for rows
        // stuck in `processing` by age. Both need their own index.
        for (name, cols) in [
            (
                "idx_notification_deliveries_status_next",
                vec![
                    NotificationDeliveries::Status,
                    NotificationDeliveries::NextAttemptAt,
                ],
            ),
            (
                "idx_notification_deliveries_status_updated",
                vec![
                    NotificationDeliveries::Status,
                    NotificationDeliveries::UpdatedAt,
                ],
            ),
            (
                "idx_notification_deliveries_user_channel",
                vec![
                    NotificationDeliveries::UserId,
                    NotificationDeliveries::Channel,
                ],
            ),
            (
                "idx_notification_deliveries_item",
                vec![NotificationDeliveries::ItemId],
            ),
            (
                "idx_notification_deliveries_scheduled",
                vec![NotificationDeliveries::ScheduledAt],
            ),
        ] {
            let mut index = Index::create();
            index.name(name).table(NotificationDeliveries::Table);
            for col in cols {
                index.col(col);
            }
            m.create_index(index.to_owned()).await?;
        }

        Ok(())
    }

    async fn down(&self, m: &SchemaManager) -> Result<(), DbErr> {
        drop_table(m, "notification_deliveries").await
    }
}
