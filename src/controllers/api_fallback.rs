use axum::http::StatusCode;
use axum::response::IntoResponse;
use axum::Json;
use loco_rs::prelude::*;

use crate::dtos::common::ApiError;

/// Catch-all for unknown `/api/*` routes.
///
/// Loco's static middleware serves the SPA for unmatched paths. Without this
/// route a typo in an API path would answer with `index.html` and a 200, which
/// is confusing for clients and hides broken integrations. The static fallback
/// still applies to every non-API path.
async fn not_found() -> impl IntoResponse {
    (
        StatusCode::NOT_FOUND,
        Json(ApiError {
            code: "not_found".to_string(),
            message: "API route not found".to_string(),
            details: None,
        }),
    )
}

pub fn routes() -> Routes {
    Routes::new()
        .prefix("/api")
        .add("/", axum::routing::any(not_found))
        .add("/{*rest}", axum::routing::any(not_found))
}
