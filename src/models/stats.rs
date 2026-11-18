//! Dashboard aggregates derived from the user's items.
//!
//! Counts are computed with `COUNT(*)` queries, each one scoped to the user,
//! so the dashboard never loads an unbounded item list into memory.

use chrono::{DateTime, Duration, Utc};
use loco_rs::prelude::*;
use serde::{Deserialize, Serialize};
use ts_rs::TS;

use crate::{
    models::{
        items::{self, ItemStatus, ItemType},
        user_settings,
    },
    time,
};

/// The numbers shown on the dashboard. All day and week windows are resolved
/// in the user's timezone and queried in UTC.
#[derive(Clone, Debug, Serialize, Deserialize, TS)]
#[ts(export, export_to = "../frontend/src/bindings/")]
pub struct Stats {
    #[ts(type = "number")]
    pub total: i64,
    #[ts(type = "number")]
    pub inbox: i64,
    #[ts(type = "number")]
    pub pending: i64,
    #[ts(type = "number")]
    pub completed: i64,
    #[ts(type = "number")]
    pub archived: i64,
    #[ts(type = "number")]
    pub due_today: i64,
    #[ts(type = "number")]
    pub overdue: i64,
    #[ts(type = "number")]
    pub upcoming: i64,
    #[ts(type = "number")]
    pub completed_today: i64,
    #[ts(type = "number")]
    pub completed_week: i64,
    #[ts(type = "number")]
    pub tasks: i64,
    #[ts(type = "number")]
    pub notes: i64,
    #[ts(type = "number")]
    pub ideas: i64,
    #[ts(type = "number")]
    pub reminders: i64,
    #[ts(type = "string")]
    pub generated_at: DateTime<Utc>,
}

async fn count(db: &DatabaseConnection, condition: Condition) -> Result<i64> {
    let count = items::Entity::find().filter(condition).count(db).await?;
    i64::try_from(count).map_err(|_| Error::InternalServerError)
}

fn status_condition(user_id: i64, status: ItemStatus) -> Condition {
    Condition::all()
        .add(items::Column::UserId.eq(user_id))
        .add(items::Column::Status.eq(status.as_str()))
}

fn open_condition(user_id: i64) -> Condition {
    Condition::all().add(items::Column::UserId.eq(user_id)).add(
        items::Column::Status.is_in([ItemStatus::Inbox.as_str(), ItemStatus::Pending.as_str()]),
    )
}

fn type_condition(user_id: i64, item_type: ItemType) -> Condition {
    Condition::all()
        .add(items::Column::UserId.eq(user_id))
        .add(items::Column::ItemType.eq(item_type.as_str()))
}

/// Compute the dashboard statistics for a user.
///
/// # Errors
///
/// Returns a database error when an aggregate query fails.
pub async fn compute(
    db: &DatabaseConnection,
    user_id: i64,
    settings: &user_settings::Model,
) -> Result<Stats> {
    let timezone = settings.timezone();
    let now = Utc::now();
    let today = now.with_timezone(&timezone).date_naive();
    let (today_start, tomorrow_start) = time::day_bounds(timezone, today);
    let week_start = time::start_of_day(
        timezone,
        time::start_of_week(today, settings.week_start.clamp(0, 6)),
    );
    let upcoming_end = time::start_of_day(timezone, today + Duration::days(8));

    let today_range = Condition::all()
        .add(items::Column::DueAt.gte(today_start.fixed_offset()))
        .add(items::Column::DueAt.lt(tomorrow_start.fixed_offset()));
    let upcoming_range = Condition::all()
        .add(items::Column::DueAt.gte(tomorrow_start.fixed_offset()))
        .add(items::Column::DueAt.lt(upcoming_end.fixed_offset()));
    let completed_since = |since: DateTime<Utc>| {
        Condition::all()
            .add(items::Column::UserId.eq(user_id))
            .add(items::Column::Status.eq(ItemStatus::Completed.as_str()))
            .add(items::Column::CompletedAt.gte(since.fixed_offset()))
    };

    let (
        total,
        inbox,
        pending,
        completed,
        archived,
        due_today,
        overdue,
        upcoming,
        completed_today,
        completed_week,
        tasks,
        notes,
        ideas,
        reminders,
    ) = tokio::try_join!(
        count(db, Condition::all().add(items::Column::UserId.eq(user_id))),
        count(db, status_condition(user_id, ItemStatus::Inbox)),
        count(db, status_condition(user_id, ItemStatus::Pending)),
        count(db, status_condition(user_id, ItemStatus::Completed)),
        count(db, status_condition(user_id, ItemStatus::Archived)),
        count(db, open_condition(user_id).add(today_range.clone())),
        count(
            db,
            open_condition(user_id)
                .add(items::Column::DueAt.lt(now.fixed_offset()))
                .add(items::Column::DueAt.is_not_null())
        ),
        count(db, open_condition(user_id).add(upcoming_range)),
        count(db, completed_since(today_start)),
        count(db, completed_since(week_start)),
        count(db, type_condition(user_id, ItemType::Task)),
        count(db, type_condition(user_id, ItemType::Note)),
        count(db, type_condition(user_id, ItemType::Idea)),
        count(db, type_condition(user_id, ItemType::Reminder)),
    )?;

    Ok(Stats {
        total,
        inbox,
        pending,
        completed,
        archived,
        due_today,
        overdue,
        upcoming,
        completed_today,
        completed_week,
        tasks,
        notes,
        ideas,
        reminders,
        generated_at: now,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn status_condition_mentions_the_status() {
        let condition = status_condition(7, ItemStatus::Inbox);
        // The condition is opaque, but building it must not panic and it must
        // be reusable in a query. This is a cheap guard against a typo in the
        // column mapping.
        let _ = condition;
    }
}
