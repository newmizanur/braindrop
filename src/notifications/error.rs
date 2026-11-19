//! Errors raised while preparing or delivering a notification.
//!
//! Provider response bodies are never stored verbatim: the clients reduce them
//! to a status class, and the delivery record keeps a short, credential-free
//! description. The full webhook URL and bot token never appear in an error.

use std::fmt;
use std::time::Duration;

/// A notification failure.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum NotificationError {
    /// Notifications are disabled for the account or the server.
    Disabled(String),
    /// A channel is enabled but its credentials are missing or unusable.
    NotConfigured(String),
    /// The user's input failed local validation.
    InvalidInput(String),
    /// The per-user test-notification budget was exhausted.
    RateLimited,
    /// The provider asked the caller to slow down. `retry_after` comes from the
    /// provider's `Retry-After` header or JSON body when it supplies one.
    ProviderRateLimited { retry_after: Option<Duration> },
    /// The request did not finish before the configured timeout.
    Timeout,
    /// The provider rejected the credential.
    Authentication,
    /// A non-retryable provider error (bad chat id, missing webhook, ...).
    Permanent(String),
    /// A retryable provider or transport error.
    Provider(String),
    /// The provider returned something the client could not use.
    InvalidResponse(String),
    /// A URL is outside the approved provider hosts.
    UnsafeUrl(String),
    /// A stored credential could not be encrypted or decrypted.
    Encryption(String),
}

impl NotificationError {
    /// Whether the delivery worker should try again later.
    #[must_use]
    pub fn is_retryable(&self) -> bool {
        matches!(
            self,
            Self::ProviderRateLimited { .. } | Self::Timeout | Self::Provider(_)
        )
    }

    /// The provider's requested delay, if it sent one.
    #[must_use]
    pub fn retry_after(&self) -> Option<Duration> {
        match self {
            Self::ProviderRateLimited { retry_after } => *retry_after,
            _ => None,
        }
    }
}

impl fmt::Display for NotificationError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Disabled(reason) => write!(f, "notifications are disabled: {reason}"),
            Self::NotConfigured(reason) => write!(f, "notifications are not configured: {reason}"),
            Self::InvalidInput(message) => write!(f, "{message}"),
            Self::RateLimited => write!(f, "per-user notification rate limit reached"),
            Self::ProviderRateLimited { .. } => {
                write!(f, "the notification provider is rate limiting requests")
            }
            Self::Timeout => write!(f, "the notification provider timed out"),
            Self::Authentication => write!(f, "the notification provider rejected the credentials"),
            Self::Permanent(message) => write!(
                f,
                "the notification provider rejected the request: {message}"
            ),
            Self::Provider(message) => write!(f, "the notification provider failed: {message}"),
            Self::InvalidResponse(message) => {
                write!(
                    f,
                    "the notification provider response was invalid: {message}"
                )
            }
            Self::UnsafeUrl(message) => write!(f, "the notification URL is not allowed: {message}"),
            Self::Encryption(message) => write!(f, "credential encryption failed: {message}"),
        }
    }
}

impl std::error::Error for NotificationError {}

/// Turn a database failure into a safe provider error.
///
/// SQLite error strings can name tables, columns and constraints. They are
/// useful in server logs, but they must never reach an API response or a
/// delivery record that the UI displays.
pub fn redacted_db_error<E: fmt::Display>(error: E, operation: &str) -> NotificationError {
    tracing::error!(
        operation,
        error = %error,
        "notification database operation failed"
    );
    NotificationError::Provider("the notification could not be prepared".to_string())
}

impl From<NotificationError> for loco_rs::Error {
    fn from(error: NotificationError) -> Self {
        Self::string(&error.to_string())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn database_errors_are_redacted() {
        let error = redacted_db_error("no such table: private_schema", "find_by_user");
        let message = error.to_string();
        assert!(!message.contains("private_schema"));
        assert!(message.contains("could not be prepared"));
    }
}
