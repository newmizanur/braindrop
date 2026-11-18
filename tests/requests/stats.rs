use braindrop::{app::App, time};
use chrono::{Duration, Utc};
use chrono_tz::Tz;
use loco_rs::testing::prelude::*;
use serde_json::{json, Value};
use serial_test::serial;

use super::prepare_data;

async fn stats(request: &loco_rs::TestServer, token: &str) -> Value {
    let (key, value) = prepare_data::auth_header(token);
    let response = request.get("/api/stats").add_header(key, value).await;
    assert_eq!(response.status_code(), 200, "{}", response.text());
    serde_json::from_str(&response.text()).unwrap()
}

async fn create(request: &loco_rs::TestServer, token: &str, payload: Value) {
    let (key, value) = prepare_data::auth_header(token);
    let response = request
        .post("/api/items")
        .add_header(key, value)
        .json(&payload)
        .await;
    assert_eq!(response.status_code(), 201, "{}", response.text());
}

#[tokio::test]
#[serial]
async fn stats_require_authentication() {
    request::<App, _, _>(|request, _ctx| async move {
        assert_eq!(request.get("/api/stats").await.status_code(), 401);
    })
    .await;
}

#[tokio::test]
#[serial]
async fn stats_match_the_users_items() {
    request::<App, _, _>(|request, ctx| async move {
        let user = prepare_data::init_user_login(&request, &ctx).await;

        let now = Utc::now();
        let today = now.date_naive();
        let tomorrow = today + Duration::days(1);
        let (_, tomorrow_start) = time::day_bounds(Tz::UTC, today);
        let due_today = tomorrow_start - Duration::minutes(1);
        let yesterday = today - Duration::days(1);
        let (yesterday_start, _) = time::day_bounds(Tz::UTC, yesterday);
        let due_yesterday = yesterday_start + Duration::hours(12);
        let (tomorrow_day_start, _) = time::day_bounds(Tz::UTC, tomorrow);
        let due_tomorrow = tomorrow_day_start + Duration::hours(12);

        create(
            &request,
            &user.token,
            json!({ "title": "due today", "due_at": due_today }),
        )
        .await;
        create(
            &request,
            &user.token,
            json!({ "title": "overdue", "due_at": due_yesterday }),
        )
        .await;
        create(
            &request,
            &user.token,
            json!({
                "title": "upcoming",
                "status": "pending",
                "due_at": due_tomorrow
            }),
        )
        .await;
        create(
            &request,
            &user.token,
            json!({ "title": "a note", "type": "note" }),
        )
        .await;
        create(
            &request,
            &user.token,
            json!({ "title": "done", "status": "completed" }),
        )
        .await;

        let summary = stats(&request, &user.token).await;
        assert_eq!(summary["total"], 5);
        assert_eq!(summary["inbox"], 3);
        assert_eq!(summary["pending"], 1);
        assert_eq!(summary["completed"], 1);
        assert_eq!(summary["archived"], 0);
        assert_eq!(summary["due_today"], 1);
        assert_eq!(summary["overdue"], 1);
        assert_eq!(summary["upcoming"], 1);
        assert_eq!(summary["completed_today"], 1);
        assert_eq!(summary["completed_week"], 1);
        assert_eq!(summary["tasks"], 4);
        assert_eq!(summary["notes"], 1);
        assert_eq!(summary["ideas"], 0);
        assert_eq!(summary["reminders"], 0);
    })
    .await;
}

#[tokio::test]
#[serial]
async fn stats_use_the_users_timezone_for_day_boundaries() {
    request::<App, _, _>(|request, ctx| async move {
        let user = prepare_data::init_user_login(&request, &ctx).await;

        // Set a timezone 14 hours ahead of UTC, where "today" can be a
        // different calendar day than it is in UTC.
        let (key, value) = prepare_data::auth_header(&user.token);
        let response = request
            .patch("/api/settings")
            .add_header(key, value)
            .json(&json!({ "timezone": "Pacific/Kiritimati" }))
            .await;
        assert_eq!(response.status_code(), 200);

        let timezone = Tz::Pacific__Kiritimati;
        let local_today = Utc::now().with_timezone(&timezone).date_naive();
        // One minute into the local day: inside Kiritimati's today, but
        // before the start of the local day in a timezone far to the west.
        let due = time::start_of_day(timezone, local_today) + Duration::minutes(1);

        create(
            &request,
            &user.token,
            json!({ "title": "local today", "due_at": due }),
        )
        .await;

        let summary = stats(&request, &user.token).await;
        assert_eq!(
            summary["due_today"], 1,
            "the item is due today in Kiritimati"
        );

        // The same instant falls on a different local day far to the west.
        let (key, value) = prepare_data::auth_header(&user.token);
        let response = request
            .patch("/api/settings")
            .add_header(key, value)
            .json(&json!({ "timezone": "Pacific/Midway" }))
            .await;
        assert_eq!(response.status_code(), 200);

        let summary = stats(&request, &user.token).await;
        assert_eq!(
            summary["due_today"], 0,
            "day boundaries must follow the configured timezone, not UTC"
        );
    })
    .await;
}
