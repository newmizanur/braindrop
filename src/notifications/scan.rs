//! Scheduler scans that turn item state into durable delivery records.
//!
//! Every scan is idempotent: it may run on every scheduler tick and after a
//! restart, because the `notification_deliveries` unique key rejects a second
//! insert for the same reminder, digest day or overdue day. Creating the record
//! happens before the job is enqueued, so a crash between the two is recovered
//! by the pending-delivery scan rather than losing the notification.

use std::collections::HashMap;

use chrono::{DateTime, Utc};
use loco_rs::prelude::*;

use crate::{
    models::{
        items::{self, ItemStatus},
        notification_deliveries::{
            digest_key, overdue_key, reminder_key, Entity as DeliveryEntity, Model as Delivery,
            NewDelivery, NotificationType,
        },
        user_settings,
    },
    notifications::{
        config::NotificationSettings,
        dispatch::{self, DeliveryOutcome},
        error::redacted_db_error,
        provider::NotificationSender,
    },
    time,
    workers::notification_delivery::{NotificationDeliveryArgs, NotificationDeliveryWorker},
};

/// Enqueue a delivery record. Unique jobs keep a second scan from stacking
/// duplicate queue entries while one is still waiting.
async fn enqueue(ctx: &AppContext, delivery_id: i64) -> Result<()> {
    NotificationDeliveryWorker::perform_later_unique(
        ctx,
        NotificationDeliveryArgs { delivery_id },
        format!("notification-delivery:{delivery_id}"),
    )
    .await?;
    Ok(())
}

/// Persist a delivery and enqueue it when this call created the record.
///
/// # Errors
///
/// Returns a database or queue error.
pub async fn schedule(ctx: &AppContext, new: NewDelivery) -> Result<bool> {
    let Some(id) = Delivery::create(&ctx.db, new).await? else {
        return Ok(false);
    };
    enqueue(ctx, id).await?;
    Ok(true)
}

/// Enqueue reminders whose `remind_at` has arrived.
///
/// The grace window keeps a restart from replaying every reminder ever missed;
/// a reminder older than the window is simply skipped.
///
/// # Errors
///
/// Returns a database or queue error.
pub async fn reminders(ctx: &AppContext, now: DateTime<Utc>) -> Result<u64> {
    let config = NotificationSettings::from_config(&ctx.config)?;
    let grace = config.reminder_grace();

    let candidates = items::Entity::find()
        .filter(items::Column::RemindAt.is_not_null())
        .filter(items::Column::RemindAt.lte(now))
        .filter(items::Column::RemindAt.gte(now - grace))
        .filter(
            items::Column::Status.is_in([ItemStatus::Inbox.as_str(), ItemStatus::Pending.as_str()]),
        )
        .all(&ctx.db)
        .await?;

    let mut by_user: HashMap<i64, Vec<items::Model>> = HashMap::new();
    for item in candidates {
        by_user.entry(item.user_id).or_default().push(item);
    }

    let mut scheduled = 0;
    for (user_id, items) in by_user {
        let Some(settings) = user_settings::Model::find_by_user(&ctx.db, user_id).await? else {
            continue;
        };
        if !settings.notifications_enabled() || !settings.reminders_enabled {
            continue;
        }
        let channels = settings.enabled_channels();
        if channels.is_empty() {
            continue;
        }

        for item in items {
            let Some(remind_at) = item.remind_at else {
                continue;
            };
            let remind_at = remind_at.with_timezone(&Utc);
            for channel in &channels {
                let created = schedule(
                    ctx,
                    NewDelivery {
                        user_id,
                        item_id: Some(item.id),
                        channel: *channel,
                        notification_type: NotificationType::Reminder,
                        deduplication_key: reminder_key(item.pid, remind_at, *channel),
                        scheduled_at: remind_at,
                    },
                )
                .await?;
                if created {
                    scheduled += 1;
                }
            }
        }
    }

    Ok(scheduled)
}

/// Enqueue the daily digest once per local day and channel.
///
/// Eligibility is the user's configured local time plus a window, so a restart
/// shortly after the delivery time still sends the digest. An empty digest is
/// skipped without creating a record.
///
/// # Errors
///
/// Returns a database or queue error.
pub async fn digests(ctx: &AppContext, now: DateTime<Utc>) -> Result<u64> {
    let config = NotificationSettings::from_config(&ctx.config)?;
    let window = config.digest_window().num_minutes();

    let users = user_settings::Entity::find()
        .filter(user_settings::Column::NotificationsEnabled.eq(true))
        .filter(user_settings::Column::DailyDigestEnabled.eq(true))
        .all(&ctx.db)
        .await?;

    let mut scheduled = 0;
    for settings in users {
        let channels = settings.enabled_channels();
        if channels.is_empty() {
            continue;
        }
        let timezone = settings.timezone();
        if !in_daily_window(settings.daily_digest_time(), timezone, now, window) {
            continue;
        }
        let date = time::daily_window_date(settings.daily_digest_time(), timezone, now);
        let (due_today, upcoming, overdue) =
            dispatch::digest_items(ctx, settings.user_id, timezone, date).await?;
        if due_today.is_empty() && upcoming.is_empty() && overdue.is_empty() {
            continue;
        }

        for channel in channels {
            let created = schedule(
                ctx,
                NewDelivery {
                    user_id: settings.user_id,
                    item_id: None,
                    channel,
                    notification_type: NotificationType::DailyDigest,
                    deduplication_key: digest_key(settings.user_id, date, channel),
                    scheduled_at: now,
                },
            )
            .await?;
            if created {
                scheduled += 1;
            }
        }
    }

    Ok(scheduled)
}

/// Enqueue a grouped overdue notification once per local day and channel.
///
/// The notification becomes eligible at the user's daily delivery time and the
/// hourly scan sends it on the first tick after that, so a restart earlier in
/// the day does not lose it.
///
/// # Errors
///
/// Returns a database or queue error.
pub async fn overdue(ctx: &AppContext, now: DateTime<Utc>) -> Result<u64> {
    let users = user_settings::Entity::find()
        .filter(user_settings::Column::NotificationsEnabled.eq(true))
        .filter(user_settings::Column::OverdueNotificationsEnabled.eq(true))
        .all(&ctx.db)
        .await?;

    let mut scheduled = 0;
    for settings in users {
        let channels = settings.enabled_channels();
        if channels.is_empty() {
            continue;
        }
        let timezone = settings.timezone();
        if !daily_time_reached(settings.daily_digest_time(), timezone, now) {
            continue;
        }
        let overdue = dispatch::overdue_items(ctx, settings.user_id, timezone, now).await?;
        if overdue.is_empty() {
            continue;
        }

        let date = time::local_date(timezone, now);
        for channel in channels {
            let created = schedule(
                ctx,
                NewDelivery {
                    user_id: settings.user_id,
                    item_id: None,
                    channel,
                    notification_type: NotificationType::Overdue,
                    deduplication_key: overdue_key(settings.user_id, date, channel),
                    scheduled_at: now,
                },
            )
            .await?;
            if created {
                scheduled += 1;
            }
        }
    }

    Ok(scheduled)
}

/// Reset deliveries abandoned in `processing` and re-enqueue pending ones.
///
/// This is the restart path: a worker that died mid-send leaves a row in
/// `processing`, and a record written just before a crash may still be
/// `pending` with no queue entry.
///
/// # Errors
///
/// Returns a database or queue error.
pub async fn recover(ctx: &AppContext, now: DateTime<Utc>) -> Result<u64> {
    let config = NotificationSettings::from_config(&ctx.config)?;
    Delivery::reset_stale_processing(&ctx.db, now - config.stale_processing(), now).await?;

    let pending = Delivery::pending_due(&ctx.db, now, config.recovery_batch()).await?;
    let mut count = 0;
    for delivery in pending {
        enqueue(ctx, delivery.id).await?;
        count += 1;
    }
    Ok(count)
}

/// Delete finished delivery history older than the retention window.
///
/// # Errors
///
/// Returns a database error when the delete fails.
pub async fn cleanup(ctx: &AppContext, now: DateTime<Utc>) -> Result<u64> {
    let config = NotificationSettings::from_config(&ctx.config)?;
    let cutoff = now - chrono::Duration::days(config.retention_days());
    Ok(Delivery::delete_finished_before(&ctx.db, cutoff).await?)
}

/// Run one delivery in the foreground. Used by the worker and by tests.
///
/// The claim is atomic, so two workers that pick up the same job cannot both
/// send. Retryable failures go back to `pending` with a backoff; permanent
/// failures end in `failed`; obsolete deliveries end in `cancelled`.
///
/// # Errors
///
/// Returns only unexpected database errors. Provider failures are recorded on
/// the delivery instead of failing the job, so the queue does not retry a
/// message that the delivery record already tracks.
pub async fn deliver(ctx: &AppContext, delivery_id: i64, now: DateTime<Utc>) -> Result<()> {
    let Some(delivery) = DeliveryEntity::find_by_id(delivery_id).one(&ctx.db).await? else {
        return Ok(());
    };
    if !Delivery::claim(&ctx.db, delivery.id, now).await? {
        return Ok(());
    }

    let config = NotificationSettings::from_config(&ctx.config)?;
    let outcome = match dispatch::prepare(ctx, &delivery, now).await {
        Ok(outcome) => outcome,
        Err(error) => {
            Delivery::mark_failed(&ctx.db, delivery.id, &error.to_string()).await?;
            return Ok(());
        }
    };

    let message = match outcome {
        DeliveryOutcome::Skip(reason) => {
            Delivery::mark_cancelled(&ctx.db, delivery.id, &reason).await?;
            return Ok(());
        }
        DeliveryOutcome::Send(message) => message,
    };

    let Some(channel) = delivery.channel() else {
        Delivery::mark_cancelled(&ctx.db, delivery.id, "unknown channel").await?;
        return Ok(());
    };

    let result = async {
        let secret = dispatch::secret_box(ctx, &config)?;
        let settings = user_settings::Model::find_by_user(&ctx.db, delivery.user_id)
            .await
            .map_err(|error| redacted_db_error(error, "find_by_user"))?
            .ok_or_else(|| {
                crate::notifications::error::NotificationError::NotConfigured(
                    "the account has no settings row".to_string(),
                )
            })?;
        let credentials = dispatch::credentials(&settings, channel, &secret)?;
        let sender: std::sync::Arc<dyn NotificationSender> = dispatch::sender(ctx, &config)?;
        sender.send(channel, &credentials, &message).await
    }
    .await;

    match result {
        Ok(()) => {
            Delivery::mark_sent(&ctx.db, delivery.id, now).await?;
        }
        Err(error)
            if matches!(
                error,
                crate::notifications::error::NotificationError::Disabled(_)
                    | crate::notifications::error::NotificationError::NotConfigured(_)
                    | crate::notifications::error::NotificationError::InvalidInput(_)
                    | crate::notifications::error::NotificationError::UnsafeUrl(_)
            ) =>
        {
            Delivery::mark_cancelled(&ctx.db, delivery.id, &error.to_string()).await?;
        }
        Err(error) if error.is_retryable() && delivery.attempts + 1 < config.max_attempts() => {
            let delay = dispatch::backoff(&config, delivery.attempts + 1, error.retry_after());
            Delivery::mark_retry(&ctx.db, delivery.id, now + delay, &error.to_string()).await?;
        }
        Err(error) => {
            Delivery::mark_failed(&ctx.db, delivery.id, &error.to_string()).await?;
        }
    }

    Ok(())
}

fn in_daily_window(
    digest_time: chrono::NaiveTime,
    timezone: chrono_tz::Tz,
    now: DateTime<Utc>,
    window_minutes: i64,
) -> bool {
    let local = time::local_minutes(time::local_time(timezone, now));
    let target = time::local_minutes(digest_time);
    let offset = (local - target).rem_euclid(24 * 60);
    offset < window_minutes
}

fn daily_time_reached(
    digest_time: chrono::NaiveTime,
    timezone: chrono_tz::Tz,
    now: DateTime<Utc>,
) -> bool {
    let local = time::local_minutes(time::local_time(timezone, now));
    local >= time::local_minutes(digest_time)
}

#[cfg(test)]
mod tests {
    use chrono::NaiveTime;

    use super::*;

    fn utc(value: &str) -> DateTime<Utc> {
        DateTime::parse_from_rfc3339(value)
            .unwrap()
            .with_timezone(&Utc)
    }

    #[test]
    fn digest_window_respects_the_local_clock_and_midnight() {
        let berlin = time::parse_timezone("Europe/Berlin").unwrap();
        let nine = NaiveTime::from_hms_opt(9, 0, 0).unwrap();
        // 08:00 UTC is 09:00 in Berlin (winter).
        assert!(in_daily_window(
            nine,
            berlin,
            utc("2026-11-19T08:00:00Z"),
            120
        ));
        assert!(in_daily_window(
            nine,
            berlin,
            utc("2026-11-19T09:59:00Z"),
            120
        ));
        assert!(!in_daily_window(
            nine,
            berlin,
            utc("2026-11-19T10:01:00Z"),
            120
        ));
        assert!(!in_daily_window(
            nine,
            berlin,
            utc("2026-11-19T07:59:00Z"),
            120
        ));

        // A 23:30 digest with a two-hour window wraps into the next day.
        let late = NaiveTime::from_hms_opt(23, 30, 0).unwrap();
        assert!(in_daily_window(
            late,
            berlin,
            utc("2026-11-19T23:15:00Z"),
            120
        ));
        assert!(in_daily_window(
            late,
            berlin,
            utc("2026-11-20T00:15:00Z"),
            120
        ));
        assert!(!in_daily_window(
            late,
            berlin,
            utc("2026-11-19T22:00:00Z"),
            120
        ));
    }

    #[test]
    fn daily_time_reached_compares_local_minutes() {
        let tokyo = time::parse_timezone("Asia/Tokyo").unwrap();
        let nine = NaiveTime::from_hms_opt(9, 0, 0).unwrap();
        assert!(!daily_time_reached(
            nine,
            tokyo,
            utc("2026-11-18T23:59:00Z")
        ));
        assert!(daily_time_reached(nine, tokyo, utc("2026-11-19T00:00:00Z")));
        assert!(daily_time_reached(nine, tokyo, utc("2026-11-19T12:00:00Z")));
    }

    #[test]
    fn dst_transition_does_not_skip_the_digest_window() {
        let berlin = time::parse_timezone("Europe/Berlin").unwrap();
        let nine = NaiveTime::from_hms_opt(9, 0, 0).unwrap();
        // 2026-03-29 is the spring-forward day; 09:00 local is 07:00 UTC.
        assert!(in_daily_window(
            nine,
            berlin,
            utc("2026-03-29T07:00:00Z"),
            120
        ));
        // 2026-10-25 is the fall-back day; 09:00 local is 08:00 UTC.
        assert!(in_daily_window(
            nine,
            berlin,
            utc("2026-10-25T08:00:00Z"),
            120
        ));
    }

    #[test]
    fn local_date_uses_the_zone_not_utc() {
        let tokyo = time::parse_timezone("Asia/Tokyo").unwrap();
        let date = time::local_date(tokyo, utc("2026-11-19T00:30:00Z"));
        assert_eq!(date, chrono::NaiveDate::from_ymd_opt(2026, 11, 19).unwrap());
        let berlin = time::parse_timezone("Europe/Berlin").unwrap();
        let date = time::local_date(berlin, utc("2026-11-19T00:30:00Z"));
        assert_eq!(date, chrono::NaiveDate::from_ymd_opt(2026, 11, 19).unwrap());
    }
}
