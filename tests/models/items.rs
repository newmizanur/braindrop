use braindrop::{
    app::App,
    models::items::{self, ItemPatch, ItemPriority, ItemStatus, ItemType, NewItem},
};
use loco_rs::testing::prelude::*;
use sea_orm::EntityTrait;
use serial_test::serial;

macro_rules! configure_insta {
    ($($expr:expr),*) => {
        let mut settings = insta::Settings::clone_current();
        settings.set_prepend_module_to_snapshot(false);
        let _guard = settings.bind_to_scope();
    };
}

fn new_item(title: &str) -> NewItem {
    NewItem {
        item_type: ItemType::Task,
        title: title.to_string(),
        content: Some("  body  ".to_string()),
        category: Some("  work  ".to_string()),
        tags: vec!["b".to_string(), " a ".to_string(), "a".to_string()],
        status: ItemStatus::Inbox,
        priority: ItemPriority::Normal,
        due_at: None,
        remind_at: None,
    }
}

/// The `Items` entity matches the table its migration created.
#[tokio::test]
#[serial]
async fn can_query_items() {
    configure_insta!();

    let boot = boot_test::<App>().await.unwrap();
    seed::<App>(&boot.app_context).await.unwrap();

    items::Entity::find()
        .all(&boot.app_context.db)
        .await
        .expect("`items` should be queryable — entity and migration must agree");
}

#[tokio::test]
#[serial]
async fn create_many_persists_normalized_items() {
    configure_insta!();

    let boot = boot_test::<App>().await.unwrap();
    seed::<App>(&boot.app_context).await.unwrap();

    let created = items::Model::create_many(
        &boot.app_context.db,
        1,
        vec![new_item("first"), new_item("second")],
    )
    .await
    .expect("batch insert should succeed");
    assert_eq!(created.len(), 2);
    assert_eq!(created[0].tags, Some(serde_json::json!(["a", "b"])));
    assert_eq!(created[0].status, "inbox");
    assert_eq!(created[0].priority, "normal");
    assert!(created[0].completed_at.is_none());

    // The second seeded user cannot see the first user's item.
    let error =
        items::Model::find_by_pid_for_user(&boot.app_context.db, 2, &created[0].pid.to_string())
            .await
            .expect_err("another user's item must not be visible");
    assert!(matches!(error, loco_rs::model::ModelError::EntityNotFound));
}

#[tokio::test]
#[serial]
async fn state_transitions_stamp_and_clear_timestamps() {
    configure_insta!();

    let boot = boot_test::<App>().await.unwrap();
    seed::<App>(&boot.app_context).await.unwrap();

    let created = items::Model::create_many(&boot.app_context.db, 1, vec![new_item("work")])
        .await
        .unwrap();
    let pid = created[0].pid.to_string();

    let completed = items::Model::complete_for_user(&boot.app_context.db, 1, &pid)
        .await
        .unwrap();
    assert_eq!(completed.status, "completed");
    assert!(completed.completed_at.is_some());

    let reopened = items::Model::reopen_for_user(&boot.app_context.db, 1, &pid)
        .await
        .unwrap();
    assert_eq!(reopened.status, "inbox");
    assert!(reopened.completed_at.is_none());

    let patched = items::Model::update_for_user(
        &boot.app_context.db,
        1,
        &pid,
        ItemPatch {
            content: Some(None),
            tags: Some(vec!["solo".to_string()]),
            status: Some(ItemStatus::Archived),
            ..Default::default()
        },
    )
    .await
    .unwrap();
    assert_eq!(patched.content, None);
    assert_eq!(patched.tags, Some(serde_json::json!(["solo"])));
    assert_eq!(patched.status, "archived");
    assert!(patched.archived_at.is_some());
}

#[tokio::test]
#[serial]
async fn delete_is_scoped_to_the_owner() {
    configure_insta!();

    let boot = boot_test::<App>().await.unwrap();
    seed::<App>(&boot.app_context).await.unwrap();

    let created = items::Model::create_many(&boot.app_context.db, 1, vec![new_item("keep")])
        .await
        .unwrap();
    let pid = created[0].pid.to_string();

    let error = items::Model::delete_for_user(&boot.app_context.db, 2, &pid)
        .await
        .expect_err("another user must not be able to delete the item");
    assert!(matches!(error, loco_rs::model::ModelError::EntityNotFound));

    items::Model::delete_for_user(&boot.app_context.db, 1, &pid)
        .await
        .expect("the owner can delete the item");
    assert!(
        items::Model::find_by_pid_for_user(&boot.app_context.db, 1, &pid)
            .await
            .is_err()
    );
}
