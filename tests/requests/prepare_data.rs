use axum::http::{HeaderName, HeaderValue};
use braindrop::{models::users, views::auth::LoginResponse};
use loco_rs::{app::AppContext, TestServer};

const USER_EMAIL: &str = "test@loco.com";
const USER_PASSWORD: &str = "12341234";

pub struct LoggedInUser {
    pub user: users::Model,
    pub token: String,
}

pub async fn init_user_login(request: &TestServer, ctx: &AppContext) -> LoggedInUser {
    init_user_login_as(request, ctx, "loco", USER_EMAIL, USER_PASSWORD).await
}

/// Register, verify and log in a user with an explicit identity.
///
/// The default `init_user_login` uses a fixed account; tests that need a
/// second user to prove isolation use this variant.
pub async fn init_user_login_as(
    request: &TestServer,
    ctx: &AppContext,
    name: &str,
    email: &str,
    password: &str,
) -> LoggedInUser {
    let register_payload = serde_json::json!({
        "name": name,
        "email": email,
        "password": password
    });

    //Creating a new user
    request
        .post("/api/auth/register")
        .json(&register_payload)
        .await;
    let user = users::Model::find_by_email(&ctx.db, email).await.unwrap();

    if let Some(token) = user.email_verification_token.clone() {
        let verify_payload = serde_json::json!({
            "token": token,
        });
        request.post("/api/auth/verify").json(&verify_payload).await;
    }

    let response = request
        .post("/api/auth/login")
        .json(&serde_json::json!({
            "email": email,
            "password": password
        }))
        .await;

    let login_response: LoginResponse = serde_json::from_str(&response.text()).unwrap();

    LoggedInUser {
        user: users::Model::find_by_email(&ctx.db, email).await.unwrap(),
        token: login_response.token,
    }
}

pub fn auth_header(token: &str) -> (HeaderName, HeaderValue) {
    let auth_header_value = HeaderValue::from_str(&format!("Bearer {token}")).unwrap();

    (HeaderName::from_static("authorization"), auth_header_value)
}
