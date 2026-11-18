use std::{
    sync::{Arc, Mutex},
    time::Duration as StdDuration,
};

use async_trait::async_trait;
use base64::{engine::general_purpose::STANDARD, Engine};
use braindrop::{
    app::App,
    dtos::settings::SettingsResponse,
    models::{
        items::{self, ItemPriority, ItemStatus, ItemType, NewItem},
        notification_deliveries::{
            self, DeliveryStatus, Entity as DeliveryEntity, Model as Delivery, NewDelivery,
            NotificationChannel, NotificationType,
        },
        user_settings,
    },
    notifications::{
        error::NotificationError,
        provider::{ChannelCredentials, NotificationSender},
        scan,
    },
};
use chrono::{DateTime, Duration, TimeZone, Utc};
use loco_rs::{app::AppContext, testing::prelude::*};
use sea_orm::{ColumnTrait, EntityTrait, QueryFilter};
use serde_json::json;
use serial_test::serial;

use super::prepare_data;

const TELEGRAM_TOKEN: &str = "123456789:abcdefghijklmnopqrstuvwxyz";
const TELEGRAM_CHAT: &str = "-1001234567890";
const DISCORD_WEBHOOK: &str =
    "https://discord.com/api/webhooks/123456789012345678/abcdefghijklmnopqrstuvwxyz0123456789ABCD";

const ENV_KEYS: &[&str] = &[
    "NOTIFICATION_ENCRYPTION_KEY",
    "AI_ENCRYPTION_KEY",
    "NOTIFICATION_TEST_RATE_LIMIT",
    "NOTIFICATION_MAX_ATTEMPTS",
    "NOTIFICATION_RETRY_BASE_SECONDS",
    "NOTIFICATION_RETRY_MAX_SECONDS",
    "NOTIFICATION_HISTORY_RETENTION_DAYS",
    "TELEGRAM_API_BASE",
    "DISCORD_API_BASE",
];

fn encryption_key() -> String {
    STANDARD.encode([11u8; 32])
}

/// Set notification configuration for one test and restore the environment
/// afterwards. `#[serial]` keeps the process environment stable between tests.
struct NotificationEnv;

impl NotificationEnv {
    fn default_env() -> Self {
        Self::apply(&[])
    }

    fn with(vars: &[(&str, &str)]) -> Self {
        Self::apply(vars)
    }

    fn apply(vars: &[(&str, &str)]) -> Self {
        std::env::set_var("NOTIFICATION_ENCRYPTION_KEY", encryption_key());
        std::env::set_var("AI_ENCRYPTION_KEY", encryption_key());
        std::env::set_var("NOTIFICATION_TEST_RATE_LIMIT", "5");
        for (key, value) in vars {
            std::env::set_var(key, value);
        }
        Self
    }
}

impl Drop for NotificationEnv {
    fn drop(&mut self) {
        for key in ENV_KEYS {
            std::env::remove_var(key);
        }
    }
}

#[derive(Clone, Debug)]
struct Call {
    channel: NotificationChannel,
    credentials: ChannelCredentials,
    text: String,
}

#[derive(Clone)]
struct MockSender {
    calls: Arc<Mutex<Vec<Call>>>,
    reply: Arc<dyn Fn() -> Result<(), NotificationError> + Send + Sync>,
}

impl MockSender {
    fn ok() -> Self {
        Self::with_reply(|| Ok(()))
    }

    fn failing(error: NotificationError) -> Self {
        Self::with_reply(move || Err(error.clone()))
    }

    fn with_reply(
        reply: impl Fn() -> Result<(), NotificationError> + Send + Sync + 'static,
    ) -> Self {
        Self {
            calls: Arc::new(Mutex::new(Vec::new())),
            reply: Arc::new(reply),
        }
    }

    fn calls(&self) -> Vec<Call> {
        self.calls.lock().unwrap().clone()
    }
}

#[async_trait]
impl NotificationSender for MockSender {
    async fn send(
        &self,
        channel: NotificationChannel,
        credentials: &ChannelCredentials,
        message: &braindrop::notifications::message::OutgoingMessage,
    ) -> Result<(), NotificationError> {
        self.calls.lock().unwrap().push(Call {
            channel,
            credentials: credentials.clone(),
            text: message.text.clone(),
        });
        (self.reply)()
    }
}

fn install(ctx: &AppContext, sender: &MockSender) {
    ctx.shared_store
        .insert::<Arc<dyn NotificationSender>>(Arc::new(sender.clone()));
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

async fn get_settings(request: &loco_rs::TestServer, token: &str) -> (u16, String) {
    let (key, value) = prepare_data::auth_header(token);
    let response = request.get("/api/settings").add_header(key, value).await;
    (response.status_code().as_u16(), response.text())
}

async fn send_test(
    request: &loco_rs::TestServer,
    token: &str,
    channel: &str,
) -> (u16, serde_json::Value) {
    let (key, value) = prepare_data::auth_header(token);
    let response = request
        .post("/api/notifications/test")
        .add_header(key, value)
        .json(&json!({ "channel": channel }))
        .await;
    let status = response.status_code().as_u16();
    let body = serde_json::from_str(&response.text()).unwrap_or(serde_json::Value::Null);
    (status, body)
}

fn enabled_payload() -> serde_json::Value {
    json!({
        "notifications_enabled": true,
        "telegram_enabled": true,
        "telegram_chat_id": TELEGRAM_CHAT,
        "telegram_bot_token": TELEGRAM_TOKEN,
        "discord_enabled": true,
        "discord_webhook": DISCORD_WEBHOOK,
        "reminders_enabled": true,
        "daily_digest_enabled": true,
        "daily_digest_time": "09:00",
        "overdue_notifications_enabled": true,
    })
}

fn new_item(title: &str) -> NewItem {
    NewItem {
        item_type: ItemType::Task,
        title: title.to_string(),
        content: None,
        category: None,
        tags: Vec::new(),
        status: ItemStatus::Inbox,
        priority: ItemPriority::Normal,
        due_at: None,
        remind_at: None,
    }
}

fn fixed_now() -> DateTime<Utc> {
    Utc.with_ymd_and_hms(2026, 11, 19, 9, 30, 0).unwrap()
}

async fn deliveries(ctx: &AppContext) -> Vec<Delivery> {
    DeliveryEntity::find().all(&ctx.db).await.unwrap()
}

async fn single_delivery(ctx: &AppContext) -> Delivery {
    let rows = deliveries(ctx).await;
    assert_eq!(rows.len(), 1, "expected exactly one delivery record");
    rows.into_iter().next().unwrap()
}

#[tokio::test]
#[serial]
async fn notification_test_requires_authentication() {
    let _env = NotificationEnv::default_env();
    request::<App, _, _>(|request, _ctx| async move {
        let response = request
            .post("/api/notifications/test")
            .json(&json!({ "channel": "telegram" }))
            .await;
        assert_eq!(response.status_code(), 401);
    })
    .await;
}

#[tokio::test]
#[serial]
async fn notification_test_requires_an_enabled_configured_channel() {
    let _env = NotificationEnv::default_env();
    request::<App, _, _>(|request, ctx| async move {
        let user = prepare_data::init_user_login(&request, &ctx).await;
        let sender = MockSender::ok();
        install(&ctx, &sender);

        // Both channels are disabled by default.
        let (status, _) = send_test(&request, &user.token, "telegram").await;
        assert_eq!(status, 409);

        // Enabled but with no credentials is still a conflict.
        update_settings(&request, &user.token, json!({ "telegram_enabled": true })).await;
        let (status, body) = send_test(&request, &user.token, "telegram").await;
        assert_eq!(status, 409, "{body}");
        assert!(sender.calls().is_empty());
    })
    .await;
}

#[tokio::test]
#[serial]
async fn notification_test_sends_through_both_enabled_channels() {
    let _env = NotificationEnv::default_env();
    request::<App, _, _>(|request, ctx| async move {
        let user = prepare_data::init_user_login(&request, &ctx).await;
        let sender = MockSender::ok();
        install(&ctx, &sender);
        update_settings(&request, &user.token, enabled_payload()).await;

        let (status, body) = send_test(&request, &user.token, "telegram").await;
        assert_eq!(status, 200, "{body}");
        assert_eq!(body["status"], "sent");
        assert_eq!(body["channel"], "telegram");

        let (status, body) = send_test(&request, &user.token, "discord").await;
        assert_eq!(status, 200, "{body}");

        let calls = sender.calls();
        assert_eq!(calls.len(), 2);
        assert_eq!(calls[0].channel, NotificationChannel::Telegram);
        assert!(calls[0].text.contains("telegram"));
        assert!(matches!(
            calls[0].credentials,
            ChannelCredentials::Telegram {
                ref bot_token,
                ref chat_id
            } if bot_token == TELEGRAM_TOKEN && chat_id == TELEGRAM_CHAT
        ));
        assert_eq!(calls[1].channel, NotificationChannel::Discord);
        assert!(calls[1].text.contains("discord"));
        assert!(matches!(
            calls[1].credentials,
            ChannelCredentials::Discord { ref webhook_url } if webhook_url == DISCORD_WEBHOOK
        ));
    })
    .await;
}

#[tokio::test]
#[serial]
async fn notification_test_maps_provider_failure_without_leaking_secrets() {
    let _env = NotificationEnv::default_env();
    request::<App, _, _>(|request, ctx| async move {
        let user = prepare_data::init_user_login(&request, &ctx).await;
        let sender = MockSender::failing(NotificationError::Permanent("bad chat".to_string()));
        install(&ctx, &sender);
        update_settings(&request, &user.token, enabled_payload()).await;

        let (status, body) = send_test(&request, &user.token, "telegram").await;
        assert_eq!(status, 502, "{body}");
        let text = body.to_string();
        assert!(!text.contains(TELEGRAM_TOKEN));
        assert!(!text.contains(DISCORD_WEBHOOK));
    })
    .await;
}

#[tokio::test]
#[serial]
async fn notification_test_is_rate_limited_per_user() {
    let _env = NotificationEnv::with(&[("NOTIFICATION_TEST_RATE_LIMIT", "1")]);
    request::<App, _, _>(|request, ctx| async move {
        let user = prepare_data::init_user_login(&request, &ctx).await;
        let sender = MockSender::ok();
        install(&ctx, &sender);
        update_settings(&request, &user.token, enabled_payload()).await;

        let (status, _) = send_test(&request, &user.token, "telegram").await;
        assert_eq!(status, 200);
        let (status, body) = send_test(&request, &user.token, "telegram").await;
        assert_eq!(status, 429, "{body}");
        assert_eq!(sender.calls().len(), 1);
    })
    .await;
}

#[tokio::test]
#[serial]
async fn notification_credentials_are_sealed_and_never_returned() {
    let _env = NotificationEnv::default_env();
    request::<App, _, _>(|request, ctx| async move {
        let user = prepare_data::init_user_login(&request, &ctx).await;
        let settings = update_settings(&request, &user.token, enabled_payload()).await;

        assert!(settings.notifications.enabled);
        assert!(settings.notifications.telegram.enabled);
        assert!(settings.notifications.telegram.configured);
        assert!(settings.notifications.telegram.token_set);
        assert_eq!(settings.notifications.telegram.chat_id, TELEGRAM_CHAT);
        assert!(settings.notifications.discord.enabled);
        assert!(settings.notifications.discord.configured);
        assert!(settings.notifications.discord.webhook_set);

        let (status, body) = get_settings(&request, &user.token).await;
        assert_eq!(status, 200);
        assert!(!body.contains(TELEGRAM_TOKEN), "{body}");
        assert!(!body.contains(DISCORD_WEBHOOK), "{body}");
        assert!(!body.contains("bot_token_encrypted"), "{body}");

        let stored = user_settings::Model::find_by_user(&ctx.db, user.user.id)
            .await
            .unwrap()
            .unwrap();
        let token = stored.telegram_bot_token_encrypted.unwrap();
        let webhook = stored.discord_webhook_encrypted.unwrap();
        assert!(token.starts_with("v1:"));
        assert!(webhook.starts_with("v1:"));
        assert!(!token.contains(TELEGRAM_TOKEN));
        assert!(!webhook.contains(DISCORD_WEBHOOK));

        // Removing a credential is a write-only empty string, not a 422.
        let cleared = update_settings(
            &request,
            &user.token,
            json!({ "telegram_bot_token": "", "discord_webhook": "" }),
        )
        .await;
        assert!(!cleared.notifications.telegram.token_set);
        assert!(!cleared.notifications.discord.webhook_set);
        assert!(!cleared.notifications.telegram.configured);
    })
    .await;
}

#[tokio::test]
#[serial]
async fn notification_settings_are_isolated_between_users() {
    let _env = NotificationEnv::default_env();
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
        let sender = MockSender::ok();
        install(&ctx, &sender);

        update_settings(&request, &first.token, enabled_payload()).await;
        let (status, body) = get_settings(&request, &second.token).await;
        assert_eq!(status, 200);
        let settings: SettingsResponse = serde_json::from_str(&body).unwrap();
        assert!(!settings.notifications.enabled);
        assert!(!settings.notifications.telegram.configured);
        assert!(!settings.notifications.discord.configured);

        // The second account cannot borrow the first account's credentials.
        let (status, _) = send_test(&request, &second.token, "telegram").await;
        assert_eq!(status, 409);
        assert!(sender.calls().is_empty());
    })
    .await;
}

#[tokio::test]
#[serial]
async fn notification_settings_reject_unsafe_or_malformed_configuration() {
    let _env = NotificationEnv::default_env();
    request::<App, _, _>(|request, ctx| async move {
        let user = prepare_data::init_user_login(&request, &ctx).await;

        for payload in [
            // SSRF and non-official hosts.
            json!({ "discord_webhook": "http://discord.com/api/webhooks/123456789012345678/abcdefghijklmnopqrstuvwxyz0123456789ABCD" }),
            json!({ "discord_webhook": "https://127.0.0.1/api/webhooks/123456789012345678/abcdefghijklmnopqrstuvwxyz0123456789ABCD" }),
            json!({ "discord_webhook": "https://discord.com.evil.example/api/webhooks/123456789012345678/abcdefghijklmnopqrstuvwxyz0123456789ABCD" }),
            json!({ "discord_webhook": "https://discord.com:8443/api/webhooks/123456789012345678/abcdefghijklmnopqrstuvwxyz0123456789ABCD" }),
            json!({ "discord_webhook": "https://discord.com/not-a-webhook" }),
            json!({ "telegram_bot_token": "not-a-token" }),
            json!({ "telegram_chat_id": "bad chat id" }),
            json!({ "daily_digest_time": "25:99" }),
        ] {
            let (key, value) = prepare_data::auth_header(&user.token);
            let response = request
                .patch("/api/settings")
                .add_header(key, value)
                .json(&payload)
                .await;
            assert_eq!(response.status_code(), 422, "{payload} -> {}", response.text());
        }
    })
    .await;
}

#[tokio::test]
#[serial]
async fn reminders_are_enqueued_once_and_sent_through_each_channel() {
    let _env = NotificationEnv::default_env();
    request::<App, _, _>(|request, ctx| async move {
        let user = prepare_data::init_user_login(&request, &ctx).await;
        let sender = MockSender::ok();
        install(&ctx, &sender);
        update_settings(&request, &user.token, enabled_payload()).await;

        let now = fixed_now();
        let mut item = new_item("Renew domain");
        item.remind_at = Some((now - Duration::minutes(1)).fixed_offset());
        let item = items::Model::create_many(&ctx.db, user.user.id, vec![item])
            .await
            .unwrap()
            .remove(0);

        let created = scan::reminders(&ctx, now).await.unwrap();
        assert_eq!(created, 2, "one delivery per enabled channel");

        // A second tick must not create duplicates.
        assert_eq!(scan::reminders(&ctx, now).await.unwrap(), 0);
        assert_eq!(deliveries(&ctx).await.len(), 2);

        for delivery in deliveries(&ctx).await {
            scan::deliver(&ctx, delivery.id, now).await.unwrap();
        }

        let calls = sender.calls();
        assert_eq!(calls.len(), 2);
        assert!(calls.iter().all(|call| call.text.contains("Renew domain")));

        let sent = deliveries(&ctx).await;
        assert!(sent
            .iter()
            .all(|delivery| delivery.status == DeliveryStatus::Sent.as_str()));
        assert_eq!(sent[0].item_id, Some(item.id));
    })
    .await;
}

#[tokio::test]
#[serial]
async fn completed_archived_and_deleted_items_are_not_notified() {
    let _env = NotificationEnv::default_env();
    request::<App, _, _>(|request, ctx| async move {
        let user = prepare_data::init_user_login(&request, &ctx).await;
        let sender = MockSender::ok();
        install(&ctx, &sender);
        update_settings(&request, &user.token, enabled_payload()).await;

        let now = fixed_now();
        let mut completed = new_item("Completed task");
        completed.status = ItemStatus::Completed;
        completed.remind_at = Some((now - Duration::minutes(1)).fixed_offset());
        let mut archived = new_item("Archived task");
        archived.status = ItemStatus::Archived;
        archived.remind_at = Some((now - Duration::minutes(1)).fixed_offset());
        items::Model::create_many(&ctx.db, user.user.id, vec![completed, archived])
            .await
            .unwrap();

        assert_eq!(scan::reminders(&ctx, now).await.unwrap(), 0);
        assert!(deliveries(&ctx).await.is_empty());

        // A delivery created before the item was deleted is cancelled when it
        // finally runs.
        let delivery = seed_reminder(&ctx, user.user.id, now).await;
        items::Entity::delete_by_id(delivery.item_id.unwrap())
            .exec(&ctx.db)
            .await
            .unwrap();
        scan::deliver(&ctx, delivery.id, now).await.unwrap();
        assert!(sender.calls().is_empty());
        assert_eq!(
            delivery_status(&ctx, delivery.id).await,
            DeliveryStatus::Cancelled.as_str()
        );
    })
    .await;
}

#[tokio::test]
#[serial]
async fn an_edited_reminder_cancels_the_old_delivery() {
    let _env = NotificationEnv::default_env();
    request::<App, _, _>(|request, ctx| async move {
        let user = prepare_data::init_user_login(&request, &ctx).await;
        let sender = MockSender::ok();
        install(&ctx, &sender);
        update_settings(&request, &user.token, enabled_payload()).await;

        let now = fixed_now();
        let delivery = seed_reminder(&ctx, user.user.id, now).await;
        let item_id = delivery.item_id.unwrap();
        let item = items::Entity::find_by_id(item_id)
            .one(&ctx.db)
            .await
            .unwrap()
            .unwrap();

        let updated = items::Model::update_for_user(
            &ctx.db,
            user.user.id,
            &item.pid.to_string(),
            items::ItemPatch {
                remind_at: Some(Some((now + Duration::hours(1)).fixed_offset())),
                ..Default::default()
            },
        )
        .await
        .unwrap();
        assert_ne!(updated.remind_at, item.remind_at);

        scan::deliver(&ctx, delivery.id, now).await.unwrap();
        assert!(sender.calls().is_empty());
        assert_eq!(
            delivery_status(&ctx, delivery.id).await,
            DeliveryStatus::Cancelled.as_str()
        );
    })
    .await;
}

#[tokio::test]
#[serial]
async fn reminders_can_be_disabled_after_a_delivery_is_queued() {
    let _env = NotificationEnv::default_env();
    request::<App, _, _>(|request, ctx| async move {
        let user = prepare_data::init_user_login(&request, &ctx).await;
        let sender = MockSender::ok();
        install(&ctx, &sender);
        update_settings(&request, &user.token, enabled_payload()).await;

        let now = fixed_now();
        let delivery = seed_reminder(&ctx, user.user.id, now).await;

        update_settings(
            &request,
            &user.token,
            json!({ "notifications_enabled": false }),
        )
        .await;
        scan::deliver(&ctx, delivery.id, now).await.unwrap();
        assert!(sender.calls().is_empty());
        assert_eq!(
            delivery_status(&ctx, delivery.id).await,
            DeliveryStatus::Cancelled.as_str()
        );
    })
    .await;
}

#[tokio::test]
#[serial]
async fn concurrent_scans_create_one_delivery_per_channel() {
    let _env = NotificationEnv::default_env();
    request::<App, _, _>(|request, ctx| async move {
        let user = prepare_data::init_user_login(&request, &ctx).await;
        install(&ctx, &MockSender::ok());
        update_settings(&request, &user.token, enabled_payload()).await;

        let now = fixed_now();
        let mut item = new_item("Race");
        item.remind_at = Some((now - Duration::minutes(1)).fixed_offset());
        items::Model::create_many(&ctx.db, user.user.id, vec![item])
            .await
            .unwrap();

        let (first, second) = tokio::join!(scan::reminders(&ctx, now), scan::reminders(&ctx, now));
        assert_eq!(first.unwrap() + second.unwrap(), 2);
        assert_eq!(deliveries(&ctx).await.len(), 2);
    })
    .await;
}

#[tokio::test]
#[serial]
async fn daily_digest_respects_the_local_delivery_window() {
    let _env = NotificationEnv::default_env();
    request::<App, _, _>(|request, ctx| async move {
        let user = prepare_data::init_user_login(&request, &ctx).await;
        install(&ctx, &MockSender::ok());
        update_settings(
            &request,
            &user.token,
            json!({
                "timezone": "Asia/Tokyo",
                "notifications_enabled": true,
                "telegram_enabled": true,
                "telegram_chat_id": TELEGRAM_CHAT,
                "telegram_bot_token": TELEGRAM_TOKEN,
                "daily_digest_enabled": true,
                "daily_digest_time": "09:00",
            }),
        )
        .await;

        // 2026-11-19 09:30 in Tokyo is 2026-11-19 00:30 UTC.
        let in_window = Utc.with_ymd_and_hms(2026, 11, 19, 0, 30, 0).unwrap();
        let mut item = new_item("Tokyo task");
        item.due_at = Some((in_window + Duration::hours(5)).fixed_offset());
        items::Model::create_many(&ctx.db, user.user.id, vec![item])
            .await
            .unwrap();

        // 08:30 local is before the configured time.
        let before = Utc.with_ymd_and_hms(2026, 11, 18, 23, 30, 0).unwrap();
        assert_eq!(scan::digests(&ctx, before).await.unwrap(), 0);
        assert_eq!(scan::digests(&ctx, in_window).await.unwrap(), 1);
        // Repeating the scan within the window does not create a second digest.
        assert_eq!(scan::digests(&ctx, in_window).await.unwrap(), 0);

        let delivery = single_delivery(&ctx).await;
        assert_eq!(
            delivery.notification_type,
            NotificationType::DailyDigest.as_str()
        );
        assert_eq!(delivery.channel, NotificationChannel::Telegram.as_str());
        scan::deliver(&ctx, delivery.id, in_window).await.unwrap();
        assert_eq!(
            delivery_status(&ctx, delivery.id).await,
            DeliveryStatus::Sent.as_str()
        );
    })
    .await;
}

#[tokio::test]
#[serial]
async fn a_cross_midnight_digest_window_does_not_send_twice() {
    let _env = NotificationEnv::default_env();
    request::<App, _, _>(|request, ctx| async move {
        let user = prepare_data::init_user_login(&request, &ctx).await;
        install(&ctx, &MockSender::ok());
        update_settings(
            &request,
            &user.token,
            json!({
                "timezone": "UTC",
                "notifications_enabled": true,
                "telegram_enabled": true,
                "telegram_chat_id": TELEGRAM_CHAT,
                "telegram_bot_token": TELEGRAM_TOKEN,
                "daily_digest_enabled": true,
                "daily_digest_time": "23:30",
            }),
        )
        .await;

        let late = Utc.with_ymd_and_hms(2026, 11, 19, 23, 30, 0).unwrap();
        let mut item = new_item("Late digest task");
        item.due_at = Some((late - Duration::minutes(30)).fixed_offset());
        items::Model::create_many(&ctx.db, user.user.id, vec![item])
            .await
            .unwrap();

        assert_eq!(scan::digests(&ctx, late).await.unwrap(), 1);
        // 00:15 the next day is still inside the same two-hour window and must
        // reuse the previous local day's deduplication key.
        let after_midnight = Utc.with_ymd_and_hms(2026, 11, 20, 0, 15, 0).unwrap();
        assert_eq!(scan::digests(&ctx, after_midnight).await.unwrap(), 0);
        assert_eq!(deliveries(&ctx).await.len(), 1);
    })
    .await;
}

#[tokio::test]
#[serial]
async fn an_empty_digest_is_skipped() {
    let _env = NotificationEnv::default_env();
    request::<App, _, _>(|request, ctx| async move {
        let user = prepare_data::init_user_login(&request, &ctx).await;
        install(&ctx, &MockSender::ok());
        update_settings(&request, &user.token, enabled_payload()).await;

        assert_eq!(scan::digests(&ctx, fixed_now()).await.unwrap(), 0);
        assert!(deliveries(&ctx).await.is_empty());
    })
    .await;
}

#[tokio::test]
#[serial]
async fn overdue_notifications_are_grouped_once_per_local_day() {
    let _env = NotificationEnv::default_env();
    request::<App, _, _>(|request, ctx| async move {
        let user = prepare_data::init_user_login(&request, &ctx).await;
        install(&ctx, &MockSender::ok());
        update_settings(&request, &user.token, enabled_payload()).await;

        let now = fixed_now();
        for title in ["Late one", "Late two"] {
            let mut item = new_item(title);
            item.due_at = Some((now - Duration::days(1)).fixed_offset());
            items::Model::create_many(&ctx.db, user.user.id, vec![item])
                .await
                .unwrap();
        }

        assert_eq!(scan::overdue(&ctx, now).await.unwrap(), 2);
        assert_eq!(scan::overdue(&ctx, now).await.unwrap(), 0);
        assert_eq!(deliveries(&ctx).await.len(), 2);

        // The next local day gets a new grouped notification.
        let tomorrow = now + Duration::days(1);
        assert_eq!(scan::overdue(&ctx, tomorrow).await.unwrap(), 2);
        assert_eq!(deliveries(&ctx).await.len(), 4);
    })
    .await;
}

#[tokio::test]
#[serial]
async fn retryable_failures_back_off_and_then_fail_after_the_attempt_limit() {
    let _env = NotificationEnv::with(&[
        ("NOTIFICATION_MAX_ATTEMPTS", "2"),
        ("NOTIFICATION_RETRY_BASE_SECONDS", "1"),
    ]);
    request::<App, _, _>(|request, ctx| async move {
        let user = prepare_data::init_user_login(&request, &ctx).await;
        let sender = MockSender::failing(NotificationError::Provider("temporary".to_string()));
        install(&ctx, &sender);
        update_settings(&request, &user.token, enabled_payload()).await;

        let now = fixed_now();
        let delivery = seed_reminder(&ctx, user.user.id, now).await;

        scan::deliver(&ctx, delivery.id, now).await.unwrap();
        let after_first = load(&ctx, delivery.id).await;
        assert_eq!(after_first.status, DeliveryStatus::Pending.as_str());
        assert_eq!(after_first.attempts, 1);
        assert!(after_first.next_attempt_at.is_some());
        assert!(after_first
            .last_error
            .as_deref()
            .is_some_and(|error| error.contains("temporary")));

        // A duplicate job that runs before the backoff expires must not send.
        scan::deliver(&ctx, delivery.id, now).await.unwrap();
        let still_pending = load(&ctx, delivery.id).await;
        assert_eq!(still_pending.attempts, 1);
        assert_eq!(sender.calls().len(), 1);

        scan::deliver(&ctx, delivery.id, now + Duration::seconds(2))
            .await
            .unwrap();
        let after_second = load(&ctx, delivery.id).await;
        assert_eq!(after_second.status, DeliveryStatus::Failed.as_str());
        assert_eq!(after_second.attempts, 2);
        assert_eq!(sender.calls().len(), 2);
    })
    .await;
}

#[tokio::test]
#[serial]
async fn provider_rate_limits_use_retry_after() {
    let _env = NotificationEnv::default_env();
    request::<App, _, _>(|request, ctx| async move {
        let user = prepare_data::init_user_login(&request, &ctx).await;
        let sender = MockSender::failing(NotificationError::ProviderRateLimited {
            retry_after: Some(StdDuration::from_secs(7)),
        });
        install(&ctx, &sender);
        update_settings(&request, &user.token, enabled_payload()).await;

        let now = fixed_now();
        let delivery = seed_reminder(&ctx, user.user.id, now).await;
        scan::deliver(&ctx, delivery.id, now).await.unwrap();

        let after = load(&ctx, delivery.id).await;
        assert_eq!(after.status, DeliveryStatus::Pending.as_str());
        let next = after.next_attempt_at.unwrap().with_timezone(&Utc);
        assert_eq!((next - now).num_seconds(), 7);
    })
    .await;
}

#[tokio::test]
#[serial]
async fn permanent_failures_are_not_retried() {
    let _env = NotificationEnv::default_env();
    request::<App, _, _>(|request, ctx| async move {
        let user = prepare_data::init_user_login(&request, &ctx).await;
        let sender = MockSender::failing(NotificationError::Authentication);
        install(&ctx, &sender);
        update_settings(&request, &user.token, enabled_payload()).await;

        let now = fixed_now();
        let delivery = seed_reminder(&ctx, user.user.id, now).await;
        scan::deliver(&ctx, delivery.id, now).await.unwrap();

        let after = load(&ctx, delivery.id).await;
        assert_eq!(after.status, DeliveryStatus::Failed.as_str());
        assert_eq!(after.attempts, 1);
        assert!(after.next_attempt_at.is_none());
    })
    .await;
}

#[tokio::test]
#[serial]
async fn recovery_requeues_pending_and_abandoned_deliveries() {
    let _env = NotificationEnv::default_env();
    request::<App, _, _>(|request, ctx| async move {
        let user = prepare_data::init_user_login(&request, &ctx).await;
        let sender = MockSender::ok();
        install(&ctx, &sender);
        update_settings(&request, &user.token, enabled_payload()).await;

        let now = fixed_now();
        let delivery = seed_reminder(&ctx, user.user.id, now).await;
        // Simulate a worker that died while processing.
        Delivery::claim(&ctx.db, delivery.id, now - Duration::hours(1))
            .await
            .unwrap();

        let recovered = scan::recover(&ctx, now).await.unwrap();
        assert_eq!(recovered, 1);
        assert_eq!(sender.calls().len(), 1);
        assert_eq!(
            delivery_status(&ctx, delivery.id).await,
            DeliveryStatus::Sent.as_str()
        );
    })
    .await;
}

#[tokio::test]
#[serial]
async fn cleanup_deletes_finished_history_past_retention() {
    let _env = NotificationEnv::with(&[("NOTIFICATION_HISTORY_RETENTION_DAYS", "1")]);
    request::<App, _, _>(|request, ctx| async move {
        let user = prepare_data::init_user_login(&request, &ctx).await;
        install(&ctx, &MockSender::ok());
        update_settings(&request, &user.token, enabled_payload()).await;

        let now = fixed_now();
        let delivery = seed_reminder(&ctx, user.user.id, now).await;
        Delivery::mark_sent(&ctx.db, delivery.id, now - Duration::days(2))
            .await
            .unwrap();

        let removed = scan::cleanup(&ctx, now).await.unwrap();
        assert_eq!(removed, 1);
        assert!(deliveries(&ctx).await.is_empty());
    })
    .await;
}

async fn seed_reminder(ctx: &AppContext, user_id: i64, now: DateTime<Utc>) -> Delivery {
    let mut item = new_item("Seeded reminder");
    item.remind_at = Some((now - Duration::minutes(1)).fixed_offset());
    let item = items::Model::create_many(&ctx.db, user_id, vec![item])
        .await
        .unwrap()
        .remove(0);
    let id = Delivery::create(
        &ctx.db,
        NewDelivery {
            user_id,
            item_id: Some(item.id),
            channel: NotificationChannel::Telegram,
            notification_type: NotificationType::Reminder,
            deduplication_key: notification_deliveries::reminder_key(
                item.pid,
                item.remind_at.unwrap().with_timezone(&Utc),
                NotificationChannel::Telegram,
            ),
            scheduled_at: item.remind_at.unwrap().with_timezone(&Utc),
        },
    )
    .await
    .unwrap()
    .expect("the seeded delivery should be inserted");
    load(ctx, id).await
}

async fn load(ctx: &AppContext, id: i64) -> Delivery {
    DeliveryEntity::find_by_id(id)
        .one(&ctx.db)
        .await
        .unwrap()
        .unwrap()
}

async fn delivery_status(ctx: &AppContext, id: i64) -> &'static str {
    match load(ctx, id).await.delivery_status() {
        Some(DeliveryStatus::Pending) => "pending",
        Some(DeliveryStatus::Processing) => "processing",
        Some(DeliveryStatus::Sent) => "sent",
        Some(DeliveryStatus::Failed) => "failed",
        Some(DeliveryStatus::Cancelled) => "cancelled",
        None => "unknown",
    }
}

#[tokio::test]
#[serial]
async fn notification_deliveries_are_scoped_to_their_owner() {
    let _env = NotificationEnv::default_env();
    request::<App, _, _>(|request, ctx| async move {
        let first = prepare_data::init_user_login_as(
            &request,
            &ctx,
            "owner",
            "owner@example.com",
            "owner-password",
        )
        .await;
        let second = prepare_data::init_user_login_as(
            &request,
            &ctx,
            "other",
            "other@example.com",
            "other-password",
        )
        .await;
        update_settings(&request, &first.token, enabled_payload()).await;

        let now = fixed_now();
        let mut item = new_item("Private task");
        item.remind_at = Some((now - Duration::minutes(1)).fixed_offset());
        items::Model::create_many(&ctx.db, first.user.id, vec![item])
            .await
            .unwrap();
        assert_eq!(scan::reminders(&ctx, now).await.unwrap(), 2);

        // The other user's settings row cannot make their own deliveries.
        let other_settings = user_settings::Model::find_by_user(&ctx.db, second.user.id)
            .await
            .unwrap();
        assert!(other_settings.is_none());

        let rows = DeliveryEntity::find()
            .filter(notification_deliveries::Column::UserId.eq(second.user.id))
            .all(&ctx.db)
            .await
            .unwrap();
        assert!(rows.is_empty());
    })
    .await;
}
