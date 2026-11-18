use braindrop::{
    app::App,
    dtos::items::{CreateItemsResponse, ItemResponse},
    models::items::{ItemPriority, ItemStatus, ItemType},
};
use loco_rs::testing::prelude::*;
use serde_json::json;
use serial_test::serial;

use super::prepare_data;

async fn create_items(
    request: &loco_rs::TestServer,
    token: &str,
    payload: serde_json::Value,
) -> CreateItemsResponse {
    let (key, value) = prepare_data::auth_header(token);
    let response = request
        .post("/api/items")
        .add_header(key, value)
        .json(&payload)
        .await;
    assert_eq!(response.status_code(), 201, "{}", response.text());
    serde_json::from_str(&response.text()).expect("create response should be typed JSON")
}

fn item_body(title: &str) -> serde_json::Value {
    json!({ "title": title })
}

#[tokio::test]
#[serial]
async fn sorting_by_title_does_not_put_undated_items_last() {
    request::<App, _, _>(|request, ctx| async move {
        let user = prepare_data::init_user_login(&request, &ctx).await;

        create_items(
            &request,
            &user.token,
            json!({
                "items": [
                    { "title": "Zulu" },
                    { "title": "Alpha", "due_at": "2026-12-01T09:00:00Z" },
                    { "title": "Mike" }
                ]
            }),
        )
        .await;

        let (key, value) = prepare_data::auth_header(&user.token);
        let response = request
            .get("/api/items?sort=title&order=asc")
            .add_header(key, value)
            .await;
        assert_eq!(response.status_code(), 200, "{}", response.text());
        let page: serde_json::Value = serde_json::from_str(&response.text()).unwrap();
        let titles = page["items"]
            .as_array()
            .unwrap()
            .iter()
            .map(|item| item["title"].as_str().unwrap())
            .collect::<Vec<_>>();
        assert_eq!(titles, vec!["Alpha", "Mike", "Zulu"]);
    })
    .await;
}

#[tokio::test]
#[serial]
async fn items_require_authentication() {
    request::<App, _, _>(|request, _ctx| async move {
        let response = request.get("/api/items").await;
        assert_eq!(response.status_code(), 401);
    })
    .await;
}

#[tokio::test]
#[serial]
async fn can_create_list_show_update_and_delete_an_item() {
    request::<App, _, _>(|request, ctx| async move {
        let user = prepare_data::init_user_login(&request, &ctx).await;

        let created = create_items(
            &request,
            &user.token,
            json!({
                "title": "  Buy milk  ",
                "type": "task",
                "content": "  semi-skimmed  ",
                "category": "  errands ",
                "tags": ["home", " home ", "", "shopping"],
                "due_at": "2026-12-01T09:00:00Z"
            }),
        )
        .await;
        assert_eq!(created.items.len(), 1);
        let item = &created.items[0];
        assert_eq!(item.title, "Buy milk");
        assert_eq!(item.item_type, ItemType::Task);
        assert_eq!(item.status, ItemStatus::Inbox);
        assert_eq!(item.priority, ItemPriority::Normal);
        assert_eq!(item.content.as_deref(), Some("semi-skimmed"));
        assert_eq!(item.category.as_deref(), Some("errands"));
        assert_eq!(item.tags, vec!["home", "shopping"]);
        assert!(item.completed_at.is_none());

        let (key, value) = prepare_data::auth_header(&user.token);
        let list = request.get("/api/items").add_header(key, value).await;
        assert_eq!(list.status_code(), 200);
        let list: serde_json::Value = serde_json::from_str(&list.text()).unwrap();
        assert_eq!(list["total_items"], 1);
        assert_eq!(list["items"][0]["pid"], item.pid);

        let (key, value) = prepare_data::auth_header(&user.token);
        let show = request
            .get(&format!("/api/items/{}", item.pid))
            .add_header(key, value)
            .await;
        assert_eq!(show.status_code(), 200);
        let shown: ItemResponse = serde_json::from_str(&show.text()).unwrap();
        assert_eq!(shown.pid, item.pid);

        let (key, value) = prepare_data::auth_header(&user.token);
        let patched = request
            .patch(&format!("/api/items/{}", item.pid))
            .add_header(key, value)
            .json(&json!({
                "title": "Buy oat milk",
                "priority": "high",
                "content": null,
                "tags": ["errands"]
            }))
            .await;
        assert_eq!(patched.status_code(), 200, "{}", patched.text());
        let patched: ItemResponse = serde_json::from_str(&patched.text()).unwrap();
        assert_eq!(patched.title, "Buy oat milk");
        assert_eq!(patched.priority, ItemPriority::High);
        assert_eq!(patched.content, None);
        assert_eq!(patched.tags, vec!["errands"]);

        // Completing stamps a timestamp; reopening clears it.
        let (key, value) = prepare_data::auth_header(&user.token);
        let completed = request
            .patch(&format!("/api/items/{}", item.pid))
            .add_header(key, value)
            .json(&json!({ "status": "completed" }))
            .await;
        let completed: ItemResponse = serde_json::from_str(&completed.text()).unwrap();
        assert_eq!(completed.status, ItemStatus::Completed);
        assert!(completed.completed_at.is_some());

        let (key, value) = prepare_data::auth_header(&user.token);
        let reopened = request
            .patch(&format!("/api/items/{}", item.pid))
            .add_header(key, value)
            .json(&json!({ "status": "pending" }))
            .await;
        let reopened: ItemResponse = serde_json::from_str(&reopened.text()).unwrap();
        assert_eq!(reopened.status, ItemStatus::Pending);
        assert!(reopened.completed_at.is_none());

        let (key, value) = prepare_data::auth_header(&user.token);
        let deleted = request
            .delete(&format!("/api/items/{}", item.pid))
            .add_header(key, value)
            .await;
        assert_eq!(deleted.status_code(), 204);

        let (key, value) = prepare_data::auth_header(&user.token);
        let missing = request
            .get(&format!("/api/items/{}", item.pid))
            .add_header(key, value)
            .await;
        assert_eq!(missing.status_code(), 404);
    })
    .await;
}

#[tokio::test]
#[serial]
async fn timestamps_are_normalized_to_utc() {
    request::<App, _, _>(|request, ctx| async move {
        let user = prepare_data::init_user_login(&request, &ctx).await;

        let created = create_items(
            &request,
            &user.token,
            json!({
                "title": "Offset deadline",
                "due_at": "2026-12-01T09:00:00+02:00",
                "remind_at": "2026-12-02T09:00:00-05:00"
            }),
        )
        .await;
        let item = &created.items[0];
        let due = item.due_at.expect("due date should be stored");
        let remind = item.remind_at.expect("reminder should be stored");

        assert_eq!(due.offset().local_minus_utc(), 0);
        assert_eq!(
            due.with_timezone(&chrono::Utc).to_rfc3339(),
            "2026-12-01T07:00:00+00:00"
        );
        assert_eq!(remind.offset().local_minus_utc(), 0);
        assert_eq!(
            remind.with_timezone(&chrono::Utc).to_rfc3339(),
            "2026-12-02T14:00:00+00:00"
        );
    })
    .await;
}

#[tokio::test]
#[serial]
async fn batch_creation_is_atomic() {
    request::<App, _, _>(|request, ctx| async move {
        let user = prepare_data::init_user_login(&request, &ctx).await;

        let (key, value) = prepare_data::auth_header(&user.token);
        let response = request
            .post("/api/items")
            .add_header(key, value)
            .json(&json!({
                "items": [
                    { "title": "valid" },
                    { "title": "   " }
                ]
            }))
            .await;
        assert_eq!(response.status_code(), 422, "{}", response.text());

        let (key, value) = prepare_data::auth_header(&user.token);
        let list = request.get("/api/items").add_header(key, value).await;
        let list: serde_json::Value = serde_json::from_str(&list.text()).unwrap();
        assert_eq!(
            list["total_items"], 0,
            "a rejected batch must not leave partial rows"
        );

        let created = create_items(
            &request,
            &user.token,
            json!({
                "items": [
                    { "title": "first", "type": "note" },
                    { "title": "second", "priority": "urgent" }
                ]
            }),
        )
        .await;
        assert_eq!(created.items.len(), 2);
        assert_eq!(created.items[0].item_type, ItemType::Note);
        assert_eq!(created.items[1].priority, ItemPriority::Urgent);
    })
    .await;
}

#[tokio::test]
#[serial]
async fn another_users_item_is_invisible() {
    request::<App, _, _>(|request, ctx| async move {
        let owner = prepare_data::init_user_login_as(
            &request,
            &ctx,
            "owner",
            "owner@example.com",
            "owner-password",
        )
        .await;
        let other = prepare_data::init_user_login_as(
            &request,
            &ctx,
            "other",
            "other@example.com",
            "other-password",
        )
        .await;

        let created = create_items(&request, &owner.token, item_body("private")).await;
        let pid = &created.items[0].pid;

        let (key, value) = prepare_data::auth_header(&other.token);
        let show = request
            .get(&format!("/api/items/{pid}"))
            .add_header(key, value)
            .await;
        assert_eq!(show.status_code(), 404);

        let (key, value) = prepare_data::auth_header(&other.token);
        let patch = request
            .patch(&format!("/api/items/{pid}"))
            .add_header(key, value)
            .json(&json!({ "title": "stolen" }))
            .await;
        assert_eq!(patch.status_code(), 404);

        let (key, value) = prepare_data::auth_header(&other.token);
        let delete = request
            .delete(&format!("/api/items/{pid}"))
            .add_header(key, value)
            .await;
        assert_eq!(delete.status_code(), 404);

        let (key, value) = prepare_data::auth_header(&other.token);
        let list = request.get("/api/items").add_header(key, value).await;
        let list: serde_json::Value = serde_json::from_str(&list.text()).unwrap();
        assert_eq!(list["total_items"], 0);

        // The owner still sees the untouched item.
        let (key, value) = prepare_data::auth_header(&owner.token);
        let show = request
            .get(&format!("/api/items/{pid}"))
            .add_header(key, value)
            .await;
        assert_eq!(show.status_code(), 200);
    })
    .await;
}

#[tokio::test]
#[serial]
async fn invalid_input_is_rejected() {
    request::<App, _, _>(|request, ctx| async move {
        let user = prepare_data::init_user_login(&request, &ctx).await;

        for payload in [
            json!({ "title": "" }),
            json!({ "title": "   " }),
            json!({ "title": "x".repeat(501) }),
        ] {
            let (key, value) = prepare_data::auth_header(&user.token);
            let response = request
                .post("/api/items")
                .add_header(key, value)
                .json(&payload)
                .await;
            assert_eq!(response.status_code(), 422, "{}", response.text());
        }

        let (key, value) = prepare_data::auth_header(&user.token);
        let response = request
            .post("/api/items")
            .add_header(key, value)
            .json(&json!({ "items": [] }))
            .await;
        assert_eq!(response.status_code(), 422);

        let (key, value) = prepare_data::auth_header(&user.token);
        let response = request
            .post("/api/items")
            .add_header(key, value)
            .json(&json!({ "title": "ok", "type": "not-a-type" }))
            .await;
        assert_eq!(response.status_code(), 422);

        // Malformed ids are not found, not a 500.
        let (key, value) = prepare_data::auth_header(&user.token);
        let response = request
            .get("/api/items/not-a-uuid")
            .add_header(key, value)
            .await;
        assert_eq!(response.status_code(), 404);
    })
    .await;
}

#[tokio::test]
#[serial]
async fn filters_search_sort_and_paginate() {
    request::<App, _, _>(|request, ctx| async move {
        let user = prepare_data::init_user_login(&request, &ctx).await;

        create_items(
            &request,
            &user.token,
            json!({
                "items": [
                    {
                        "title": "Alpha report",
                        "type": "task",
                        "category": "work",
                        "tags": ["deep"],
                        "priority": "urgent",
                        "due_at": "2026-12-01T09:00:00Z"
                    },
                    {
                        "title": "Beta idea",
                        "type": "idea",
                        "category": "home",
                        "tags": ["shower"],
                        "priority": "low",
                        "due_at": "2026-12-02T09:00:00Z"
                    },
                    {
                        "title": "Gamma note",
                        "type": "note",
                        "content": "remember the alpha launch",
                        "category": "work",
                        "tags": ["deep", "writing"],
                        "priority": "normal",
                        "due_at": "2026-12-03T09:00:00Z"
                    }
                ]
            }),
        )
        .await;

        let list = |query: &str| {
            let request = &request;
            let token = &user.token;
            let query = query.to_string();
            async move {
                let (key, value) = prepare_data::auth_header(token);
                let response = request
                    .get(&format!("/api/items?{query}"))
                    .add_header(key, value)
                    .await;
                assert_eq!(response.status_code(), 200, "{}", response.text());
                serde_json::from_str::<serde_json::Value>(&response.text()).unwrap()
            }
        };

        let found = list("q=alpha").await;
        assert_eq!(found["total_items"], 2);

        let by_category = list("category=home").await;
        assert_eq!(by_category["total_items"], 1);
        assert_eq!(by_category["items"][0]["title"], "Beta idea");

        let by_tag = list("tag=deep").await;
        assert_eq!(by_tag["total_items"], 2);

        let by_type = list("type=idea").await;
        assert_eq!(by_type["total_items"], 1);

        let by_status = list("status=inbox,pending").await;
        assert_eq!(by_status["total_items"], 3);

        let by_due = list("due_from=2026-12-02T00:00:00Z&due_to=2026-12-03T00:00:00Z").await;
        assert_eq!(by_due["total_items"], 1);
        assert_eq!(by_due["items"][0]["title"], "Beta idea");

        let sorted = list("sort=title&order=asc").await;
        assert_eq!(sorted["items"][0]["title"], "Alpha report");
        assert_eq!(sorted["items"][2]["title"], "Gamma note");

        let page = list("page=1&page_size=2&sort=title&order=asc").await;
        assert_eq!(page["total_items"], 3);
        assert_eq!(page["page"], 1);
        assert_eq!(page["page_size"], 2);
        assert_eq!(page["total_pages"], 2);
        assert_eq!(page["items"].as_array().unwrap().len(), 2);

        let second = list("page=2&page_size=2&sort=title&order=asc").await;
        assert_eq!(second["items"].as_array().unwrap().len(), 1);

        let (key, value) = prepare_data::auth_header(&user.token);
        let unknown_status = request
            .get("/api/items?status=does-not-exist")
            .add_header(key, value)
            .await;
        assert_eq!(unknown_status.status_code(), 422);
    })
    .await;
}
