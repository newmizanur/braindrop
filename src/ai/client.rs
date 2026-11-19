//! A small OpenRouter client built on `reqwest`.
//!
//! Only chat completions with a JSON-schema response are used. There is no
//! streaming, tool calling, embedding or vector-store support.

use std::{
    collections::HashMap,
    sync::{Mutex, OnceLock},
    time::Duration,
};

use async_trait::async_trait;
use reqwest::{Client, StatusCode};
use serde::{Deserialize, Serialize};
use serde_json::Value;

use crate::ai::{
    availability::ResolvedAi,
    error::AiError,
    prompt::{response_schema, ChatMessage},
};

fn shared_http_client(timeout: Duration) -> Result<Client, AiError> {
    static CLIENTS: OnceLock<Mutex<HashMap<u64, Client>>> = OnceLock::new();
    let timeout_seconds = timeout.as_secs();
    let clients = CLIENTS.get_or_init(|| Mutex::new(HashMap::new()));
    let mut clients = clients
        .lock()
        .map_err(|_| AiError::Provider("HTTP client cache is unavailable".to_string()))?;
    if let Some(client) = clients.get(&timeout_seconds) {
        return Ok(client.clone());
    }
    let client = Client::builder()
        .timeout(timeout)
        .connect_timeout(Duration::from_secs(10))
        .redirect(reqwest::redirect::Policy::none())
        .user_agent(concat!("BrainDrop/", env!("CARGO_PKG_VERSION")))
        .build()
        .map_err(|_| AiError::Provider("could not create the HTTP client".to_string()))?;
    clients.insert(timeout_seconds, client.clone());
    Ok(client)
}

/// One extraction request. The provider owns the HTTP details; the prompt and
/// schema are supplied by the caller so they can be tested without a network.
#[derive(Clone, Debug)]
pub struct ExtractionRequest {
    pub messages: Vec<ChatMessage>,
    pub schema: Value,
}

/// The seam between the application and the provider.
#[async_trait]
pub trait AiProvider: Send + Sync {
    /// Send the extraction request and return the model's content.
    ///
    /// # Errors
    ///
    /// Returns a typed `AiError` for configuration, transport, provider and
    /// response-shape failures.
    async fn extract(&self, request: &ExtractionRequest) -> Result<String, AiError>;
}

/// An OpenRouter-compatible chat-completions client.
pub struct OpenRouterClient {
    http: Client,
    api_key: String,
    model: String,
    endpoint: String,
    max_output_bytes: usize,
    max_items: usize,
}

impl OpenRouterClient {
    /// Build a client from resolved credentials.
    ///
    /// # Errors
    ///
    /// Returns an error when the HTTP client cannot be created.
    pub fn new(resolved: &ResolvedAi) -> Result<Self, AiError> {
        // Client instances are shared by timeout setting, while API keys and
        // request endpoints remain per-provider and never enter the cache.
        let http = shared_http_client(resolved.timeout)?;

        Ok(Self {
            http,
            api_key: resolved.api_key.clone(),
            model: resolved.model.clone(),
            endpoint: format!(
                "{}/chat/completions",
                resolved.base_url.trim_end_matches('/')
            ),
            max_output_bytes: resolved.max_output_chars,
            max_items: resolved.max_items,
        })
    }

    async fn send(&self, body: &ChatRequest, allow_fallback: bool) -> Result<String, AiError> {
        let mut response = self
            .http
            .post(&self.endpoint)
            .bearer_auth(&self.api_key)
            .json(body)
            .send()
            .await
            .map_err(map_reqwest)?;

        let status = response.status();
        // Read incrementally so an oversized or malicious response is rejected
        // before it is buffered in full.
        let mut bytes = Vec::new();
        while let Some(chunk) = response.chunk().await.map_err(map_reqwest)? {
            if bytes.len() + chunk.len() > self.max_output_bytes {
                return Err(AiError::TooLarge(
                    "the provider response exceeded the configured size".to_string(),
                ));
            }
            bytes.extend_from_slice(&chunk);
        }
        let text = String::from_utf8(bytes).map_err(|_| {
            AiError::InvalidResponse("the provider response was not UTF-8".to_string())
        })?;

        if status == StatusCode::UNAUTHORIZED || status == StatusCode::FORBIDDEN {
            return Err(AiError::Authentication);
        }
        if status == StatusCode::TOO_MANY_REQUESTS {
            return Err(AiError::ProviderRateLimited);
        }
        if !status.is_success() {
            if (status == StatusCode::BAD_REQUEST || status == StatusCode::UNPROCESSABLE_ENTITY)
                && allow_fallback
                && looks_like_unsupported_structured_output(&provider_error_message(&text))
            {
                return Err(AiError::UnsupportedStructuredOutput);
            }
            return Err(AiError::Provider(format!("HTTP {}", status.as_u16())));
        }

        parse_completion(&text, self.max_output_bytes)
    }
}

#[async_trait]
impl AiProvider for OpenRouterClient {
    async fn extract(&self, request: &ExtractionRequest) -> Result<String, AiError> {
        let mut body = ChatRequest {
            model: self.model.clone(),
            messages: request.messages.clone(),
            temperature: 0.1,
            max_tokens: max_tokens(self.max_items),
            response_format: Some(ResponseFormat {
                kind: "json_schema",
                json_schema: JsonSchema {
                    name: "braindrop_items",
                    strict: true,
                    schema: request.schema.clone(),
                },
            }),
        };

        match self.send(&body, true).await {
            // Some models are chat-completions compatible but reject
            // `response_format`. The system prompt still demands JSON, so one
            // retry without the field is safe and keeps the feature usable.
            Err(AiError::UnsupportedStructuredOutput) => {
                body.response_format = None;
                self.send(&body, false).await
            }
            result => result,
        }
    }
}

#[derive(Debug, Serialize)]
struct ChatRequest {
    model: String,
    messages: Vec<ChatMessage>,
    temperature: f32,
    max_tokens: u32,
    #[serde(skip_serializing_if = "Option::is_none")]
    response_format: Option<ResponseFormat>,
}

#[derive(Debug, Serialize)]
struct ResponseFormat {
    #[serde(rename = "type")]
    kind: &'static str,
    json_schema: JsonSchema,
}

#[derive(Debug, Serialize)]
struct JsonSchema {
    name: &'static str,
    strict: bool,
    schema: Value,
}

#[derive(Debug, Deserialize)]
struct CompletionEnvelope {
    #[serde(default)]
    choices: Vec<CompletionChoice>,
    #[serde(default)]
    error: Option<ProviderError>,
}

#[derive(Debug, Deserialize)]
struct CompletionChoice {
    message: CompletionMessage,
}

#[derive(Debug, Deserialize)]
struct CompletionMessage {
    #[serde(default)]
    content: Option<String>,
}

#[derive(Debug, Deserialize)]
struct ProviderError {
    #[serde(default)]
    code: Option<Value>,
    #[serde(default)]
    message: Option<String>,
}

fn parse_completion(text: &str, max_output_bytes: usize) -> Result<String, AiError> {
    let envelope: CompletionEnvelope = serde_json::from_str(text).map_err(|_| {
        AiError::InvalidResponse("the provider envelope was not valid JSON".to_string())
    })?;

    if let Some(error) = envelope.error {
        let code = error
            .code
            .map(|code| code.to_string())
            .unwrap_or_else(|| "unknown".to_string());
        return Err(AiError::Provider(format!("provider error {code}")));
    }

    let content = envelope
        .choices
        .into_iter()
        .next()
        .and_then(|choice| choice.message.content)
        .ok_or_else(|| {
            AiError::InvalidResponse("the provider returned no message content".to_string())
        })?;
    if content.len() > max_output_bytes {
        return Err(AiError::TooLarge(
            "the provider message exceeded the configured size".to_string(),
        ));
    }
    Ok(content)
}

fn provider_error_message(text: &str) -> String {
    #[derive(Deserialize)]
    struct Envelope {
        #[serde(default)]
        error: Option<ProviderError>,
    }

    serde_json::from_str::<Envelope>(text)
        .ok()
        .and_then(|envelope| envelope.error)
        .and_then(|error| error.message)
        .unwrap_or_default()
}

fn looks_like_unsupported_structured_output(message: &str) -> bool {
    let message = message.to_ascii_lowercase();
    [
        "response_format",
        "json_schema",
        "structured output",
        "structured outputs",
        "does not support",
    ]
    .iter()
    .any(|needle| message.contains(needle))
}

fn max_tokens(max_items: usize) -> u32 {
    (max_items as u32).saturating_mul(250).clamp(512, 4_096)
}

fn map_reqwest(error: reqwest::Error) -> AiError {
    if error.is_timeout() {
        AiError::Timeout
    } else if error.is_connect() {
        AiError::Provider("could not connect to the provider".to_string())
    } else if error.is_body() || error.is_decode() {
        AiError::InvalidResponse("could not read the provider response".to_string())
    } else {
        AiError::Provider("the provider request failed".to_string())
    }
}

/// Build an extraction request from local inputs. This is the function the
/// controller calls, and it is also the unit-test seam for validation.
///
/// # Errors
///
/// Returns the provider or validation error.
pub async fn extract_items(
    provider: &dyn AiProvider,
    config: &crate::ai::config::AiSettings,
    text: &str,
    timezone: chrono_tz::Tz,
    now: chrono::DateTime<chrono::Utc>,
) -> Result<Vec<crate::dtos::ai::AiSuggestion>, AiError> {
    let messages = crate::ai::prompt::build_messages(text, timezone, now);
    let schema = response_schema(config.item_limit());
    // The HTTP client also enforces a timeout; wrapping the provider call here
    // bounds any provider implementation, including a test double.
    let content = tokio::time::timeout(
        config.timeout(),
        provider.extract(&ExtractionRequest { messages, schema }),
    )
    .await
    .map_err(|_| AiError::Timeout)??;
    crate::ai::suggestions::parse(&content, timezone, config.item_limit())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_a_normal_completion() {
        let text = r#"{"choices":[{"message":{"content":"{\"items\":[]}"}}]}"#;
        assert_eq!(parse_completion(text, 1_000).unwrap(), r#"{"items":[]}"#);
    }

    #[test]
    fn rejects_an_error_envelope_and_missing_content() {
        assert!(parse_completion(r#"{"error":{"code":500,"message":"boom"}}"#, 1_000).is_err());
        assert!(parse_completion(r#"{"choices":[]}"#, 1_000).is_err());
        assert!(parse_completion("not json", 1_000).is_err());
    }

    #[test]
    fn rejects_an_oversized_message() {
        let text = r#"{"choices":[{"message":{"content":"123456789"}}]}"#;
        assert!(matches!(
            parse_completion(text, 4),
            Err(AiError::TooLarge(_))
        ));
    }

    #[test]
    fn detects_unsupported_structured_output_errors() {
        assert!(looks_like_unsupported_structured_output(
            "response_format is not supported for this model"
        ));
        assert!(looks_like_unsupported_structured_output(
            "this model does not support json_schema"
        ));
        assert!(!looks_like_unsupported_structured_output("bad API key"));
    }

    #[test]
    fn max_tokens_stays_bounded() {
        assert_eq!(max_tokens(1), 512);
        assert_eq!(max_tokens(20), 4_096);
        assert_eq!(max_tokens(50), 4_096);
    }

    // The tests below drive the real client against a local axum server, so
    // the HTTP status mapping, the retry and the timeout are exercised end to
    // end without any external network or credentials.
    use std::{
        future::Future,
        pin::Pin,
        sync::{
            atomic::{AtomicUsize, Ordering},
            Arc, Mutex,
        },
        time::Duration,
    };

    use axum::{extract::State, http::StatusCode, response::IntoResponse, routing::post, Router};
    use serde_json::json;
    use tokio::net::TcpListener;

    type Reply = (StatusCode, String);
    type Responder =
        Arc<dyn Fn(usize) -> Pin<Box<dyn Future<Output = Reply> + Send>> + Send + Sync>;

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
            .route("/chat/completions", post(mock_handler))
            .with_state(state);
        let listener = TcpListener::bind("127.0.0.1:0").await.unwrap();
        let address = listener.local_addr().unwrap();
        tokio::spawn(async move {
            axum::serve(listener, app).await.unwrap();
        });
        (format!("http://{address}"), calls, bodies)
    }

    fn resolved(base_url: String, timeout: Duration) -> ResolvedAi {
        ResolvedAi {
            api_key: "sk-test".to_string(),
            model: "test/model".to_string(),
            base_url,
            timeout,
            max_output_chars: 10_000,
            max_items: 20,
            source: crate::ai::availability::AiCredentialSource::Server,
        }
    }

    fn extraction_request() -> ExtractionRequest {
        ExtractionRequest {
            messages: Vec::new(),
            schema: json!({ "type": "object" }),
        }
    }

    fn completion(content: &str) -> String {
        json!({ "choices": [{ "message": { "content": content } }] }).to_string()
    }

    #[tokio::test]
    async fn client_returns_the_message_content() {
        let (base_url, calls, _) = spawn_mock(Arc::new(|_| {
            Box::pin(async { (StatusCode::OK, completion(r#"{"items":[]}"#)) })
        }))
        .await;

        let client = OpenRouterClient::new(&resolved(base_url, Duration::from_secs(5))).unwrap();
        let content = client.extract(&extraction_request()).await.unwrap();

        assert_eq!(content, r#"{"items":[]}"#);
        assert_eq!(calls.load(Ordering::SeqCst), 1);
    }

    #[tokio::test]
    async fn client_maps_authentication_and_rate_limit_errors() {
        let (base_url, _, _) = spawn_mock(Arc::new(|_| {
            Box::pin(async { (StatusCode::UNAUTHORIZED, "{}".to_string()) })
        }))
        .await;
        let client = OpenRouterClient::new(&resolved(base_url, Duration::from_secs(5))).unwrap();
        assert!(matches!(
            client.extract(&extraction_request()).await,
            Err(AiError::Authentication)
        ));

        let (base_url, _, _) = spawn_mock(Arc::new(|_| {
            Box::pin(async { (StatusCode::TOO_MANY_REQUESTS, "{}".to_string()) })
        }))
        .await;
        let client = OpenRouterClient::new(&resolved(base_url, Duration::from_secs(5))).unwrap();
        assert!(matches!(
            client.extract(&extraction_request()).await,
            Err(AiError::ProviderRateLimited)
        ));
    }

    #[tokio::test]
    async fn client_times_out() {
        let (base_url, _, _) = spawn_mock(Arc::new(|_| {
            Box::pin(async {
                tokio::time::sleep(Duration::from_secs(5)).await;
                (StatusCode::OK, completion(r#"{"items":[]}"#))
            })
        }))
        .await;

        let client = OpenRouterClient::new(&resolved(base_url, Duration::from_millis(50))).unwrap();
        assert!(matches!(
            client.extract(&extraction_request()).await,
            Err(AiError::Timeout)
        ));
    }

    #[tokio::test]
    async fn client_retries_without_response_format() {
        let (base_url, calls, bodies) = spawn_mock(Arc::new(|call| {
            Box::pin(async move {
                if call == 0 {
                    (
                        StatusCode::BAD_REQUEST,
                        json!({
                            "error": { "message": "response_format is not supported for this model" }
                        })
                        .to_string(),
                    )
                } else {
                    (StatusCode::OK, completion(r#"{"items":[]}"#))
                }
            })
        }))
        .await;

        let client = OpenRouterClient::new(&resolved(base_url, Duration::from_secs(5))).unwrap();
        let content = client.extract(&extraction_request()).await.unwrap();

        assert_eq!(content, r#"{"items":[]}"#);
        assert_eq!(calls.load(Ordering::SeqCst), 2);
        let bodies = bodies.lock().unwrap();
        assert!(bodies[0].contains("response_format"));
        assert!(!bodies[1].contains("response_format"));
    }

    #[tokio::test]
    async fn client_rejects_malformed_oversized_and_error_responses() {
        let (base_url, _, _) = spawn_mock(Arc::new(|_| {
            Box::pin(async { (StatusCode::OK, "not json".to_string()) })
        }))
        .await;
        let client = OpenRouterClient::new(&resolved(base_url, Duration::from_secs(5))).unwrap();
        assert!(matches!(
            client.extract(&extraction_request()).await,
            Err(AiError::InvalidResponse(_))
        ));

        let (base_url, _, _) = spawn_mock(Arc::new(|_| {
            Box::pin(async {
                (
                    StatusCode::OK,
                    json!({ "choices": [{ "message": { "content": "x".repeat(20_000) } }] })
                        .to_string(),
                )
            })
        }))
        .await;
        let client = OpenRouterClient::new(&resolved(base_url, Duration::from_secs(5))).unwrap();
        assert!(matches!(
            client.extract(&extraction_request()).await,
            Err(AiError::TooLarge(_))
        ));

        let (base_url, _, _) = spawn_mock(Arc::new(|_| {
            Box::pin(async {
                (
                    StatusCode::OK,
                    json!({ "error": { "code": 500, "message": "boom" } }).to_string(),
                )
            })
        }))
        .await;
        let client = OpenRouterClient::new(&resolved(base_url, Duration::from_secs(5))).unwrap();
        assert!(matches!(
            client.extract(&extraction_request()).await,
            Err(AiError::Provider(_))
        ));
    }
}
