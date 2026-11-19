use braindrop::app::App;
use loco_rs::testing::prelude::*;
use serial_test::serial;

#[tokio::test]
#[serial]
async fn security_headers_and_private_cache_policy_are_present() {
    request::<App, _, _>(|request, _ctx| async move {
        let response = request.get("/api/auth/config").await;
        assert_eq!(response.status_code(), 200);

        let csp_header = response.header("content-security-policy");
        let csp = csp_header.to_str().unwrap_or_default();
        assert!(csp.contains("default-src 'self'"), "CSP: {csp}");
        assert!(csp.contains("script-src 'self'"), "CSP: {csp}");
        assert!(csp.contains("script-src-attr 'none'"), "CSP: {csp}");
        assert!(csp.contains("frame-ancestors 'none'"), "CSP: {csp}");
        assert!(csp.contains("object-src 'none'"), "CSP: {csp}");
        assert!(!csp.contains("unsafe-eval"), "CSP: {csp}");

        for (name, expected) in [
            ("x-content-type-options", "nosniff"),
            ("x-frame-options", "DENY"),
            ("referrer-policy", "no-referrer"),
            ("cross-origin-opener-policy", "same-origin"),
            ("cross-origin-resource-policy", "same-origin"),
        ] {
            assert_eq!(
                response.header(name).to_str().unwrap_or_default(),
                expected,
                "unexpected {name}"
            );
        }
        let permissions_header = response.header("permissions-policy");
        let permissions = permissions_header.to_str().unwrap_or_default();
        assert!(
            permissions.contains("camera=()"),
            "Permissions-Policy: {permissions}"
        );
        assert_eq!(
            response
                .header("cache-control")
                .to_str()
                .unwrap_or_default(),
            "no-store"
        );

        let html = request.get("/").await;
        assert_eq!(html.status_code(), 200);
        assert_eq!(
            html.header("cache-control").to_str().unwrap_or_default(),
            "no-store",
            "the SPA shell must not be stored in the browser HTTP cache"
        );
    })
    .await;
}
