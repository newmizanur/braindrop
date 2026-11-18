//! Optional AI-assisted capture.
//!
//! The module is deliberately small: one provider (OpenRouter), one operation
//! (turn free text into reviewable item suggestions), no streaming, no tools
//! and no persistence. All provider traffic is bounded by configuration and
//! every response is validated before it reaches the browser.

pub mod availability;
pub mod client;
pub mod config;
pub mod credentials;
pub mod error;
pub mod prompt;
pub mod rate_limit;
pub mod suggestions;
