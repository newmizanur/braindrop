//! A small per-user budget for the notification test endpoint.
//!
//! It mirrors the AI limiter: a fixed one-minute window kept in Loco's cache.
//! Cache failures fail open, because a broken limiter must not block the
//! settings page entirely.

use std::time::Duration;

use loco_rs::app::AppContext;

use crate::notifications::error::NotificationError;

const WINDOW_SECONDS: i64 = 60;
const CACHE_TTL_SECONDS: u64 = 120;

/// Record one test attempt for `user_id`, or reject it when the minute is full.
///
/// # Errors
///
/// Returns [`NotificationError::RateLimited`] when the account has used its
/// budget.
pub async fn check(ctx: &AppContext, user_id: i64, limit: u32) -> Result<(), NotificationError> {
    let now = chrono::Utc::now().timestamp();
    let window = now.div_euclid(WINDOW_SECONDS);
    let key = format!("notifications:test:{user_id}:{window}");

    let used = ctx
        .cache
        .get::<u32>(&key)
        .await
        .ok()
        .flatten()
        .unwrap_or_default();
    if used >= limit {
        return Err(NotificationError::RateLimited);
    }

    let _ = ctx
        .cache
        .insert_with_expiry(&key, &(used + 1), Duration::from_secs(CACHE_TTL_SECONDS))
        .await;
    Ok(())
}
