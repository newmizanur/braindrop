use loco_rs::prelude::*;
use serde::{Deserialize, Serialize};

/// Sends one persisted notification delivery.
///
/// The job carries only the row id: preferences, credentials and item state are
/// re-read when the job runs. Retries are handled by the delivery record, not
/// by the queue, so `max_retries` is zero.
pub struct NotificationDeliveryWorker {
    pub ctx: AppContext,
}

#[derive(Deserialize, Debug, Serialize)]
pub struct NotificationDeliveryArgs {
    pub delivery_id: i64,
}

#[async_trait]
impl BackgroundWorker<NotificationDeliveryArgs> for NotificationDeliveryWorker {
    fn build(ctx: &AppContext) -> Self {
        Self { ctx: ctx.clone() }
    }

    fn max_retries() -> u32 {
        0
    }

    async fn perform(&self, args: NotificationDeliveryArgs) -> Result<()> {
        crate::notifications::scan::deliver(&self.ctx, args.delivery_id, chrono::Utc::now()).await
    }
}
