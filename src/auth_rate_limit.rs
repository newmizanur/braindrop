//! Small fixed-window rate limits for authentication endpoints.
//!
//! Loco 1.3.1 does not ship an authentication rate-limiting middleware. The
//! counters live in Loco's in-process cache, so they are shared by requests in
//! one process and reset on restart. A reverse proxy must still limit requests
//! when more than one replica is used; BrainDrop supports one replica only.

use std::{sync::OnceLock, time::Duration};

use axum::http::StatusCode;
use loco_rs::{controller::ErrorDetail, prelude::*};
use tokio::sync::Mutex;

/// Authentication endpoint being guarded.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Endpoint {
    Login,
    Register,
    Forgot,
    Reset,
    Verify,
    MagicLink,
    MagicLinkVerify,
    ResendVerification,
}

impl Endpoint {
    fn key(self) -> &'static str {
        match self {
            Self::Login => "login",
            Self::Register => "register",
            Self::Forgot => "forgot",
            Self::Reset => "reset",
            Self::Verify => "verify",
            Self::MagicLink => "magic-link",
            Self::MagicLinkVerify => "magic-link-verify",
            Self::ResendVerification => "resend-verification",
        }
    }

    /// Attempts allowed per account (or token) in one window.
    fn account_limit(self) -> u32 {
        match self {
            Self::Login => 10,
            Self::Register | Self::Forgot | Self::MagicLink | Self::ResendVerification => 5,
            Self::Reset | Self::Verify | Self::MagicLinkVerify => 15,
        }
    }

    /// Attempts allowed per client in one window. Deliberately looser than the
    /// per-account limit so users behind one NAT or proxy are not blocked.
    fn client_limit(self) -> u32 {
        match self {
            Self::Login | Self::Reset | Self::Verify | Self::MagicLinkVerify => 100,
            Self::Register | Self::Forgot | Self::MagicLink | Self::ResendVerification => 60,
        }
    }

    fn window_seconds(self) -> i64 {
        match self {
            Self::Login | Self::Reset | Self::Verify | Self::MagicLinkVerify => 300,
            Self::Register | Self::Forgot | Self::MagicLink | Self::ResendVerification => 900,
        }
    }

    /// Token endpoints leave one final attempt inside the window so a correct
    /// token is never held hostage by earlier failures from the same client.
    /// Login counts only failures: a correct password is checked first and
    /// clears its counter, so a known email cannot be used to lock an account.
    fn allows_recovery_attempt(self) -> bool {
        matches!(self, Self::Reset | Self::Verify | Self::MagicLinkVerify)
    }
}

/// Loco's in-process cache has no atomic increment. A process-wide lock makes
/// check-and-increment atomic for this single-replica deployment; without it,
/// parallel requests could all read the same count and pass the limit.
fn rate_lock() -> &'static Mutex<()> {
    static LOCK: OnceLock<Mutex<()>> = OnceLock::new();
    LOCK.get_or_init(|| Mutex::new(()))
}

fn normalize(value: &str) -> String {
    value
        .trim()
        .to_ascii_lowercase()
        .chars()
        .take(256)
        .collect()
}

fn client_key(remote_ip: &RemoteIP) -> String {
    match remote_ip {
        RemoteIP::Forwarded(ip) | RemoteIP::Socket(ip) => ip.to_string(),
        RemoteIP::None => "unknown".to_string(),
    }
}

fn account_dimension(endpoint: Endpoint, identity: &str, client: &str) -> String {
    let identity = normalize(identity);
    if endpoint == Endpoint::Login {
        // A known email must not let a distributed attacker lock an account for
        // everyone. Per-account limits are therefore account + client for
        // login; the separate client budget still catches one noisy client.
        format!("{identity}:{client}")
    } else {
        identity
    }
}

fn current_window(endpoint: Endpoint) -> i64 {
    chrono::Utc::now()
        .timestamp()
        .div_euclid(endpoint.window_seconds())
}

fn cache_key(endpoint: Endpoint, dimension: &str, window: i64) -> String {
    format!("auth:rate:{}:{}:{}", endpoint.key(), dimension, window)
}

/// Record one attempt against both the account/token and the client budget.
///
/// # Errors
///
/// Returns a 429 when either budget is exhausted. Cache failures fail open so
/// a cache outage cannot lock every user out of authentication.
pub async fn check(
    ctx: &AppContext,
    endpoint: Endpoint,
    remote_ip: &RemoteIP,
    identity: &str,
) -> Result<()> {
    let client = client_key(remote_ip);
    let _guard = rate_lock().lock().await;

    check_counter(
        ctx,
        endpoint,
        &format!("account:{}", account_dimension(endpoint, identity, &client)),
        endpoint.account_limit(),
    )
    .await?;
    check_counter(
        ctx,
        endpoint,
        &format!("client:{client}"),
        endpoint.client_limit(),
    )
    .await
}

/// Enforce the client-wide budget without consuming the account failure budget.
/// Login calls this before database lookup and password hashing; account
/// failures remain counted only after a failed credential check.
pub async fn check_client(
    ctx: &AppContext,
    endpoint: Endpoint,
    remote_ip: &RemoteIP,
) -> Result<()> {
    let client = client_key(remote_ip);
    let _guard = rate_lock().lock().await;
    check_counter(
        ctx,
        endpoint,
        &format!("client:{client}"),
        endpoint.client_limit(),
    )
    .await
}

/// Record a failed login against its account dimension after the client
/// precheck has already consumed the client budget.
pub async fn check_account(
    ctx: &AppContext,
    endpoint: Endpoint,
    remote_ip: &RemoteIP,
    identity: &str,
) -> Result<()> {
    let client = client_key(remote_ip);
    let _guard = rate_lock().lock().await;
    check_counter(
        ctx,
        endpoint,
        &format!("account:{}", account_dimension(endpoint, identity, &client)),
        endpoint.account_limit(),
    )
    .await
}

/// Clear the account's failed-login counter after a successful authentication.
/// A correct password must never be held hostage by earlier wrong attempts
/// from the same client. Cache failures are ignored deliberately.
pub async fn reset(ctx: &AppContext, endpoint: Endpoint, remote_ip: &RemoteIP, identity: &str) {
    let client = client_key(remote_ip);
    let _guard = rate_lock().lock().await;
    let key = cache_key(
        endpoint,
        &format!("account:{}", account_dimension(endpoint, identity, &client)),
        current_window(endpoint),
    );
    let _ = ctx.cache.remove(&key).await;
}

async fn check_counter(
    ctx: &AppContext,
    endpoint: Endpoint,
    dimension: &str,
    limit: u32,
) -> Result<()> {
    let key = cache_key(endpoint, dimension, current_window(endpoint));

    let used = ctx
        .cache
        .get::<u32>(&key)
        .await
        .ok()
        .flatten()
        .unwrap_or_default();
    let effective_limit = if endpoint.allows_recovery_attempt() {
        limit.saturating_add(1)
    } else {
        limit
    };
    if used >= effective_limit {
        return Err(Error::CustomError(
            StatusCode::TOO_MANY_REQUESTS,
            ErrorDetail::new("rate_limited", "Too many attempts. Please try again later."),
        ));
    }

    let _ = ctx
        .cache
        .insert_with_expiry(
            &key,
            &(used + 1),
            Duration::from_secs((endpoint.window_seconds() * 2) as u64),
        )
        .await;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::net::IpAddr;

    #[test]
    fn endpoints_have_ordered_budgets() {
        assert!(Endpoint::Login.account_limit() < Endpoint::Login.client_limit());
        assert!(Endpoint::Forgot.account_limit() < Endpoint::Forgot.client_limit());
        assert!(Endpoint::Login.window_seconds() < Endpoint::Forgot.window_seconds());
    }

    #[test]
    fn login_budget_is_isolated_per_client() {
        let first = client_key(&RemoteIP::Socket("127.0.0.1".parse::<IpAddr>().unwrap()));
        let second = client_key(&RemoteIP::Socket("127.0.0.2".parse::<IpAddr>().unwrap()));
        assert_ne!(
            account_dimension(Endpoint::Login, " User@Example.com ", &first),
            account_dimension(Endpoint::Login, "user@example.com", &second)
        );
        assert_eq!(
            account_dimension(Endpoint::Forgot, " User@Example.com ", &first),
            account_dimension(Endpoint::Forgot, "user@example.com", &second)
        );
    }

    #[test]
    fn identity_keys_are_bounded() {
        let identity = "x".repeat(8_192);
        let dimension = account_dimension(Endpoint::Register, &identity, "127.0.0.1");
        assert!(dimension.chars().count() <= 256);
    }
}
