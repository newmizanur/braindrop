use serde::{Deserialize, Serialize};
use ts_rs::TS;

use crate::models::notification_deliveries::NotificationChannel;

/// The body of `POST /api/notifications/test`.
#[derive(Clone, Debug, Deserialize, Serialize, TS)]
#[ts(export, export_to = "../frontend/src/bindings/")]
pub struct TestNotificationParams {
    pub channel: NotificationChannel,
}

/// A successful test send. No provider response body is echoed back.
#[derive(Clone, Debug, Deserialize, Serialize, TS)]
#[ts(export, export_to = "../frontend/src/bindings/")]
pub struct TestNotificationResponse {
    pub channel: NotificationChannel,
    pub status: String,
}
