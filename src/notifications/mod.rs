//! Outbound notifications through Telegram and Discord.
//!
//! Both channels are optional and independent. Nothing here is required for the
//! rest of the app to work: with no credentials and every switch off, the scans
//! find nothing to enqueue and the workers never call a provider.

pub mod config;
pub mod dispatch;
pub mod error;
pub mod message;
pub mod provider;
pub mod rate_limit;
pub mod scan;
pub mod validate;
