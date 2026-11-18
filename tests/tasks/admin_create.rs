use braindrop::{app::App, models::users};
use loco_rs::{boot::run_task, task, testing::prelude::*};
use serial_test::serial;

fn vars(email: &str, password: &str, extra: Vec<(&str, &str)>) -> task::Vars {
    let mut args = vec![
        ("email".to_string(), email.to_string()),
        ("name".to_string(), "Admin User".to_string()),
        ("password".to_string(), password.to_string()),
    ];
    args.extend(
        extra
            .into_iter()
            .map(|(key, value)| (key.to_string(), value.to_string())),
    );
    task::Vars::from_cli_args(args)
}

#[tokio::test]
#[serial]
async fn admin_create_creates_a_verified_account() {
    let boot = boot_test::<App>().await.unwrap();

    run_task::<App>(
        &boot.app_context,
        Some(&"admin:create".to_string()),
        &vars("admin@example.com", "first-password", vec![]),
    )
    .await
    .expect("admin:create should succeed");

    let user = users::Model::find_by_email(&boot.app_context.db, "admin@example.com")
        .await
        .expect("administrator should exist");
    assert!(user.email_verified_at.is_some(), "admin is verified");
    assert!(user.verify_password("first-password"));
    assert_eq!(
        boot.app_context.mailer.unwrap().deliveries().count,
        0,
        "a verified admin does not need a welcome email"
    );
}

#[tokio::test]
#[serial]
async fn admin_create_requires_force_for_an_existing_account() {
    let boot = boot_test::<App>().await.unwrap();
    let vars = vars("admin@example.com", "first-password", vec![]);

    run_task::<App>(&boot.app_context, Some(&"admin:create".to_string()), &vars)
        .await
        .expect("first creation succeeds");

    let err = run_task::<App>(&boot.app_context, Some(&"admin:create".to_string()), &vars)
        .await
        .expect_err("second creation without force must fail");
    assert!(
        err.to_string().contains("force:true"),
        "error should explain how to proceed: {err}"
    );
}

#[tokio::test]
#[serial]
async fn admin_create_force_resets_password_and_keeps_verification() {
    let boot = boot_test::<App>().await.unwrap();

    run_task::<App>(
        &boot.app_context,
        Some(&"admin:create".to_string()),
        &vars("admin@example.com", "first-password", vec![]),
    )
    .await
    .expect("first creation succeeds");

    run_task::<App>(
        &boot.app_context,
        Some(&"admin:create".to_string()),
        &vars(
            "admin@example.com",
            "second-password",
            vec![("force", "true")],
        ),
    )
    .await
    .expect("forced update succeeds");

    let user = users::Model::find_by_email(&boot.app_context.db, "admin@example.com")
        .await
        .expect("administrator should exist");
    assert!(user.verify_password("second-password"));
    assert!(
        user.email_verified_at.is_some(),
        "force verifies by default"
    );
}

#[tokio::test]
#[serial]
async fn admin_create_force_can_unverify_an_existing_account() {
    let boot = boot_test::<App>().await.unwrap();

    run_task::<App>(
        &boot.app_context,
        Some(&"admin:create".to_string()),
        &vars("admin@example.com", "first-password", vec![]),
    )
    .await
    .expect("first creation succeeds");

    run_task::<App>(
        &boot.app_context,
        Some(&"admin:create".to_string()),
        &vars(
            "admin@example.com",
            "second-password",
            vec![("force", "true"), ("verified", "false")],
        ),
    )
    .await
    .expect("forced unverify succeeds");

    let user = users::Model::find_by_email(&boot.app_context.db, "admin@example.com")
        .await
        .expect("administrator should exist");
    assert!(user.verify_password("second-password"));
    assert!(
        user.email_verified_at.is_none(),
        "verified:false clears an existing verification"
    );
    assert!(user.email_verification_token.is_some());
}

#[tokio::test]
#[serial]
async fn admin_create_can_leave_an_account_unverified() {
    let boot = boot_test::<App>().await.unwrap();

    run_task::<App>(
        &boot.app_context,
        Some(&"admin:create".to_string()),
        &vars(
            "pending@example.com",
            "first-password",
            vec![("verified", "false")],
        ),
    )
    .await
    .expect("admin:create should succeed");

    let user = users::Model::find_by_email(&boot.app_context.db, "pending@example.com")
        .await
        .expect("account should exist");
    assert!(user.email_verified_at.is_none());
    assert!(user.email_verification_token.is_some());
    assert_eq!(
        boot.app_context.mailer.unwrap().deliveries().count,
        1,
        "unverified accounts receive a verification email"
    );
}
