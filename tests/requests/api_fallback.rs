use braindrop::app::App;
use loco_rs::testing::prelude::*;
use serial_test::serial;

#[tokio::test]
#[serial]
async fn unknown_api_route_returns_json_404() {
    request::<App, _, _>(|request, _ctx| async move {
        let response = request.get("/api/does-not-exist").await;

        assert_eq!(response.status_code(), 404);
        let content_type = response.header("content-type");
        assert!(
            content_type
                .to_str()
                .unwrap_or_default()
                .contains("application/json"),
            "unknown API routes must not fall through to the SPA, got {content_type:?}"
        );

        let body: serde_json::Value = response.json();
        assert_eq!(body["code"], "not_found");
        assert_eq!(body["message"], "API route not found");
    })
    .await;
}
