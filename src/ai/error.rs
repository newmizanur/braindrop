//! Errors raised by the optional AI pipeline.
//!
//! The controller turns these into safe HTTP responses. Provider bodies are
//! never returned to the browser and never logged verbatim, so an accidental
//! credential in a provider message cannot leak.

use std::fmt;

/// A failure while resolving credentials, calling OpenRouter or validating
/// its response.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum AiError {
    /// AI is turned off by the server or by the user's account.
    Disabled(String),
    /// AI is enabled but no usable key, model or endpoint is configured.
    NotConfigured(String),
    /// The user's input failed local validation.
    InvalidInput(String),
    /// The per-user request budget was exhausted.
    RateLimited,
    /// The provider asked the caller to slow down.
    ProviderRateLimited,
    /// The provider did not answer before the configured timeout.
    Timeout,
    /// The provider rejected the API key.
    Authentication,
    /// The provider rejected structured outputs; the client retries once
    /// without the `response_format` field.
    UnsupportedStructuredOutput,
    /// The provider returned an HTTP error or an unusable envelope.
    Provider(String),
    /// The provider's content was not the JSON document the contract requires.
    InvalidResponse(String),
    /// The response exceeded the configured size or item limit.
    TooLarge(String),
    /// A personal key could not be encrypted or decrypted.
    Encryption(String),
}

impl fmt::Display for AiError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Disabled(reason) => write!(f, "AI is disabled: {reason}"),
            Self::NotConfigured(reason) => write!(f, "AI is not configured: {reason}"),
            Self::InvalidInput(message) => write!(f, "{message}"),
            Self::RateLimited => write!(f, "per-user AI rate limit reached"),
            Self::ProviderRateLimited => write!(f, "the AI provider is rate limiting requests"),
            Self::Timeout => write!(f, "the AI provider timed out"),
            Self::Authentication => write!(f, "the AI provider rejected the credentials"),
            Self::UnsupportedStructuredOutput => {
                write!(f, "the AI provider does not support structured outputs")
            }
            Self::Provider(message) => write!(f, "the AI provider failed: {message}"),
            Self::InvalidResponse(message) => write!(f, "the AI response was invalid: {message}"),
            Self::TooLarge(message) => write!(f, "the AI response was too large: {message}"),
            Self::Encryption(message) => write!(f, "credential encryption failed: {message}"),
        }
    }
}

impl std::error::Error for AiError {}
