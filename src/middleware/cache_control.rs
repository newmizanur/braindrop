//! Cache policy for authenticated responses and the SPA shell.
//!
//! Browser caching of authenticated API responses is undesirable on a shared
//! or short-lived device: the service worker is already careful not to cache
//! them, but the ordinary HTTP cache still could. Hashed `/assets/*` files are
//! left cacheable so the PWA shell keeps working offline.

use axum::{
    extract::Request,
    http::{
        header::{CACHE_CONTROL, CONTENT_TYPE},
        HeaderValue,
    },
    middleware::Next,
    response::Response,
    Router as AXRouter,
};
use loco_rs::{app::AppContext, controller::middleware::MiddlewareLayer, Result};

/// Adds `Cache-Control: no-store` to API responses and served HTML documents.
#[derive(Debug, Clone, Copy)]
pub struct CacheControl;

impl MiddlewareLayer for CacheControl {
    fn name(&self) -> &'static str {
        "cache_control"
    }

    fn config(&self) -> serde_json::Result<serde_json::Value> {
        serde_json::to_value(true)
    }

    fn apply(&self, app: AXRouter<AppContext>) -> Result<AXRouter<AppContext>> {
        Ok(app.layer(axum::middleware::from_fn(set_cache_control)))
    }
}

async fn set_cache_control(request: Request, next: Next) -> Response {
    let path = request.uri().path().to_owned();
    let mut response = next.run(request).await;

    let private = path == "/api"
        || path.starts_with("/api/")
        || path.starts_with("/_")
        || response
            .headers()
            .get(CONTENT_TYPE)
            .and_then(|value| value.to_str().ok())
            .is_some_and(|value| value.starts_with("text/html"));

    if private {
        response
            .headers_mut()
            .insert(CACHE_CONTROL, HeaderValue::from_static("no-store"));
    }

    response
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn middleware_name_is_stable() {
        assert_eq!(CacheControl.name(), "cache_control");
    }
}
