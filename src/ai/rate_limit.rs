//! A small per-user request budget.
//!
//! The counter lives in Loco's cache, so it is shared by every request the
//! process serves and disappears with the process. A fixed one-minute window
//! keeps the implementation tiny; the cache expiry makes stale windows vanish
//! even if nobody calls again. Failures to reach the cache fail open: a broken
//! rate limiter must not take the whole capture flow down.

use std::time::Duration;

use loco_rs::app::AppContext;

use crate::ai::error::AiError;

const WINDOW_SECONDS: i64 = 60;
const CACHE_TTL_SECONDS: u64 = 120;

/// Record one attempt for `user_id`, or reject it when the minute is full.
///
/// # Errors
///
/// Returns [`AiError::RateLimited`] when the account has used its budget.
pub async fn check(ctx: &AppContext, user_id: i64, limit: u32) -> Result<(), AiError> {
    let now = chrono::Utc::now().timestamp();
    let window = now.div_euclid(WINDOW_SECONDS);
    let key = format!("ai:rate:{user_id}:{window}");

    let used = ctx
        .cache
        .get::<u32>(&key)
        .await
        .ok()
        .flatten()
        .unwrap_or_default();
    if used >= limit {
        return Err(AiError::RateLimited);
    }

    let _ = ctx
        .cache
        .insert_with_expiry(&key, &(used + 1), Duration::from_secs(CACHE_TTL_SECONDS))
        .await;
    Ok(())
}
