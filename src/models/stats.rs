//! Dashboard aggregates derived from the user's items.
//!
//! One conditional aggregate scan computes the dashboard counts, scoped to the
//! user, without loading an unbounded item list into memory.

use chrono::{DateTime, Duration, Utc};
use loco_rs::prelude::*;
use sea_orm::{DatabaseBackend, FromQueryResult, Statement};
use serde::{Deserialize, Serialize};
use ts_rs::TS;

use crate::{models::user_settings, time};

/// The numbers shown on the dashboard. All day and week windows are resolved
/// in the user's timezone and queried in UTC.
#[derive(Clone, Debug, Serialize, Deserialize, TS)]
#[ts(export)]
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

#[derive(FromQueryResult)]
struct Counts {
    total: i64,
    inbox: i64,
    pending: i64,
    completed: i64,
    archived: i64,
    due_today: i64,
    overdue: i64,
    upcoming: i64,
    completed_today: i64,
    completed_week: i64,
    tasks: i64,
    notes: i64,
    ideas: i64,
    reminders: i64,
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

    let query = Statement::from_sql_and_values(
        DatabaseBackend::Sqlite,
        r#"SELECT
            COUNT(*) AS total,
            COALESCE(SUM(CASE WHEN status = 'inbox' THEN 1 ELSE 0 END), 0) AS inbox,
            COALESCE(SUM(CASE WHEN status = 'pending' THEN 1 ELSE 0 END), 0) AS pending,
            COALESCE(SUM(CASE WHEN status = 'completed' THEN 1 ELSE 0 END), 0) AS completed,
            COALESCE(SUM(CASE WHEN status = 'archived' THEN 1 ELSE 0 END), 0) AS archived,
            COALESCE(SUM(CASE WHEN status IN ('inbox', 'pending') AND due_at >= ? AND due_at < ? THEN 1 ELSE 0 END), 0) AS due_today,
            COALESCE(SUM(CASE WHEN status IN ('inbox', 'pending') AND due_at IS NOT NULL AND due_at < ? THEN 1 ELSE 0 END), 0) AS overdue,
            COALESCE(SUM(CASE WHEN status IN ('inbox', 'pending') AND due_at >= ? AND due_at < ? THEN 1 ELSE 0 END), 0) AS upcoming,
            COALESCE(SUM(CASE WHEN status = 'completed' AND completed_at >= ? THEN 1 ELSE 0 END), 0) AS completed_today,
            COALESCE(SUM(CASE WHEN status = 'completed' AND completed_at >= ? THEN 1 ELSE 0 END), 0) AS completed_week,
            COALESCE(SUM(CASE WHEN item_type = 'task' THEN 1 ELSE 0 END), 0) AS tasks,
            COALESCE(SUM(CASE WHEN item_type = 'note' THEN 1 ELSE 0 END), 0) AS notes,
            COALESCE(SUM(CASE WHEN item_type = 'idea' THEN 1 ELSE 0 END), 0) AS ideas,
            COALESCE(SUM(CASE WHEN item_type = 'reminder' THEN 1 ELSE 0 END), 0) AS reminders
         FROM items WHERE user_id = ?"#,
        [
            today_start.fixed_offset().into(),
            tomorrow_start.fixed_offset().into(),
            now.fixed_offset().into(),
            tomorrow_start.fixed_offset().into(),
            upcoming_end.fixed_offset().into(),
            today_start.fixed_offset().into(),
            week_start.fixed_offset().into(),
            user_id.into(),
        ],
    );
    let counts = Counts::find_by_statement(query)
        .one(db)
        .await?
        .ok_or(Error::InternalServerError)?;

    Ok(Stats {
        total: counts.total,
        inbox: counts.inbox,
        pending: counts.pending,
        completed: counts.completed,
        archived: counts.archived,
        due_today: counts.due_today,
        overdue: counts.overdue,
        upcoming: counts.upcoming,
        completed_today: counts.completed_today,
        completed_week: counts.completed_week,
        tasks: counts.tasks,
        notes: counts.notes,
        ideas: counts.ideas,
        reminders: counts.reminders,
        generated_at: now,
    })
}
