//! Validation for the only two outbound destinations BrainDrop supports.
//!
//! Discord webhooks are user-supplied, so they are the one place where an
//! attacker could try to make the server call an internal address. The check is
//! deliberately strict: HTTPS, an official Discord host, no port, no userinfo,
//! no query and exactly `/api/webhooks/<id>/<token>`. Redirects are disabled by
//! the HTTP client as well.

use reqwest::Url;

use crate::notifications::error::NotificationError;

/// Hosts Discord itself uses for webhook URLs.
const DISCORD_HOSTS: &[&str] = &[
    "discord.com",
    "discordapp.com",
    "ptb.discord.com",
    "canary.discord.com",
];

/// The official Telegram Bot API host.
pub const TELEGRAM_HOST: &str = "api.telegram.org";

/// Validate and normalize a Discord webhook URL.
///
/// # Errors
///
/// Returns [`NotificationError::UnsafeUrl`] when the URL is not an official
/// HTTPS Discord webhook URL.
pub fn discord_webhook(value: &str) -> Result<Url, NotificationError> {
    let value = value.trim();
    let url = Url::parse(value)
        .map_err(|_| NotificationError::UnsafeUrl("the webhook is not a valid URL".to_string()))?;

    if url.scheme() != "https" {
        return Err(NotificationError::UnsafeUrl(
            "the webhook must use https".to_string(),
        ));
    }
    if url.port().is_some() {
        return Err(NotificationError::UnsafeUrl(
            "the webhook must not specify a port".to_string(),
        ));
    }
    if !url.username().is_empty() || url.password().is_some() {
        return Err(NotificationError::UnsafeUrl(
            "the webhook must not contain credentials".to_string(),
        ));
    }
    if url.query().is_some() || url.fragment().is_some() {
        return Err(NotificationError::UnsafeUrl(
            "the webhook must not contain a query or fragment".to_string(),
        ));
    }

    let host = url
        .host_str()
        .ok_or_else(|| NotificationError::UnsafeUrl("the webhook has no host".to_string()))?;
    if !DISCORD_HOSTS.contains(&host) {
        return Err(NotificationError::UnsafeUrl(
            "the webhook host is not an official Discord host".to_string(),
        ));
    }

    let segments: Vec<&str> = url
        .path_segments()
        .map(|segments| segments.filter(|segment| !segment.is_empty()).collect())
        .unwrap_or_default();
    if segments.len() != 4 || segments[0] != "api" || segments[1] != "webhooks" {
        return Err(NotificationError::UnsafeUrl(
            "the webhook path must be /api/webhooks/<id>/<token>".to_string(),
        ));
    }

    let id = segments[2];
    let token = segments[3];
    if id.is_empty()
        || id.len() > 32
        || !id.bytes().all(|byte| byte.is_ascii_digit())
        || token.len() < 16
        || token.len() > 256
        || !token
            .bytes()
            .all(|byte| byte.is_ascii_alphanumeric() || byte == b'_' || byte == b'-')
    {
        return Err(NotificationError::UnsafeUrl(
            "the webhook identifier or token is malformed".to_string(),
        ));
    }

    Ok(url)
}

/// Validate a Telegram bot token. The token is never echoed back.
///
/// # Errors
///
/// Returns [`NotificationError::InvalidInput`] when the token does not look
/// like `<digits>:<secret>`.
pub fn telegram_bot_token(value: &str) -> Result<&str, NotificationError> {
    let value = value.trim();
    if value.is_empty() {
        return Err(NotificationError::InvalidInput(
            "the Telegram bot token must not be empty".to_string(),
        ));
    }
    if value.chars().count() > 256 {
        return Err(NotificationError::InvalidInput(
            "the Telegram bot token is too long".to_string(),
        ));
    }
    let Some((id, secret)) = value.split_once(':') else {
        return Err(NotificationError::InvalidInput(
            "the Telegram bot token must look like <id>:<secret>".to_string(),
        ));
    };
    if id.len() < 5
        || id.len() > 20
        || !id.bytes().all(|byte| byte.is_ascii_digit())
        || secret.len() < 10
        || secret.len() > 200
        || !secret
            .bytes()
            .all(|byte| byte.is_ascii_alphanumeric() || byte == b'_' || byte == b'-')
    {
        return Err(NotificationError::InvalidInput(
            "the Telegram bot token is malformed".to_string(),
        ));
    }
    Ok(value)
}

/// Validate a Telegram chat id: a numeric id (optionally negative for groups)
/// or an `@channel` username.
///
/// # Errors
///
/// Returns [`NotificationError::InvalidInput`] for empty, control or malformed
/// values.
pub fn telegram_chat_id(value: &str) -> Result<&str, NotificationError> {
    let value = value.trim();
    if value.is_empty() {
        return Err(NotificationError::InvalidInput(
            "the Telegram chat id must not be empty".to_string(),
        ));
    }
    if value.chars().count() > 64 || value.chars().any(char::is_control) {
        return Err(NotificationError::InvalidInput(
            "the Telegram chat id is invalid".to_string(),
        ));
    }

    let valid = if let Some(username) = value.strip_prefix('@') {
        username.len() >= 5
            && username.len() <= 64
            && username
                .bytes()
                .all(|byte| byte.is_ascii_alphanumeric() || byte == b'_')
    } else {
        let digits = value.strip_prefix('-').unwrap_or(value);
        !digits.is_empty() && digits.len() <= 32 && digits.bytes().all(|byte| byte.is_ascii_digit())
    };

    if valid {
        Ok(value)
    } else {
        Err(NotificationError::InvalidInput(
            "the Telegram chat id must be numeric or an @username".to_string(),
        ))
    }
}

/// Validate the configured Telegram API root. It must be the official host over
/// HTTPS so a misconfiguration cannot redirect bot tokens elsewhere.
///
/// # Errors
///
/// Returns [`NotificationError::NotConfigured`] when the base is not the
/// official HTTPS endpoint.
pub fn telegram_api_base(value: &str) -> Result<Url, NotificationError> {
    let value = value.trim().trim_end_matches('/');
    let url = Url::parse(value).map_err(|_| {
        NotificationError::NotConfigured("the Telegram API base is not a valid URL".to_string())
    })?;
    if url.scheme() != "https"
        || url.host_str() != Some(TELEGRAM_HOST)
        || url.port().is_some()
        || url.path() != "/"
        || url.query().is_some()
        || url.fragment().is_some()
        || !url.username().is_empty()
        || url.password().is_some()
    {
        return Err(NotificationError::NotConfigured(
            "the Telegram API base must be https://api.telegram.org".to_string(),
        ));
    }
    Ok(url)
}

#[cfg(test)]
mod tests {
    use super::*;

    const WEBHOOK: &str =
        "https://discord.com/api/webhooks/123456789012345678/abcdefghijklmnopqrstuvwxyz0123456789ABCD";

    #[test]
    fn accepts_an_official_webhook() {
        let url = discord_webhook(WEBHOOK).unwrap();
        assert_eq!(url.host_str(), Some("discord.com"));
        assert!(discord_webhook(&WEBHOOK.replace("discord.com", "discordapp.com")).is_ok());
    }

    #[test]
    fn rejects_ssrf_and_unsafe_webhooks() {
        for value in [
            "http://discord.com/api/webhooks/123456789012345678/abcdefghijklmnopqrstuvwxyz0123456789ABCD",
            "https://127.0.0.1/api/webhooks/123456789012345678/abcdefghijklmnopqrstuvwxyz0123456789ABCD",
            "https://localhost/api/webhooks/123456789012345678/abcdefghijklmnopqrstuvwxyz0123456789ABCD",
            "https://discord.com.evil.example/api/webhooks/123456789012345678/abcdefghijklmnopqrstuvwxyz0123456789ABCD",
            "https://discord.com:8443/api/webhooks/123456789012345678/abcdefghijklmnopqrstuvwxyz0123456789ABCD",
            "https://user:pass@discord.com/api/webhooks/123456789012345678/abcdefghijklmnopqrstuvwxyz0123456789ABCD",
            "https://discord.com/api/webhooks/123456789012345678/abcdefghijklmnopqrstuvwxyz0123456789ABCD?x=1",
            "https://discord.com/api/webhooks/123456789012345678/abcdefghijklmnopqrstuvwxyz0123456789ABCD#frag",
            "https://discord.com/not-a-webhook/123/abcdefghijklmnopqrstuvwxyz0123456789ABCD",
            "https://discord.com/api/webhooks/not-a-number/abcdefghijklmnopqrstuvwxyz0123456789ABCD",
            "https://discord.com/api/webhooks/123456789012345678/short",
            "not a url",
        ] {
            assert!(
                discord_webhook(value).is_err(),
                "should reject {value}"
            );
        }
    }

    #[test]
    fn validates_telegram_tokens_and_chat_ids() {
        assert_eq!(
            telegram_bot_token("123456789:abcdefghijklmnopqrstuvwxyz").unwrap(),
            "123456789:abcdefghijklmnopqrstuvwxyz"
        );
        assert!(telegram_bot_token("123456789").is_err());
        assert!(telegram_bot_token("123:short").is_err());
        assert!(telegram_bot_token("abc:abcdefghijklmnop").is_err());

        assert_eq!(
            telegram_chat_id("-1001234567890").unwrap(),
            "-1001234567890"
        );
        assert_eq!(telegram_chat_id("@my_channel").unwrap(), "@my_channel");
        assert!(telegram_chat_id("").is_err());
        assert!(telegram_chat_id("not a chat").is_err());
        assert!(telegram_chat_id("@ab").is_err());
    }

    #[test]
    fn only_the_official_telegram_base_is_allowed() {
        assert!(telegram_api_base("https://api.telegram.org").is_ok());
        assert!(telegram_api_base("https://api.telegram.org/").is_ok());
        assert!(telegram_api_base("http://api.telegram.org").is_err());
        assert!(telegram_api_base("https://api.telegram.org.evil.example").is_err());
        assert!(telegram_api_base("https://127.0.0.1").is_err());
    }
}
