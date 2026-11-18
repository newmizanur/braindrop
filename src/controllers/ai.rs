use std::sync::Arc;

use axum::http::StatusCode;
use loco_rs::{controller::ErrorDetail, prelude::*};

use crate::{
    ai::{
        availability, client::AiProvider, client::OpenRouterClient, config::AiSettings,
        error::AiError, rate_limit,
    },
    dtos::ai::{AiParseResponse, ParseItemsParams},
    models::{user_settings, users},
};

/// Map an internal failure onto a safe HTTP response. Provider bodies and
/// credentials never appear in the message.
fn map_error(error: AiError) -> Error {
    let (status, code) = match &error {
        AiError::Disabled(_) => (StatusCode::CONFLICT, "ai_disabled"),
        AiError::NotConfigured(_) => (StatusCode::CONFLICT, "ai_not_configured"),
        AiError::InvalidInput(_) => (StatusCode::UNPROCESSABLE_ENTITY, "invalid_input"),
        AiError::RateLimited | AiError::ProviderRateLimited => {
            (StatusCode::TOO_MANY_REQUESTS, "ai_rate_limited")
        }
        AiError::Timeout => (StatusCode::GATEWAY_TIMEOUT, "ai_timeout"),
        AiError::Authentication => (StatusCode::BAD_GATEWAY, "ai_authentication"),
        AiError::UnsupportedStructuredOutput => {
            (StatusCode::BAD_GATEWAY, "ai_unsupported_response_format")
        }
        AiError::Provider(_) => (StatusCode::BAD_GATEWAY, "ai_provider_error"),
        AiError::InvalidResponse(_) | AiError::TooLarge(_) => {
            (StatusCode::BAD_GATEWAY, "ai_invalid_response")
        }
        AiError::Encryption(_) => (StatusCode::INTERNAL_SERVER_ERROR, "ai_encryption"),
    };

    Error::CustomError(status, ErrorDetail::new(code, error.to_string()))
}

/// The provider seam. Tests (and future integrations) can install a provider
/// in the app's shared store; normal requests build the OpenRouter client.
fn provider(
    ctx: &AppContext,
    resolved: &crate::ai::availability::ResolvedAi,
) -> Result<Arc<dyn AiProvider>, AiError> {
    if let Some(provider) = ctx.shared_store.get::<Arc<dyn AiProvider>>() {
        return Ok(provider);
    }
    Ok(Arc::new(OpenRouterClient::new(resolved)?))
}

#[debug_handler]
async fn parse(
    auth: auth::JWT,
    State(ctx): State<AppContext>,
    Json(params): Json<ParseItemsParams>,
) -> Result<Response> {
    let user = users::Model::find_by_pid(&ctx.db, &auth.claims.pid)
        .await
        .map_err(Error::from)?;
    let settings = user_settings::Model::get_or_create(&ctx.db, user.id).await?;
    let config = AiSettings::from_config(&ctx.config)?;

    let text = params.text.trim();
    if text.is_empty() {
        return Err(map_error(AiError::InvalidInput(
            "text must not be empty".to_string(),
        )));
    }
    let limit = config.input_limit();
    if text.chars().count() > limit {
        return Err(map_error(AiError::InvalidInput(format!(
            "text must be at most {limit} characters"
        ))));
    }

    // Resolve first so a disabled or unconfigured account gets a clear answer
    // without spending a rate-limit slot or making a network call.
    let resolved = availability::resolve(&config, &settings).map_err(map_error)?;
    rate_limit::check(&ctx, user.id, config.rate_limit())
        .await
        .map_err(map_error)?;

    let provider = provider(&ctx, &resolved).map_err(map_error)?;
    let items = crate::ai::client::extract_items(
        provider.as_ref(),
        &config,
        text,
        settings.timezone(),
        chrono::Utc::now(),
    )
    .await
    .map_err(map_error)?;

    // Metadata only: the captured text and the suggestions stay in memory.
    tracing::info!(
        user_id = user.id,
        items = items.len(),
        credential = ?resolved.source,
        "ai parse completed"
    );

    format::json(AiParseResponse { items })
}

pub fn routes() -> Routes {
    Routes::new().add("/api/ai/parse", post(parse))
}
