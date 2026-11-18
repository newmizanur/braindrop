//! Turning a persisted delivery into a provider request.
//!
//! Every send re-reads the user's preferences, the stored credentials and the
//! item it refers to. That is deliberate: a queued job may run minutes after it
//! was created, and the user may have completed the item, removed the channel
//! or deleted their credentials in the meantime. Nothing here trusts the state
//! captured at enqueue time except the delivery id.

use std::sync::Arc;

use chrono::{DateTime, Duration, Utc};
use loco_rs::prelude::*;

use crate::{
    ai::{config::AiSettings, credentials::SecretBox},
    models::{
        items::{self, ItemStatus},
        notification_deliveries::{DeliveryStatus, Model as Delivery, NotificationChannel},
        user_settings,
    },
    notifications::{
        config::NotificationSettings,
        error::NotificationError,
        message::{self, OutgoingMessage},
        provider::{ChannelCredentials, HttpNotificationSender, NotificationSender},
        validate,
    },
    time,
};

/// What the dispatcher decided to do with a delivery.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum DeliveryOutcome {
    /// The delivery is still relevant; send this message.
    Send(OutgoingMessage),
    /// The delivery is obsolete; cancel it with this reason.
    Skip(String),
}

/// Read the notification configuration block.
///
/// # Errors
///
/// Returns a typed error when the `settings:` block is malformed.
pub fn notification_settings(ctx: &AppContext) -> Result<NotificationSettings, NotificationError> {
    NotificationSettings::from_config(&ctx.config)
        .map_err(|error| NotificationError::NotConfigured(error.to_string()))
}

/// Build the credential cipher, falling back to the AI encryption key.
///
/// # Errors
///
/// Returns [`NotificationError::NotConfigured`] when the deployment has no
/// usable key.
pub fn secret_box(
    ctx: &AppContext,
    config: &NotificationSettings,
) -> Result<SecretBox, NotificationError> {
    let ai = AiSettings::from_config(&ctx.config)
        .map_err(|error| NotificationError::Encryption(error.to_string()))?;
    config.secret_box(&ai)?.ok_or_else(|| {
        NotificationError::NotConfigured(
            "the server has no credential encryption key configured".to_string(),
        )
    })
}

/// Use a provider installed in the shared store (tests) or build the real one.
///
/// # Errors
///
/// Returns an error when the HTTP client or the configured Telegram endpoint is
/// invalid.
pub fn sender(
    ctx: &AppContext,
    config: &NotificationSettings,
) -> Result<Arc<dyn NotificationSender>, NotificationError> {
    if let Some(sender) = ctx.shared_store.get::<Arc<dyn NotificationSender>>() {
        return Ok(sender);
    }
    Ok(Arc::new(HttpNotificationSender::new(config)?))
}

/// Decrypt and validate the credentials for one channel.
///
/// # Errors
///
/// Returns [`NotificationError::NotConfigured`] when nothing is stored and
/// [`NotificationError::Encryption`] when the stored envelope cannot be opened.
pub fn credentials(
    settings: &user_settings::Model,
    channel: NotificationChannel,
    secret: &SecretBox,
) -> Result<ChannelCredentials, NotificationError> {
    match channel {
        NotificationChannel::Telegram => {
            let envelope = settings
                .telegram_bot_token_encrypted
                .as_deref()
                .ok_or_else(|| {
                    NotificationError::NotConfigured("no Telegram bot token is stored".to_string())
                })?;
            let token = secret
                .decrypt(envelope)
                .map_err(|error| NotificationError::Encryption(error.to_string()))?;
            let token = validate::telegram_bot_token(&token)?;
            let chat_id = validate::telegram_chat_id(&settings.telegram_chat_id)?;
            Ok(ChannelCredentials::Telegram {
                bot_token: token.to_string(),
                chat_id: chat_id.to_string(),
            })
        }
        NotificationChannel::Discord => {
            let envelope = settings
                .discord_webhook_encrypted
                .as_deref()
                .ok_or_else(|| {
                    NotificationError::NotConfigured("no Discord webhook is stored".to_string())
                })?;
            let webhook = secret
                .decrypt(envelope)
                .map_err(|error| NotificationError::Encryption(error.to_string()))?;
            let url = validate::discord_webhook(&webhook)?;
            Ok(ChannelCredentials::Discord {
                webhook_url: url.to_string(),
            })
        }
    }
}

/// Send a one-off test message through one channel.
///
/// The master notification switch is intentionally ignored: a user who is still
/// setting the channel up wants to verify it before enabling anything. The
/// channel itself must be enabled and fully configured.
///
/// # Errors
///
/// Returns a typed error for disabled channels, missing credentials, rate
/// limiting and provider failures.
pub async fn send_test(
    ctx: &AppContext,
    user_id: i64,
    channel: NotificationChannel,
) -> Result<(), NotificationError> {
    let config = notification_settings(ctx)?;
    let settings = user_settings::Model::get_or_create(&ctx.db, user_id)
        .await
        .map_err(|error| NotificationError::Provider(error.to_string()))?;

    if !settings.channel_ready(channel) {
        return Err(NotificationError::NotConfigured(format!(
            "the {} channel is not enabled and configured",
            channel.as_str()
        )));
    }

    crate::notifications::rate_limit::check(ctx, user_id, config.test_rate_limit()).await?;

    let secret = secret_box(ctx, &config)?;
    let credentials = credentials(&settings, channel, &secret)?;
    let sender = sender(ctx, &config)?;
    sender
        .send(
            channel,
            &credentials,
            &message::test_message(channel.as_str()),
        )
        .await
}

/// Decide what a delivery should contain right now.
///
/// # Errors
///
/// Returns a database or configuration error. A missing item or a disabled
/// preference is a [`DeliveryOutcome::Skip`], not an error.
pub async fn prepare(
    ctx: &AppContext,
    delivery: &Delivery,
    now: DateTime<Utc>,
) -> Result<DeliveryOutcome, NotificationError> {
    let Some(channel) = delivery.channel() else {
        return Ok(DeliveryOutcome::Skip(
            "the delivery has an unknown channel".to_string(),
        ));
    };
    let Some(notification_type) = delivery.notification_type() else {
        return Ok(DeliveryOutcome::Skip(
            "the delivery has an unknown notification type".to_string(),
        ));
    };

    let settings = user_settings::Model::find_by_user(&ctx.db, delivery.user_id)
        .await
        .map_err(|error| NotificationError::Provider(error.to_string()))?
        .ok_or_else(|| {
            NotificationError::NotConfigured("the account has no settings row".to_string())
        })?;

    if !settings.notifications_enabled() {
        return Ok(DeliveryOutcome::Skip(
            "notifications are disabled for the account".to_string(),
        ));
    }
    if !settings.channel_ready(channel) {
        return Ok(DeliveryOutcome::Skip(format!(
            "the {} channel is disabled or incomplete",
            channel.as_str()
        )));
    }

    let enabled_for_type = match notification_type {
        crate::models::notification_deliveries::NotificationType::Reminder => {
            settings.reminders_enabled
        }
        crate::models::notification_deliveries::NotificationType::DailyDigest => {
            settings.daily_digest_enabled
        }
        crate::models::notification_deliveries::NotificationType::Overdue => {
            settings.overdue_notifications_enabled
        }
    };
    if !enabled_for_type {
        return Ok(DeliveryOutcome::Skip(
            "this notification type is disabled".to_string(),
        ));
    }

    let timezone = settings.timezone();
    match notification_type {
        crate::models::notification_deliveries::NotificationType::Reminder => {
            let Some(item_id) = delivery.item_id else {
                return Ok(DeliveryOutcome::Skip(
                    "the item no longer exists".to_string(),
                ));
            };
            let Some(item) = items::Entity::find_by_id(item_id)
                .one(&ctx.db)
                .await
                .map_err(|error| NotificationError::Provider(error.to_string()))?
            else {
                return Ok(DeliveryOutcome::Skip("the item was deleted".to_string()));
            };
            if item.user_id != delivery.user_id {
                return Ok(DeliveryOutcome::Skip(
                    "the item no longer belongs to this account".to_string(),
                ));
            }
            if !ItemStatus::parse(&item.status).is_some_and(ItemStatus::is_open) {
                return Ok(DeliveryOutcome::Skip(
                    "the item is completed or archived".to_string(),
                ));
            }
            // The reminder timestamp is part of the dedup key. A changed
            // reminder produces a new delivery; the old one stops here.
            if item.remind_at != Some(delivery.scheduled_at) {
                return Ok(DeliveryOutcome::Skip(
                    "the reminder was changed".to_string(),
                ));
            }
            Ok(DeliveryOutcome::Send(message::reminder(&item, timezone)))
        }
        crate::models::notification_deliveries::NotificationType::DailyDigest => {
            let config = notification_settings(ctx)?;
            let reference = delivery.scheduled_at.with_timezone(&Utc);
            let date = time::daily_window_date(settings.daily_digest_time(), timezone, reference);
            let (due_today, upcoming, overdue) =
                digest_items(ctx, delivery.user_id, timezone, date).await?;
            if due_today.is_empty() && upcoming.is_empty() && overdue.is_empty() {
                return Ok(DeliveryOutcome::Skip(
                    "the digest would be empty".to_string(),
                ));
            }
            Ok(DeliveryOutcome::Send(message::digest(
                &due_today,
                &upcoming,
                &overdue,
                timezone,
                config.message_items(),
            )))
        }
        crate::models::notification_deliveries::NotificationType::Overdue => {
            let overdue = overdue_items(ctx, delivery.user_id, timezone, now).await?;
            if overdue.is_empty() {
                return Ok(DeliveryOutcome::Skip("nothing is overdue".to_string()));
            }
            Ok(DeliveryOutcome::Send(message::overdue(
                &overdue,
                timezone,
                message_items(ctx)?,
            )))
        }
    }
}

/// Partition the user's open, dated items into the three digest sections.
///
/// # Errors
///
/// Returns a database error when the item query fails.
pub async fn digest_items(
    ctx: &AppContext,
    user_id: i64,
    timezone: chrono_tz::Tz,
    date: chrono::NaiveDate,
) -> Result<(Vec<items::Model>, Vec<items::Model>, Vec<items::Model>), NotificationError> {
    let (start, end) = time::day_bounds(timezone, date);
    let upcoming_end = time::day_bounds(timezone, date + Duration::days(7)).1;
    let open = open_items_with_due(ctx, user_id).await?;

    let mut due_today = Vec::new();
    let mut upcoming = Vec::new();
    let mut overdue = Vec::new();
    for item in open {
        let Some(due_at) = item.due_at else {
            continue;
        };
        if due_at < start {
            overdue.push(item);
        } else if due_at < end {
            due_today.push(item);
        } else if due_at < upcoming_end {
            upcoming.push(item);
        }
    }
    Ok((due_today, upcoming, overdue))
}

/// The user's open items whose due date is before today.
///
/// # Errors
///
/// Returns a database error when the item query fails.
pub async fn overdue_items(
    ctx: &AppContext,
    user_id: i64,
    timezone: chrono_tz::Tz,
    now: DateTime<Utc>,
) -> Result<Vec<items::Model>, NotificationError> {
    let date = time::local_date(timezone, now);
    let start = time::start_of_day(timezone, date);
    let open = open_items_with_due(ctx, user_id).await?;
    Ok(open
        .into_iter()
        .filter(|item| item.due_at.is_some_and(|due_at| due_at < start))
        .collect())
}

fn message_items(ctx: &AppContext) -> Result<usize, NotificationError> {
    Ok(notification_settings(ctx)?.message_items())
}

async fn open_items_with_due(
    ctx: &AppContext,
    user_id: i64,
) -> Result<Vec<items::Model>, NotificationError> {
    items::Entity::find()
        .filter(items::Column::UserId.eq(user_id))
        .filter(
            items::Column::Status.is_in([ItemStatus::Inbox.as_str(), ItemStatus::Pending.as_str()]),
        )
        .filter(items::Column::DueAt.is_not_null())
        .all(&ctx.db)
        .await
        .map_err(|error| NotificationError::Provider(error.to_string()))
}

/// Compute the delay before the next attempt.
///
/// The provider's `Retry-After` wins when it supplies one, capped by the
/// configured maximum. Otherwise the delay doubles per attempt.
#[must_use]
pub fn backoff(
    config: &NotificationSettings,
    attempts: i64,
    retry_after: Option<std::time::Duration>,
) -> Duration {
    if let Some(retry_after) = retry_after {
        let seconds = retry_after.as_secs().min(config.retry_max().as_secs());
        return Duration::seconds(i64::try_from(seconds).unwrap_or(i64::MAX));
    }
    let exponent = u32::try_from(attempts.saturating_sub(1).clamp(0, 10)).unwrap_or(0);
    let seconds = config
        .retry_base()
        .as_secs()
        .saturating_mul(1u64 << exponent)
        .min(config.retry_max().as_secs());
    Duration::seconds(i64::try_from(seconds).unwrap_or(i64::MAX))
}

/// Whether a delivery is still claimable.
#[must_use]
pub fn is_pending(delivery: &Delivery) -> bool {
    delivery.delivery_status() == Some(DeliveryStatus::Pending)
}
