//! The two provider clients and the seam tests use to replace them.
//!
//! Both clients use a bounded timeout, no redirects and a small response-size
//! cap. Telegram messages are plain text; Discord mentions are explicitly
//! disabled so an item title cannot ping `@everyone` or a role.

use std::{
    collections::HashMap,
    fmt,
    sync::{Mutex, OnceLock},
    time::Duration,
};

use async_trait::async_trait;
use reqwest::{Client, StatusCode, Url};
use serde::{Deserialize, Serialize};

use crate::{
    models::notification_deliveries::NotificationChannel,
    notifications::{
        config::NotificationSettings, error::NotificationError, message::OutgoingMessage, validate,
    },
};

const MAX_RESPONSE_BYTES: usize = 16 * 1024;

/// The credentials needed to send through one channel. `Debug` is redacted.
#[derive(Clone)]
pub enum ChannelCredentials {
    Telegram { bot_token: String, chat_id: String },
    Discord { webhook_url: String },
}

impl fmt::Debug for ChannelCredentials {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Telegram { chat_id, .. } => f
                .debug_struct("TelegramCredentials")
                .field("bot_token", &"<redacted>")
                .field("chat_id", chat_id)
                .finish(),
            Self::Discord { .. } => f
                .debug_struct("DiscordCredentials")
                .field("webhook_url", &"<redacted>")
                .finish(),
        }
    }
}

impl ChannelCredentials {
    #[must_use]
    pub fn channel(&self) -> NotificationChannel {
        match self {
            Self::Telegram { .. } => NotificationChannel::Telegram,
            Self::Discord { .. } => NotificationChannel::Discord,
        }
    }
}

/// The provider seam. Normal requests build [`HttpNotificationSender`]; tests
/// install a recording double in the app's shared store.
#[async_trait]
pub trait NotificationSender: Send + Sync {
    /// Send one message through one channel.
    ///
    /// # Errors
    ///
    /// Returns a typed [`NotificationError`]. Credentials never appear in the
    /// error text.
    async fn send(
        &self,
        channel: NotificationChannel,
        credentials: &ChannelCredentials,
        message: &OutgoingMessage,
    ) -> Result<(), NotificationError>;
}

/// The real HTTP sender.
pub struct HttpNotificationSender {
    telegram: TelegramClient,
    discord: DiscordClient,
}

impl HttpNotificationSender {
    /// Build a sender from server configuration.
    ///
    /// # Errors
    ///
    /// Returns an error when the configured Telegram API root is not the
    /// official HTTPS endpoint or when the HTTP client cannot be created.
    pub fn new(config: &NotificationSettings) -> Result<Self, NotificationError> {
        Ok(Self {
            telegram: TelegramClient::new(config)?,
            discord: DiscordClient::new(config)?,
        })
    }
}

#[async_trait]
impl NotificationSender for HttpNotificationSender {
    async fn send(
        &self,
        channel: NotificationChannel,
        credentials: &ChannelCredentials,
        message: &OutgoingMessage,
    ) -> Result<(), NotificationError> {
        if channel != credentials.channel() {
            return Err(NotificationError::NotConfigured(
                "the delivery channel does not match the stored credentials".to_string(),
            ));
        }
        match credentials {
            ChannelCredentials::Telegram { bot_token, chat_id } => {
                self.telegram.send(bot_token, chat_id, &message.text).await
            }
            ChannelCredentials::Discord { webhook_url } => {
                self.discord.send(webhook_url, &message.text).await
            }
        }
    }
}

/// A Telegram Bot API client.
pub struct TelegramClient {
    http: Client,
    endpoint: String,
    max_chars: usize,
}

impl TelegramClient {
    /// Build the official client.
    ///
    /// # Errors
    ///
    /// Returns an error when the base URL is not the official endpoint or the
    /// HTTP client cannot be created.
    pub fn new(config: &NotificationSettings) -> Result<Self, NotificationError> {
        let base = validate::telegram_api_base(config.telegram_base())?;
        Self::with_endpoint(
            base.as_str().trim_end_matches('/').to_string(),
            config.timeout(),
            config.telegram_limit(),
        )
    }

    /// Build a client pointed at an arbitrary endpoint. Used only by unit tests
    /// with a local mock server; production code always goes through `new`.
    fn with_endpoint(
        endpoint: String,
        timeout: Duration,
        max_chars: usize,
    ) -> Result<Self, NotificationError> {
        Ok(Self {
            http: http_client(timeout)?,
            endpoint,
            max_chars,
        })
    }

    async fn send(
        &self,
        bot_token: &str,
        chat_id: &str,
        text: &str,
    ) -> Result<(), NotificationError> {
        let bot_token = validate::telegram_bot_token(bot_token)?;
        let chat_id = validate::telegram_chat_id(chat_id)?;
        let url = format!("{}/bot{bot_token}/sendMessage", self.endpoint);
        let text = crate::notifications::message::truncate(
            &crate::notifications::message::sanitize(text),
            self.max_chars,
        );

        let response = self
            .http
            .post(url)
            .json(&TelegramPayload {
                chat_id,
                text,
                disable_web_page_preview: true,
            })
            .send()
            .await
            .map_err(map_reqwest)?;

        let status = response.status();
        let body = read_capped(response).await?;

        if status == StatusCode::TOO_MANY_REQUESTS {
            return Err(NotificationError::ProviderRateLimited {
                retry_after: telegram_retry_after(&body),
            });
        }
        if status == StatusCode::UNAUTHORIZED {
            return Err(NotificationError::Authentication);
        }
        if status.is_success() {
            let envelope: TelegramEnvelope = serde_json::from_str(&body).map_err(|_| {
                NotificationError::InvalidResponse(
                    "the Telegram response was not valid JSON".to_string(),
                )
            })?;
            if envelope.ok {
                return Ok(());
            }
            return Err(NotificationError::Permanent(sanitize_provider_message(
                envelope.description.as_deref(),
            )));
        }
        if status.is_client_error() {
            let description = serde_json::from_str::<TelegramEnvelope>(&body)
                .ok()
                .and_then(|envelope| envelope.description);
            let message = description.map_or_else(
                || format!("HTTP {}", status.as_u16()),
                |description| sanitize_provider_message(Some(&description)),
            );
            return Err(NotificationError::Permanent(message));
        }
        Err(NotificationError::Provider(format!(
            "HTTP {}",
            status.as_u16()
        )))
    }
}

/// A Discord webhook client.
pub struct DiscordClient {
    http: Client,
    max_chars: usize,
}

impl DiscordClient {
    /// Build the official client.
    ///
    /// # Errors
    ///
    /// Returns an error when the HTTP client cannot be created.
    pub fn new(config: &NotificationSettings) -> Result<Self, NotificationError> {
        Self::with_endpoint(config.timeout(), config.discord_limit())
    }

    fn with_endpoint(timeout: Duration, max_chars: usize) -> Result<Self, NotificationError> {
        Ok(Self {
            http: http_client(timeout)?,
            max_chars,
        })
    }

    async fn send(&self, webhook_url: &str, text: &str) -> Result<(), NotificationError> {
        let url: Url = validate::discord_webhook(webhook_url)?;
        let text = crate::notifications::message::truncate(
            &crate::notifications::message::sanitize(text),
            self.max_chars,
        );

        let response = self
            .http
            .post(url)
            .json(&DiscordPayload {
                content: text,
                allowed_mentions: AllowedMentions { parse: Vec::new() },
            })
            .send()
            .await
            .map_err(map_reqwest)?;

        let status = response.status();
        let retry_after_header = response
            .headers()
            .get(reqwest::header::RETRY_AFTER)
            .and_then(|value| value.to_str().ok())
            .and_then(|value| value.parse::<u64>().ok())
            .map(Duration::from_secs);
        let body = read_capped(response).await?;

        if status == StatusCode::TOO_MANY_REQUESTS {
            return Err(NotificationError::ProviderRateLimited {
                retry_after: discord_retry_after(&body).or(retry_after_header),
            });
        }
        if status == StatusCode::UNAUTHORIZED || status == StatusCode::FORBIDDEN {
            return Err(NotificationError::Authentication);
        }
        if status.is_success() {
            return Ok(());
        }
        if status.is_client_error() {
            return Err(NotificationError::Permanent(format!(
                "HTTP {}",
                status.as_u16()
            )));
        }
        Err(NotificationError::Provider(format!(
            "HTTP {}",
            status.as_u16()
        )))
    }
}

#[derive(Serialize)]
struct TelegramPayload<'a> {
    chat_id: &'a str,
    text: String,
    disable_web_page_preview: bool,
}

#[derive(Deserialize)]
struct TelegramEnvelope {
    #[serde(default)]
    ok: bool,
    #[serde(default)]
    description: Option<String>,
}

#[derive(Serialize)]
struct DiscordPayload {
    content: String,
    allowed_mentions: AllowedMentions,
}

#[derive(Serialize)]
struct AllowedMentions {
    parse: Vec<String>,
}

fn http_client(timeout: Duration) -> Result<Client, NotificationError> {
    static CLIENTS: OnceLock<Mutex<HashMap<u64, Client>>> = OnceLock::new();
    let timeout_seconds = timeout.as_secs();
    let clients = CLIENTS.get_or_init(|| Mutex::new(HashMap::new()));
    let mut clients = clients.lock().map_err(|_| {
        NotificationError::NotConfigured("HTTP client cache is unavailable".to_string())
    })?;
    if let Some(client) = clients.get(&timeout_seconds) {
        return Ok(client.clone());
    }
    let client = Client::builder()
        .timeout(timeout)
        .connect_timeout(Duration::from_secs(5))
        .redirect(reqwest::redirect::Policy::none())
        .user_agent(concat!("BrainDrop/", env!("CARGO_PKG_VERSION")))
        .build()
        .map_err(|_| {
            NotificationError::NotConfigured("could not create the HTTP client".to_string())
        })?;
    clients.insert(timeout_seconds, client.clone());
    Ok(client)
}

async fn read_capped(mut response: reqwest::Response) -> Result<String, NotificationError> {
    let mut bytes = Vec::new();
    while let Some(chunk) = response.chunk().await.map_err(map_reqwest)? {
        if bytes.len() + chunk.len() > MAX_RESPONSE_BYTES {
            return Err(NotificationError::InvalidResponse(
                "the provider response was too large".to_string(),
            ));
        }
        bytes.extend_from_slice(&chunk);
    }
    String::from_utf8(bytes).map_err(|_| {
        NotificationError::InvalidResponse("the provider response was not UTF-8".to_string())
    })
}

fn telegram_retry_after(body: &str) -> Option<Duration> {
    #[derive(Deserialize)]
    struct Envelope {
        #[serde(default)]
        parameters: Option<Parameters>,
    }
    #[derive(Deserialize)]
    struct Parameters {
        #[serde(default)]
        retry_after: Option<u64>,
    }
    serde_json::from_str::<Envelope>(body)
        .ok()
        .and_then(|envelope| envelope.parameters)
        .and_then(|parameters| parameters.retry_after)
        .map(Duration::from_secs)
}

fn discord_retry_after(body: &str) -> Option<Duration> {
    #[derive(Deserialize)]
    struct Envelope {
        #[serde(default)]
        retry_after: Option<f64>,
    }
    serde_json::from_str::<Envelope>(body)
        .ok()
        .and_then(|envelope| envelope.retry_after)
        .filter(|seconds| seconds.is_finite() && *seconds >= 0.0)
        .map(Duration::from_secs_f64)
}

fn sanitize_provider_message(message: Option<&str>) -> String {
    let message = message.unwrap_or("the provider rejected the request");
    let message = crate::notifications::message::sanitize(message);
    crate::notifications::message::truncate(&message, 160)
}

fn map_reqwest(error: reqwest::Error) -> NotificationError {
    if error.is_timeout() {
        NotificationError::Timeout
    } else if error.is_connect() {
        NotificationError::Provider("could not connect to the provider".to_string())
    } else if error.is_body() || error.is_decode() {
        NotificationError::InvalidResponse("could not read the provider response".to_string())
    } else {
        NotificationError::Provider("the provider request failed".to_string())
    }
}

#[cfg(test)]
mod tests {
    use std::{
        pin::Pin,
        sync::{
            atomic::{AtomicUsize, Ordering},
            Arc, Mutex,
        },
    };

    use axum::{extract::State, http::StatusCode, response::IntoResponse, routing::post, Router};
    use serde_json::{json, Value};
    use tokio::net::TcpListener;

    use super::*;

    type Reply = (StatusCode, String);
    type Responder = Arc<
        dyn Fn(usize) -> Pin<Box<dyn std::future::Future<Output = Reply> + Send>> + Send + Sync,
    >;

    #[derive(Clone)]
    struct MockState {
        calls: Arc<AtomicUsize>,
        bodies: Arc<Mutex<Vec<String>>>,
        responder: Responder,
    }

    async fn mock_handler(State(state): State<MockState>, body: String) -> impl IntoResponse {
        state.bodies.lock().unwrap().push(body);
        let call = state.calls.fetch_add(1, Ordering::SeqCst);
        (state.responder)(call).await
    }

    async fn spawn_mock(
        responder: Responder,
    ) -> (String, Arc<AtomicUsize>, Arc<Mutex<Vec<String>>>) {
        let calls = Arc::new(AtomicUsize::new(0));
        let bodies = Arc::new(Mutex::new(Vec::new()));
        let state = MockState {
            calls: calls.clone(),
            bodies: bodies.clone(),
            responder,
        };
        let app = Router::new()
            .route("/bot{*rest}", post(mock_handler))
            .route("/api/webhooks/{*rest}", post(mock_handler))
            .with_state(state);
        let listener = TcpListener::bind("127.0.0.1:0").await.unwrap();
        let address = listener.local_addr().unwrap();
        tokio::spawn(async move {
            axum::serve(listener, app).await.unwrap();
        });
        (format!("http://{address}"), calls, bodies)
    }

    const TOKEN: &str = "123456789:abcdefghijklmnopqrstuvwxyz";
    const CHAT: &str = "-1001234567890";
    const WEBHOOK: &str =
        "https://discord.com/api/webhooks/123456789012345678/abcdefghijklmnopqrstuvwxyz0123456789ABCD";

    fn credentials(channel: NotificationChannel) -> ChannelCredentials {
        match channel {
            NotificationChannel::Telegram => ChannelCredentials::Telegram {
                bot_token: TOKEN.to_string(),
                chat_id: CHAT.to_string(),
            },
            NotificationChannel::Discord => ChannelCredentials::Discord {
                webhook_url: WEBHOOK.to_string(),
            },
        }
    }

    #[tokio::test]
    async fn telegram_sends_plain_text_and_maps_success() {
        let (endpoint, calls, bodies) = spawn_mock(Arc::new(|_| {
            Box::pin(async { (StatusCode::OK, json!({ "ok": true }).to_string()) })
        }))
        .await;
        let client =
            TelegramClient::with_endpoint(endpoint, Duration::from_secs(5), 4_096).unwrap();

        client
            .send(TOKEN, CHAT, "hello <b>world</b>")
            .await
            .unwrap();
        assert_eq!(calls.load(Ordering::SeqCst), 1);
        let body: Value = serde_json::from_str(&bodies.lock().unwrap()[0]).unwrap();
        assert_eq!(body["text"], "hello <b>world</b>");
        assert!(body.get("parse_mode").is_none());
        assert_eq!(body["chat_id"], CHAT);
    }

    #[tokio::test]
    async fn telegram_maps_rate_limits_and_permanent_errors() {
        let (endpoint, _, _) = spawn_mock(Arc::new(|_| {
            Box::pin(async {
                (
                    StatusCode::TOO_MANY_REQUESTS,
                    json!({ "ok": false, "parameters": { "retry_after": 7 } }).to_string(),
                )
            })
        }))
        .await;
        let client =
            TelegramClient::with_endpoint(endpoint, Duration::from_secs(5), 4_096).unwrap();
        let error = client.send(TOKEN, CHAT, "hello").await.unwrap_err();
        assert!(matches!(
            error,
            NotificationError::ProviderRateLimited {
                retry_after: Some(delay)
            } if delay == Duration::from_secs(7)
        ));

        let (endpoint, _, _) = spawn_mock(Arc::new(|_| {
            Box::pin(async {
                (
                    StatusCode::BAD_REQUEST,
                    json!({ "ok": false, "description": "chat not found" }).to_string(),
                )
            })
        }))
        .await;
        let client =
            TelegramClient::with_endpoint(endpoint, Duration::from_secs(5), 4_096).unwrap();
        let error = client.send(TOKEN, CHAT, "hello").await.unwrap_err();
        assert!(
            matches!(error, NotificationError::Permanent(ref message) if message == "chat not found")
        );
        assert!(!error.is_retryable());
    }

    #[tokio::test]
    async fn telegram_rejects_malformed_credentials_before_a_request() {
        let (endpoint, calls, _) = spawn_mock(Arc::new(|_| {
            Box::pin(async { (StatusCode::OK, json!({ "ok": true }).to_string()) })
        }))
        .await;
        let client =
            TelegramClient::with_endpoint(endpoint, Duration::from_secs(5), 4_096).unwrap();
        assert!(client.send("bad-token", CHAT, "hello").await.is_err());
        assert_eq!(calls.load(Ordering::SeqCst), 0);
    }

    #[tokio::test]
    async fn discord_disables_mentions_and_maps_success() {
        // A local mock cannot receive the official host, so exercise the
        // payload and validation separately: validation rejects the local URL,
        // and the sender never reaches the network.
        let client = DiscordClient::with_endpoint(Duration::from_secs(5), 2_000).unwrap();
        let error = client
            .send("http://127.0.0.1/webhook", "hello")
            .await
            .unwrap_err();
        assert!(matches!(error, NotificationError::UnsafeUrl(_)));

        let payload = serde_json::to_value(DiscordPayload {
            content: "hello".to_string(),
            allowed_mentions: AllowedMentions { parse: Vec::new() },
        })
        .unwrap();
        assert_eq!(payload["allowed_mentions"]["parse"], json!([]));
    }

    #[tokio::test]
    async fn http_sender_rejects_mismatched_credentials() {
        let sender = HttpNotificationSender::new(&NotificationSettings::default()).unwrap();
        let error = sender
            .send(
                NotificationChannel::Discord,
                &credentials(NotificationChannel::Telegram),
                &OutgoingMessage::new("hello".to_string()),
            )
            .await
            .unwrap_err();
        assert!(matches!(error, NotificationError::NotConfigured(_)));
    }

    #[test]
    fn debug_output_redacts_credentials() {
        let telegram = credentials(NotificationChannel::Telegram);
        let discord = credentials(NotificationChannel::Discord);
        assert!(!format!("{telegram:?}").contains(TOKEN));
        assert!(!format!("{discord:?}").contains(WEBHOOK));
    }
}
