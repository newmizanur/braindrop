//! Scheduler entry points for notifications.
//!
//! Each task is a thin wrapper around one scan in
//! [`crate::notifications::scan`]. The scheduler names them in
//! `config/<env>.yaml`; running the same task by hand is safe because every
//! scan is idempotent.

use loco_rs::prelude::*;

fn now() -> chrono::DateTime<chrono::Utc> {
    chrono::Utc::now()
}

pub struct NotificationReminders;
#[async_trait]
impl Task for NotificationReminders {
    fn task(&self) -> TaskInfo {
        TaskInfo {
            name: "notifications:reminders".to_string(),
            detail: "Enqueue reminders whose remind_at has arrived.".to_string(),
        }
    }
    async fn run(&self, ctx: &AppContext, _vars: &task::Vars) -> Result<()> {
        let count = crate::notifications::scan::reminders(ctx, now()).await?;
        tracing::info!(count, "notification reminder scan finished");
        Ok(())
    }
}

pub struct NotificationDigests;
#[async_trait]
impl Task for NotificationDigests {
    fn task(&self) -> TaskInfo {
        TaskInfo {
            name: "notifications:digests".to_string(),
            detail: "Enqueue daily digests for users whose local delivery time arrived."
                .to_string(),
        }
    }
    async fn run(&self, ctx: &AppContext, _vars: &task::Vars) -> Result<()> {
        let count = crate::notifications::scan::digests(ctx, now()).await?;
        tracing::info!(count, "notification digest scan finished");
        Ok(())
    }
}

pub struct NotificationOverdue;
#[async_trait]
impl Task for NotificationOverdue {
    fn task(&self) -> TaskInfo {
        TaskInfo {
            name: "notifications:overdue".to_string(),
            detail: "Enqueue the daily grouped overdue notification.".to_string(),
        }
    }
    async fn run(&self, ctx: &AppContext, _vars: &task::Vars) -> Result<()> {
        let count = crate::notifications::scan::overdue(ctx, now()).await?;
        tracing::info!(count, "notification overdue scan finished");
        Ok(())
    }
}

pub struct NotificationRecover;
#[async_trait]
impl Task for NotificationRecover {
    fn task(&self) -> TaskInfo {
        TaskInfo {
            name: "notifications:recover".to_string(),
            detail: "Requeue pending deliveries and reset abandoned ones.".to_string(),
        }
    }
    async fn run(&self, ctx: &AppContext, _vars: &task::Vars) -> Result<()> {
        let count = crate::notifications::scan::recover(ctx, now()).await?;
        tracing::info!(count, "notification recovery scan finished");
        Ok(())
    }
}

pub struct NotificationCleanup;
#[async_trait]
impl Task for NotificationCleanup {
    fn task(&self) -> TaskInfo {
        TaskInfo {
            name: "notifications:cleanup".to_string(),
            detail: "Delete finished delivery history past the retention window.".to_string(),
        }
    }
    async fn run(&self, ctx: &AppContext, _vars: &task::Vars) -> Result<()> {
        let count = crate::notifications::scan::cleanup(ctx, now()).await?;
        tracing::info!(count, "notification cleanup finished");
        Ok(())
    }
}
