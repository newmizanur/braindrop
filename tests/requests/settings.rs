use braindrop::{
    app::App,
    dtos::settings::SettingsResponse,
    models::items::{ItemPriority, ItemType},
    models::user_settings::Theme,
};
use loco_rs::testing::prelude::*;
use serde_json::json;
use serial_test::serial;

use super::prepare_data;

#[tokio::test]
#[serial]
async fn settings_require_authentication() {
    request::<App, _, _>(|request, _ctx| async move {
        assert_eq!(request.get("/api/settings").await.status_code(), 401);
        assert_eq!(
            request
                .patch("/api/settings")
                .json(&json!({ "theme": "dark" }))
                .await
                .status_code(),
            401
        );
    })
    .await;
}

#[tokio::test]
#[serial]
async fn settings_defaults_and_updates_persist() {
    request::<App, _, _>(|request, ctx| async move {
        let user = prepare_data::init_user_login(&request, &ctx).await;

        let (key, value) = prepare_data::auth_header(&user.token);
        let response = request.get("/api/settings").add_header(key, value).await;
        assert_eq!(response.status_code(), 200);
        let defaults: SettingsResponse = serde_json::from_str(&response.text()).unwrap();
        assert_eq!(defaults.timezone, "UTC");
        assert_eq!(defaults.theme, Theme::Auto);
        assert_eq!(defaults.default_item_type, ItemType::Task);
        assert_eq!(defaults.default_priority, ItemPriority::Normal);
        assert_eq!(defaults.week_start, 1);

        let (key, value) = prepare_data::auth_header(&user.token);
        let response = request
            .patch("/api/settings")
            .add_header(key, value)
            .json(&json!({
                "timezone": "Europe/Berlin",
                "theme": "dark",
                "default_item_type": "note",
                "default_priority": "high",
                "week_start": 0
            }))
            .await;
        assert_eq!(response.status_code(), 200, "{}", response.text());
        let updated: SettingsResponse = serde_json::from_str(&response.text()).unwrap();
        assert_eq!(updated.timezone, "Europe/Berlin");
        assert_eq!(updated.theme, Theme::Dark);
        assert_eq!(updated.default_item_type, ItemType::Note);
        assert_eq!(updated.default_priority, ItemPriority::High);
        assert_eq!(updated.week_start, 0);

        // A second read returns the persisted values, not defaults.
        let (key, value) = prepare_data::auth_header(&user.token);
        let response = request.get("/api/settings").add_header(key, value).await;
        let reread: SettingsResponse = serde_json::from_str(&response.text()).unwrap();
        assert_eq!(reread.timezone, "Europe/Berlin");
        assert_eq!(reread.theme, Theme::Dark);

        // A partial patch leaves the other fields alone.
        let (key, value) = prepare_data::auth_header(&user.token);
        let response = request
            .patch("/api/settings")
            .add_header(key, value)
            .json(&json!({ "theme": "light" }))
            .await;
        let partial: SettingsResponse = serde_json::from_str(&response.text()).unwrap();
        assert_eq!(partial.theme, Theme::Light);
        assert_eq!(partial.timezone, "Europe/Berlin");
        assert_eq!(partial.week_start, 0);

        // Credentials are never part of the settings contract.
        let body = response.text();
        assert!(!body.contains("password"));
        assert!(!body.contains("secret"));
        assert!(!body.contains("api_key"));
    })
    .await;
}

#[tokio::test]
#[serial]
async fn settings_are_isolated_between_users() {
    request::<App, _, _>(|request, ctx| async move {
        let first = prepare_data::init_user_login_as(
            &request,
            &ctx,
            "first",
            "first@example.com",
            "first-password",
        )
        .await;
        let second = prepare_data::init_user_login_as(
            &request,
            &ctx,
            "second",
            "second@example.com",
            "second-password",
        )
        .await;

        let (key, value) = prepare_data::auth_header(&first.token);
        let response = request
            .patch("/api/settings")
            .add_header(key, value)
            .json(&json!({ "timezone": "Asia/Tokyo", "theme": "dark" }))
            .await;
        assert_eq!(response.status_code(), 200);

        let (key, value) = prepare_data::auth_header(&second.token);
        let response = request.get("/api/settings").add_header(key, value).await;
        let second_settings: SettingsResponse = serde_json::from_str(&response.text()).unwrap();
        assert_eq!(second_settings.timezone, "UTC");
        assert_eq!(second_settings.theme, Theme::Auto);
    })
    .await;
}

#[tokio::test]
#[serial]
async fn settings_validate_timezone_and_week_start() {
    request::<App, _, _>(|request, ctx| async move {
        let user = prepare_data::init_user_login(&request, &ctx).await;

        for payload in [
            json!({ "timezone": "Mars/Olympus_Mons" }),
            json!({ "week_start": 7 }),
            json!({ "week_start": -1 }),
        ] {
            let (key, value) = prepare_data::auth_header(&user.token);
            let response = request
                .patch("/api/settings")
                .add_header(key, value)
                .json(&payload)
                .await;
            assert_eq!(response.status_code(), 422, "{}", response.text());
        }
    })
    .await;
}
