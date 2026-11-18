use axum::http::StatusCode;
use loco_rs::{controller::ErrorDetail, prelude::*};

use crate::{
    dtos::notifications::{TestNotificationParams, TestNotificationResponse},
    models::users,
    notifications::{dispatch, error::NotificationError},
};

/// Map a notification failure onto a safe HTTP response. Provider bodies and
/// credentials never appear in the message.
fn map_error(error: NotificationError) -> Error {
    let (status, code) = match &error {
        NotificationError::Disabled(_) => (StatusCode::CONFLICT, "notifications_disabled"),
        NotificationError::NotConfigured(_) => {
            (StatusCode::CONFLICT, "notifications_not_configured")
        }
        NotificationError::InvalidInput(_) | NotificationError::UnsafeUrl(_) => (
            StatusCode::UNPROCESSABLE_ENTITY,
            "notification_invalid_configuration",
        ),
        NotificationError::RateLimited | NotificationError::ProviderRateLimited { .. } => {
            (StatusCode::TOO_MANY_REQUESTS, "notification_rate_limited")
        }
        NotificationError::Timeout => (StatusCode::GATEWAY_TIMEOUT, "notification_timeout"),
        NotificationError::Authentication => {
            (StatusCode::BAD_GATEWAY, "notification_authentication")
        }
        NotificationError::Permanent(_) | NotificationError::Provider(_) => {
            (StatusCode::BAD_GATEWAY, "notification_provider_error")
        }
        NotificationError::InvalidResponse(_) => {
            (StatusCode::BAD_GATEWAY, "notification_invalid_response")
        }
        NotificationError::Encryption(_) => {
            (StatusCode::INTERNAL_SERVER_ERROR, "notification_encryption")
        }
    };

    Error::CustomError(status, ErrorDetail::new(code, error.to_string()))
}

#[debug_handler]
async fn test(
    auth: auth::JWT,
    State(ctx): State<AppContext>,
    Json(params): Json<TestNotificationParams>,
) -> Result<Response> {
    let user = users::Model::find_by_pid(&ctx.db, &auth.claims.pid)
        .await
        .map_err(Error::from)?;

    dispatch::send_test(&ctx, user.id, params.channel)
        .await
        .map_err(map_error)?;

    tracing::info!(
        user_id = user.id,
        channel = params.channel.as_str(),
        "notification test sent"
    );

    format::json(TestNotificationResponse {
        channel: params.channel,
        status: "sent".to_string(),
    })
}

pub fn routes() -> Routes {
    Routes::new().add("/api/notifications/test", post(test))
}
