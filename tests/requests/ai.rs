use std::{
    sync::{Arc, Mutex},
    time::Duration,
};

use async_trait::async_trait;
use base64::{engine::general_purpose::STANDARD, Engine};
use braindrop::{
    ai::{
        client::{AiProvider, ExtractionRequest},
        credentials::SecretBox,
        error::AiError,
    },
    app::App,
    dtos::ai::{AiParseResponse, AiStatus},
    dtos::settings::SettingsResponse,
    models::user_settings,
};
use loco_rs::{app::AppContext, testing::prelude::*};
use serde_json::json;
use serial_test::serial;

use super::prepare_data;

const ENV_KEYS: &[&str] = &[
    "AI_ENABLED",
    "OPENROUTER_API_KEY",
    "OPENROUTER_MODEL",
    "OPENROUTER_BASE_URL",
    "AI_TIMEOUT_SECONDS",
    "AI_MAX_INPUT_CHARS",
    "AI_RATE_LIMIT_PER_MINUTE",
    "AI_ENCRYPTION_KEY",
];

fn encryption_key() -> String {
    STANDARD.encode([9u8; 32])
}

/// Set AI configuration for one test and restore the environment afterwards.
struct AiEnv;

impl AiEnv {
    fn server() -> Self {
        Self::apply(&[
            ("AI_ENABLED", "true"),
            ("OPENROUTER_API_KEY", "sk-test-server"),
            ("OPENROUTER_MODEL", "test/server-model"),
            ("AI_RATE_LIMIT_PER_MINUTE", "10"),
        ])
    }

    fn without_server_key() -> Self {
        Self::apply(&[
            ("AI_ENABLED", "true"),
            ("OPENROUTER_API_KEY", ""),
            ("OPENROUTER_MODEL", "test/server-model"),
            ("AI_RATE_LIMIT_PER_MINUTE", "10"),
        ])
    }

    fn disabled_server() -> Self {
        Self::apply(&[
            ("AI_ENABLED", "false"),
            ("OPENROUTER_API_KEY", "sk-test-server"),
            ("OPENROUTER_MODEL", "test/server-model"),
        ])
    }

    fn with(vars: &[(&str, &str)]) -> Self {
        Self::apply(vars)
    }

    fn apply(vars: &[(&str, &str)]) -> Self {
        std::env::set_var("OPENROUTER_BASE_URL", "https://openrouter.ai/api/v1");
        std::env::set_var("AI_ENCRYPTION_KEY", encryption_key());
        for (key, value) in vars {
            std::env::set_var(key, value);
        }
        Self
    }
}

impl Drop for AiEnv {
    fn drop(&mut self) {
        for key in ENV_KEYS {
            std::env::remove_var(key);
        }
    }
}

#[derive(Clone)]
struct MockProvider {
    calls: Arc<Mutex<usize>>,
    last_request: Arc<Mutex<Option<ExtractionRequest>>>,
    reply: Arc<dyn Fn() -> Result<String, AiError> + Send + Sync>,
    delay: Option<Duration>,
}

impl MockProvider {
    fn ok(content: &str) -> Self {
        let content = content.to_string();
        Self {
            calls: Arc::new(Mutex::new(0)),
            last_request: Arc::new(Mutex::new(None)),
            reply: Arc::new(move || Ok(content.clone())),
            delay: None,
        }
    }

    fn failing(error: AiError) -> Self {
        Self {
            calls: Arc::new(Mutex::new(0)),
            last_request: Arc::new(Mutex::new(None)),
            reply: Arc::new(move || Err(error.clone())),
            delay: None,
        }
    }

    fn slow(mut self, delay: Duration) -> Self {
        self.delay = Some(delay);
        self
    }

    fn calls(&self) -> usize {
        *self.calls.lock().unwrap()
    }

    fn last_request(&self) -> ExtractionRequest {
        self.last_request
            .lock()
            .unwrap()
            .clone()
            .expect("the provider should have been called")
    }
}

#[async_trait]
impl AiProvider for MockProvider {
    async fn extract(&self, request: &ExtractionRequest) -> Result<String, AiError> {
        *self.calls.lock().unwrap() += 1;
        *self.last_request.lock().unwrap() = Some(request.clone());
        if let Some(delay) = self.delay {
            tokio::time::sleep(delay).await;
        }
        (self.reply)()
    }
}

fn install(ctx: &AppContext, provider: &MockProvider) {
    ctx.shared_store
        .insert::<Arc<dyn AiProvider>>(Arc::new(provider.clone()));
}

fn suggestion(title: &str, due_at: Option<&str>) -> serde_json::Value {
    json!({
        "type": "task",
        "title": title,
        "content": null,
        "category": "work",
        "tags": ["test"],
        "priority": "normal",
        "due_at": due_at,
        "remind_at": null
    })
}

async fn parse(request: &loco_rs::TestServer, token: &str, text: &str) -> (u16, serde_json::Value) {
    let (key, value) = prepare_data::auth_header(token);
    let response = request
        .post("/api/ai/parse")
        .add_header(key, value)
        .json(&json!({ "text": text }))
        .await;
    let status = response.status_code().as_u16();
    let body = serde_json::from_str(&response.text()).unwrap_or(serde_json::Value::Null);
    (status, body)
}

async fn update_settings(
    request: &loco_rs::TestServer,
    token: &str,
    payload: serde_json::Value,
) -> SettingsResponse {
    let (key, value) = prepare_data::auth_header(token);
    let response = request
        .patch("/api/settings")
        .add_header(key, value)
        .json(&payload)
        .await;
    assert_eq!(response.status_code(), 200, "{}", response.text());
    serde_json::from_str(&response.text()).unwrap()
}

#[tokio::test]
#[serial]
async fn ai_parse_returns_multiple_suggestions_and_persists_nothing() {
    let _env = AiEnv::server();

    request::<App, _, _>(|request, ctx| async move {
        let user = prepare_data::init_user_login(&request, &ctx).await;
        let provider = MockProvider::ok(
            &json!({
                "items": [
                    suggestion("Buy milk", Some("2026-12-15T09:00:00+00:00")),
                    suggestion("Renew domain", None)
                ]
            })
            .to_string(),
        );
        install(&ctx, &provider);

        let (status, body) = parse(&request, &user.token, "buy milk tomorrow, renew domain").await;
        assert_eq!(status, 200, "{body}");
        let parsed: AiParseResponse = serde_json::from_value(body).unwrap();
        assert_eq!(parsed.items.len(), 2);
        assert_eq!(parsed.items[0].title, "Buy milk");
        assert_eq!(parsed.items[1].title, "Renew domain");
        assert!(parsed.items[1].due_at.is_none());
        assert_eq!(provider.calls(), 1);

        // The suggestions are reviewable data, not stored items.
        let (key, value) = prepare_data::auth_header(&user.token);
        let response = request.get("/api/items").add_header(key, value).await;
        let page: serde_json::Value = serde_json::from_str(&response.text()).unwrap();
        assert_eq!(page["total_items"], 0);
    })
    .await;
}

#[tokio::test]
#[serial]
async fn ai_parse_rejects_malformed_model_output() {
    let _env = AiEnv::server();

    request::<App, _, _>(|request, ctx| async move {
        let user = prepare_data::init_user_login(&request, &ctx).await;
        let provider =
            MockProvider::ok(&json!({ "items": [{ "title": "missing the rest" }] }).to_string());
        install(&ctx, &provider);

        let (status, body) = parse(&request, &user.token, "something").await;
        assert_eq!(status, 502, "{body}");
        assert_eq!(body["error"], "ai_invalid_response");

        let (key, value) = prepare_data::auth_header(&user.token);
        let response = request.get("/api/items").add_header(key, value).await;
        let page: serde_json::Value = serde_json::from_str(&response.text()).unwrap();
        assert_eq!(page["total_items"], 0);
    })
    .await;
}

#[tokio::test]
#[serial]
async fn ai_parse_maps_provider_authentication_failures() {
    let _env = AiEnv::server();

    request::<App, _, _>(|request, ctx| async move {
        let user = prepare_data::init_user_login(&request, &ctx).await;
        let provider = MockProvider::failing(AiError::Authentication);
        install(&ctx, &provider);

        let (status, body) = parse(&request, &user.token, "something").await;
        assert_eq!(status, 502, "{body}");
        assert_eq!(body["error"], "ai_authentication");
    })
    .await;
}

#[tokio::test]
#[serial]
async fn ai_parse_respects_the_account_switch() {
    let _env = AiEnv::server();

    request::<App, _, _>(|request, ctx| async move {
        let user = prepare_data::init_user_login(&request, &ctx).await;
        let provider = MockProvider::ok(&json!({ "items": [] }).to_string());
        install(&ctx, &provider);

        let settings = update_settings(&request, &user.token, json!({ "ai_enabled": false })).await;
        assert_eq!(settings.ai.status, AiStatus::Disabled);
        assert!(settings.ai.status_reason.unwrap().contains("account"));

        let (status, body) = parse(&request, &user.token, "something").await;
        assert_eq!(status, 409, "{body}");
        assert_eq!(body["error"], "ai_disabled");
        assert_eq!(provider.calls(), 0);
    })
    .await;
}

#[tokio::test]
#[serial]
async fn ai_parse_reports_a_disabled_server() {
    let _env = AiEnv::disabled_server();

    request::<App, _, _>(|request, ctx| async move {
        let user = prepare_data::init_user_login(&request, &ctx).await;
        let provider = MockProvider::ok(&json!({ "items": [] }).to_string());
        install(&ctx, &provider);

        let (status, body) = parse(&request, &user.token, "something").await;
        assert_eq!(status, 409, "{body}");
        assert_eq!(body["error"], "ai_disabled");
        assert_eq!(provider.calls(), 0);
    })
    .await;
}

#[tokio::test]
#[serial]
async fn ai_parse_reports_missing_credentials() {
    let _env = AiEnv::without_server_key();

    request::<App, _, _>(|request, ctx| async move {
        let user = prepare_data::init_user_login(&request, &ctx).await;
        let provider = MockProvider::ok(&json!({ "items": [] }).to_string());
        install(&ctx, &provider);

        let settings =
            update_settings(&request, &user.token, json!({ "ai_use_server_key": false })).await;
        assert_eq!(settings.ai.status, AiStatus::NotConfigured);

        let (status, body) = parse(&request, &user.token, "something").await;
        assert_eq!(status, 409, "{body}");
        assert_eq!(body["error"], "ai_not_configured");
        assert_eq!(provider.calls(), 0);
    })
    .await;
}

#[tokio::test]
#[serial]
async fn ai_parse_rate_limits_per_user() {
    let _env = AiEnv::with(&[
        ("AI_ENABLED", "true"),
        ("OPENROUTER_API_KEY", "sk-test-server"),
        ("OPENROUTER_MODEL", "test/server-model"),
        ("AI_RATE_LIMIT_PER_MINUTE", "1"),
    ]);

    request::<App, _, _>(|request, ctx| async move {
        let user = prepare_data::init_user_login(&request, &ctx).await;
        let provider = MockProvider::ok(&json!({ "items": [] }).to_string());
        install(&ctx, &provider);

        let (first, body) = parse(&request, &user.token, "first").await;
        assert_eq!(first, 200, "{body}");

        let (second, body) = parse(&request, &user.token, "second").await;
        assert_eq!(second, 429, "{body}");
        assert_eq!(body["error"], "ai_rate_limited");
        assert_eq!(provider.calls(), 1);
    })
    .await;
}

#[tokio::test]
#[serial]
async fn ai_parse_times_out() {
    let _env = AiEnv::with(&[
        ("AI_ENABLED", "true"),
        ("OPENROUTER_API_KEY", "sk-test-server"),
        ("OPENROUTER_MODEL", "test/server-model"),
        ("AI_TIMEOUT_SECONDS", "1"),
    ]);

    request::<App, _, _>(|request, ctx| async move {
        let user = prepare_data::init_user_login(&request, &ctx).await;
        let provider =
            MockProvider::ok(&json!({ "items": [] }).to_string()).slow(Duration::from_secs(3));
        install(&ctx, &provider);

        let (status, body) = parse(&request, &user.token, "slow").await;
        assert_eq!(status, 504, "{body}");
        assert_eq!(body["error"], "ai_timeout");
    })
    .await;
}

#[tokio::test]
#[serial]
async fn ai_parse_sends_the_user_timezone_and_local_time() {
    let _env = AiEnv::server();

    request::<App, _, _>(|request, ctx| async move {
        let user = prepare_data::init_user_login(&request, &ctx).await;
        let provider = MockProvider::ok(
            &json!({ "items": [suggestion("Renew domain", Some("2026-12-15"))] }).to_string(),
        );
        install(&ctx, &provider);

        update_settings(&request, &user.token, json!({ "timezone": "Asia/Tokyo" })).await;

        let (status, body) = parse(&request, &user.token, "renew the domain tomorrow").await;
        assert_eq!(status, 200, "{body}");

        let request = provider.last_request();
        let system = &request.messages[0].content;
        assert!(system.contains("Asia/Tokyo"));
        assert!(system.contains("+09:00"));

        // A date-only model value is resolved in the user's timezone, then
        // stored as UTC.
        let parsed: AiParseResponse = serde_json::from_value(body).unwrap();
        let due_at = parsed.items[0].due_at.unwrap();
        assert_eq!(due_at.timestamp(), 1_797_260_400);
    })
    .await;
}

#[tokio::test]
#[serial]
async fn ai_parse_validates_input_locally() {
    let _env = AiEnv::with(&[
        ("AI_ENABLED", "true"),
        ("OPENROUTER_API_KEY", "sk-test-server"),
        ("OPENROUTER_MODEL", "test/server-model"),
        ("AI_MAX_INPUT_CHARS", "5"),
    ]);

    request::<App, _, _>(|request, ctx| async move {
        let user = prepare_data::init_user_login(&request, &ctx).await;
        let provider = MockProvider::ok(&json!({ "items": [] }).to_string());
        install(&ctx, &provider);

        let (status, body) = parse(&request, &user.token, "   ").await;
        assert_eq!(status, 422, "{body}");
        assert_eq!(body["error"], "invalid_input");

        let (status, body) = parse(&request, &user.token, "too long").await;
        assert_eq!(status, 422, "{body}");
        assert_eq!(body["error"], "invalid_input");
        assert_eq!(provider.calls(), 0);
    })
    .await;
}

#[tokio::test]
#[serial]
async fn personal_keys_are_encrypted_and_isolated_between_users() {
    let _env = AiEnv::without_server_key();

    request::<App, _, _>(|request, ctx| async move {
        let first = prepare_data::init_user_login_as(
            &request,
            &ctx,
            "first",
            "ai-first@example.com",
            "first-password",
        )
        .await;
        let second = prepare_data::init_user_login_as(
            &request,
            &ctx,
            "second",
            "ai-second@example.com",
            "second-password",
        )
        .await;

        let secret = "sk-or-v1-personal-secret";
        let settings = update_settings(
            &request,
            &first.token,
            json!({
                "ai_use_server_key": false,
                "ai_model": "test/personal-model",
                "ai_api_key": secret
            }),
        )
        .await;
        assert!(settings.ai.has_personal_key);
        assert_eq!(settings.ai.status, AiStatus::Available);
        assert_eq!(
            settings.ai.active_model.as_deref(),
            Some("test/personal-model")
        );
        assert!(!serde_json::to_string(&settings).unwrap().contains(secret));

        // The database holds a sealed envelope, not the key.
        let stored = user_settings::Model::find_by_user(&ctx.db, first.user.id)
            .await
            .unwrap()
            .unwrap();
        let envelope = stored.ai_api_key_encrypted.clone().unwrap();
        assert!(envelope.starts_with("v1:"));
        assert!(!envelope.contains(secret));
        let cipher = SecretBox::from_base64(&encryption_key()).unwrap().unwrap();
        assert_eq!(cipher.decrypt(&envelope).unwrap(), secret);

        let provider = MockProvider::ok(&json!({ "items": [] }).to_string());
        install(&ctx, &provider);

        let (status, body) = parse(&request, &first.token, "first user").await;
        assert_eq!(status, 200, "{body}");
        assert_eq!(provider.calls(), 1);

        // The second account has no key and must not borrow the first one's.
        let (status, body) = parse(&request, &second.token, "second user").await;
        assert_eq!(status, 409, "{body}");
        assert_eq!(body["error"], "ai_not_configured");
        assert_eq!(provider.calls(), 1);

        let (key, value) = prepare_data::auth_header(&second.token);
        let response = request.get("/api/settings").add_header(key, value).await;
        let second_settings: SettingsResponse = serde_json::from_str(&response.text()).unwrap();
        assert!(!second_settings.ai.has_personal_key);

        // Removing the key is explicit and immediate.
        let settings = update_settings(&request, &first.token, json!({ "ai_api_key": "" })).await;
        assert!(!settings.ai.has_personal_key);
        let stored = user_settings::Model::find_by_user(&ctx.db, first.user.id)
            .await
            .unwrap()
            .unwrap();
        assert!(stored.ai_api_key_encrypted.is_none());
    })
    .await;
}

#[tokio::test]
#[serial]
async fn settings_never_return_the_personal_key() {
    let _env = AiEnv::without_server_key();

    request::<App, _, _>(|request, ctx| async move {
        let user = prepare_data::init_user_login(&request, &ctx).await;
        let secret = "sk-or-v1-do-not-echo";

        let (key, value) = prepare_data::auth_header(&user.token);
        let response = request
            .patch("/api/settings")
            .add_header(key, value)
            .json(&json!({ "ai_api_key": secret }))
            .await;
        assert_eq!(response.status_code(), 200, "{}", response.text());
        let body = response.text();
        assert!(!body.contains(secret));
        assert!(!body.contains("api_key"));

        let (key, value) = prepare_data::auth_header(&user.token);
        let response = request.get("/api/settings").add_header(key, value).await;
        assert_eq!(response.status_code(), 200);
        let body = response.text();
        assert!(!body.contains(secret));
        assert!(!body.contains("api_key"));
    })
    .await;
}
