//! Durable notification delivery records.
//!
//! Every notification the app intends to send is written here *before* a job is
//! enqueued. The unique `deduplication_key` is the real guard against duplicate
//! sends: repeated scheduler scans and restart recovery all insert through it,
//! and only the first insert wins. Jobs are at-least-once, so external delivery
//! itself cannot be exactly-once after an ambiguous provider failure.

use chrono::{DateTime, NaiveDate, Utc};
use loco_rs::prelude::*;
use sea_orm::sea_query::Expr;
use serde::{Deserialize, Serialize};
use ts_rs::TS;
use uuid::Uuid;

pub use super::_entities::notification_deliveries::{ActiveModel, Column, Entity, Model, Relation};
pub type NotificationDeliveries = Entity;

#[async_trait::async_trait]
impl ActiveModelBehavior for ActiveModel {}

/// The provider a delivery goes through.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize, TS)]
#[serde(rename_all = "lowercase")]
#[ts(export)]
pub enum NotificationChannel {
    Telegram,
    Discord,
}

impl NotificationChannel {
    #[must_use]
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::Telegram => "telegram",
            Self::Discord => "discord",
        }
    }

    #[must_use]
    pub fn parse(value: &str) -> Option<Self> {
        match value {
            "telegram" => Some(Self::Telegram),
            "discord" => Some(Self::Discord),
            _ => None,
        }
    }
}

/// Why a delivery exists.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize, TS)]
#[serde(rename_all = "snake_case")]
#[ts(export)]
pub enum NotificationType {
    Reminder,
    DailyDigest,
    Overdue,
}

impl NotificationType {
    #[must_use]
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::Reminder => "reminder",
            Self::DailyDigest => "daily_digest",
            Self::Overdue => "overdue",
        }
    }

    #[must_use]
    pub fn parse(value: &str) -> Option<Self> {
        match value {
            "reminder" => Some(Self::Reminder),
            "daily_digest" => Some(Self::DailyDigest),
            "overdue" => Some(Self::Overdue),
            _ => None,
        }
    }
}

/// The lifecycle of one delivery.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize, TS)]
#[serde(rename_all = "lowercase")]
#[ts(export)]
pub enum DeliveryStatus {
    Pending,
    Processing,
    Sent,
    Failed,
    Cancelled,
}

impl DeliveryStatus {
    #[must_use]
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::Pending => "pending",
            Self::Processing => "processing",
            Self::Sent => "sent",
            Self::Failed => "failed",
            Self::Cancelled => "cancelled",
        }
    }

    #[must_use]
    pub fn parse(value: &str) -> Option<Self> {
        match value {
            "pending" => Some(Self::Pending),
            "processing" => Some(Self::Processing),
            "sent" => Some(Self::Sent),
            "failed" => Some(Self::Failed),
            "cancelled" => Some(Self::Cancelled),
            _ => None,
        }
    }
}

/// A new delivery to persist.
#[derive(Clone, Debug)]
pub struct NewDelivery {
    pub user_id: i64,
    pub item_id: Option<i64>,
    pub channel: NotificationChannel,
    pub notification_type: NotificationType,
    pub deduplication_key: String,
    pub scheduled_at: DateTime<Utc>,
}

/// Build the deduplication key for a reminder. The reminder timestamp is part
/// of the key, so editing `remind_at` creates a new delivery and leaves the old
/// one to be cancelled by the worker.
#[must_use]
pub fn reminder_key(
    item_pid: Uuid,
    remind_at: DateTime<Utc>,
    channel: NotificationChannel,
) -> String {
    format!(
        "reminder:{}:{}:{}",
        item_pid,
        remind_at.timestamp(),
        channel.as_str()
    )
}

/// One digest per user, local calendar day and provider.
#[must_use]
pub fn digest_key(user_id: i64, date: NaiveDate, channel: NotificationChannel) -> String {
    format!("digest:{}:{}:{}", user_id, date, channel.as_str())
}

/// One grouped overdue notification per user, local calendar day and provider.
#[must_use]
pub fn overdue_key(user_id: i64, date: NaiveDate, channel: NotificationChannel) -> String {
    format!("overdue:{}:{}:{}", user_id, date, channel.as_str())
}

impl Model {
    /// Persist a delivery unless one with the same deduplication key already
    /// exists. Returns the new row's id when this call won the insert.
    ///
    /// # Errors
    ///
    /// Returns a database error when the insert fails for a reason other than a
    /// duplicate key.
    pub async fn create(db: &DatabaseConnection, new: NewDelivery) -> ModelResult<Option<i64>> {
        let active = ActiveModel {
            user_id: ActiveValue::set(new.user_id),
            item_id: ActiveValue::set(new.item_id),
            channel: ActiveValue::set(new.channel.as_str().to_string()),
            notification_type: ActiveValue::set(new.notification_type.as_str().to_string()),
            deduplication_key: ActiveValue::set(new.deduplication_key),
            scheduled_at: ActiveValue::set(new.scheduled_at.into()),
            status: ActiveValue::set(DeliveryStatus::Pending.as_str().to_string()),
            attempts: ActiveValue::set(0),
            ..Default::default()
        };

        match Entity::insert(active)
            .on_conflict_do_nothing_on([Column::DeduplicationKey])
            .exec(db)
            .await?
        {
            sea_orm::TryInsertResult::Inserted(result) => Ok(Some(result.last_insert_id)),
            sea_orm::TryInsertResult::Conflicted | sea_orm::TryInsertResult::Empty => Ok(None),
        }
    }

    /// Atomically move a pending delivery to `processing` and count the attempt.
    /// Returns `false` when another worker already claimed it, when it is no
    /// longer pending, or when a retry backoff has not expired yet.
    ///
    /// # Errors
    ///
    /// Returns a database error when the conditional update fails.
    pub async fn claim(db: &DatabaseConnection, id: i64, now: DateTime<Utc>) -> ModelResult<bool> {
        let result = Entity::update_many()
            .col_expr(
                Column::Status,
                Expr::value(DeliveryStatus::Processing.as_str()),
            )
            .col_expr(Column::Attempts, Expr::cust("attempts + 1"))
            .col_expr(Column::UpdatedAt, Expr::value(now))
            .filter(Column::Id.eq(id))
            .filter(Column::Status.eq(DeliveryStatus::Pending.as_str()))
            // A duplicate queue job must not bypass a backoff scheduled by an
            // earlier attempt. The row is only claimable once it is due again.
            .filter(
                Condition::any()
                    .add(Column::NextAttemptAt.is_null())
                    .add(Column::NextAttemptAt.lte(now)),
            )
            .exec(db)
            .await?;
        Ok(result.rows_affected == 1)
    }

    /// Mark a delivery as delivered.
    ///
    /// # Errors
    ///
    /// Returns a database error when the update fails.
    pub async fn mark_sent(
        db: &DatabaseConnection,
        id: i64,
        now: DateTime<Utc>,
    ) -> ModelResult<()> {
        Entity::update_many()
            .col_expr(Column::Status, Expr::value(DeliveryStatus::Sent.as_str()))
            .col_expr(Column::SentAt, Expr::value(Some(now)))
            .col_expr(
                Column::NextAttemptAt,
                Expr::value(Option::<DateTime<Utc>>::None),
            )
            .col_expr(Column::LastError, Expr::value(Option::<String>::None))
            .col_expr(Column::UpdatedAt, Expr::value(now))
            .filter(Column::Id.eq(id))
            .exec(db)
            .await?;
        Ok(())
    }

    /// Put a delivery back to `pending` with a later attempt time.
    ///
    /// # Errors
    ///
    /// Returns a database error when the update fails.
    pub async fn mark_retry(
        db: &DatabaseConnection,
        id: i64,
        next_attempt_at: DateTime<Utc>,
        error: &str,
    ) -> ModelResult<()> {
        Entity::update_many()
            .col_expr(
                Column::Status,
                Expr::value(DeliveryStatus::Pending.as_str()),
            )
            .col_expr(Column::NextAttemptAt, Expr::value(Some(next_attempt_at)))
            .col_expr(Column::LastError, Expr::value(Some(error.to_string())))
            .col_expr(Column::UpdatedAt, Expr::value(Utc::now()))
            .filter(Column::Id.eq(id))
            .exec(db)
            .await?;
        Ok(())
    }

    /// End a delivery as permanently failed.
    ///
    /// # Errors
    ///
    /// Returns a database error when the update fails.
    pub async fn mark_failed(db: &DatabaseConnection, id: i64, error: &str) -> ModelResult<()> {
        Entity::update_many()
            .col_expr(Column::Status, Expr::value(DeliveryStatus::Failed.as_str()))
            .col_expr(
                Column::NextAttemptAt,
                Expr::value(Option::<DateTime<Utc>>::None),
            )
            .col_expr(Column::LastError, Expr::value(Some(error.to_string())))
            .col_expr(Column::UpdatedAt, Expr::value(Utc::now()))
            .filter(Column::Id.eq(id))
            .exec(db)
            .await?;
        Ok(())
    }

    /// End a delivery without sending because it is no longer relevant.
    ///
    /// # Errors
    ///
    /// Returns a database error when the update fails.
    pub async fn mark_cancelled(db: &DatabaseConnection, id: i64, reason: &str) -> ModelResult<()> {
        Entity::update_many()
            .col_expr(
                Column::Status,
                Expr::value(DeliveryStatus::Cancelled.as_str()),
            )
            .col_expr(
                Column::NextAttemptAt,
                Expr::value(Option::<DateTime<Utc>>::None),
            )
            .col_expr(Column::LastError, Expr::value(Some(reason.to_string())))
            .col_expr(Column::UpdatedAt, Expr::value(Utc::now()))
            .filter(Column::Id.eq(id))
            .exec(db)
            .await?;
        Ok(())
    }

    /// Requeue deliveries abandoned in `processing` by a crashed worker.
    /// Returns the number of rows reset.
    ///
    /// # Errors
    ///
    /// Returns a database error when the update fails.
    pub async fn reset_stale_processing(
        db: &DatabaseConnection,
        stale_before: DateTime<Utc>,
        now: DateTime<Utc>,
    ) -> ModelResult<u64> {
        let result = Entity::update_many()
            .col_expr(
                Column::Status,
                Expr::value(DeliveryStatus::Pending.as_str()),
            )
            .col_expr(
                Column::NextAttemptAt,
                Expr::value(Option::<DateTime<Utc>>::None),
            )
            .col_expr(Column::UpdatedAt, Expr::value(now))
            .filter(Column::Status.eq(DeliveryStatus::Processing.as_str()))
            .filter(Column::UpdatedAt.lt(stale_before))
            .exec(db)
            .await?;
        Ok(result.rows_affected)
    }

    /// Deliveries that are due to be processed.
    ///
    /// # Errors
    ///
    /// Returns a database error when the query fails.
    pub async fn pending_due(
        db: &DatabaseConnection,
        now: DateTime<Utc>,
        limit: u64,
    ) -> ModelResult<Vec<Self>> {
        Entity::find()
            .filter(Column::Status.eq(DeliveryStatus::Pending.as_str()))
            .filter(
                Condition::any()
                    .add(Column::NextAttemptAt.is_null())
                    .add(Column::NextAttemptAt.lte(now)),
            )
            .order_by_asc(Column::ScheduledAt)
            .limit(limit)
            .all(db)
            .await
            .map_err(ModelError::from)
    }

    /// Delete finished deliveries older than the retention window. Returns the
    /// number of rows removed.
    ///
    /// # Errors
    ///
    /// Returns a database error when the delete fails.
    pub async fn delete_finished_before(
        db: &DatabaseConnection,
        cutoff: DateTime<Utc>,
    ) -> ModelResult<u64> {
        let result = Entity::delete_many()
            .filter(Column::Status.is_in([
                DeliveryStatus::Sent.as_str(),
                DeliveryStatus::Failed.as_str(),
                DeliveryStatus::Cancelled.as_str(),
            ]))
            .filter(Column::UpdatedAt.lt(cutoff))
            .exec(db)
            .await?;
        Ok(result.rows_affected)
    }

    #[must_use]
    pub fn channel(&self) -> Option<NotificationChannel> {
        NotificationChannel::parse(&self.channel)
    }

    #[must_use]
    pub fn notification_type(&self) -> Option<NotificationType> {
        NotificationType::parse(&self.notification_type)
    }

    #[must_use]
    pub fn delivery_status(&self) -> Option<DeliveryStatus> {
        DeliveryStatus::parse(&self.status)
    }
}
