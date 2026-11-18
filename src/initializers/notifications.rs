use async_trait::async_trait;
use loco_rs::{
    app::{AppContext, Initializer},
    config::WorkerMode,
    Result,
};

/// Requeue deliveries that were pending or abandoned when the process stopped.
///
/// The scheduler's recovery task repeats this every minute, so this hook is an
/// optimisation for a fast restart, not the only safety net. It is skipped when
/// the queue is not running (`ForegroundBlocking` in tests) so a test boot never
/// sends anything.
pub struct Notifications;

#[async_trait]
impl Initializer for Notifications {
    fn name(&self) -> String {
        "notifications".to_string()
    }

    async fn before_run(&self, ctx: &AppContext) -> Result<()> {
        if matches!(ctx.config.workers.mode, WorkerMode::BackgroundQueue) {
            let recovered = crate::notifications::scan::recover(ctx, chrono::Utc::now()).await?;
            tracing::info!(recovered, "notification deliveries recovered at boot");
        }
        Ok(())
    }
}
