use crate::{
    auth_policy::AuthPolicy,
    auth_rate_limit::{self, Endpoint as RateLimitEndpoint},
    mailers::auth::AuthMailer,
    models::{
        _entities::users,
        users::{LoginParams, RegisterParams},
    },
    views::auth::{CurrentResponse, LoginResponse},
};
use axum::http::StatusCode;
use loco_rs::{controller::ErrorDetail, hash, prelude::*};
use serde::{Deserialize, Serialize};
use std::sync::OnceLock;

#[derive(Debug, Deserialize, Serialize)]
pub struct ForgotParams {
    pub email: String,
}

#[derive(Debug, Validate, Deserialize, Serialize)]
pub struct ResetParams {
    pub token: String,
    #[validate(length(min = 8, message = "password must be at least 8 characters"))]
    pub password: String,
}

#[derive(Debug, Deserialize, Serialize)]
pub struct MagicLinkParams {
    pub email: String,
}

#[derive(Debug, Deserialize, Serialize)]
pub struct ResendVerificationParams {
    pub email: String,
}

#[derive(Debug, Deserialize, Serialize)]
pub struct AuthConfigResponse {
    pub allow_registration: bool,
    pub require_email_verification: bool,
}

/// Return a 403 with a stable machine-readable code and a user-safe message.
fn forbidden<U>(code: &str, message: &str) -> Result<U> {
    Err(Error::CustomError(
        StatusCode::FORBIDDEN,
        ErrorDetail::new(code, message),
    ))
}

/// A stable hash used to keep unknown-email logins as expensive as real ones.
fn dummy_password_hash() -> &'static str {
    static HASH: OnceLock<String> = OnceLock::new();
    HASH.get_or_init(|| {
        hash::hash_password("braindrop-dummy-password-for-timing-parity").unwrap_or_default()
    })
}

/// Sign a session token for `user`.
///
/// # Errors
///
/// Returns a 500 when the configured JWT secret cannot be used for signing.
/// That is a server misconfiguration, not a failed login, so it must not be
/// reported as a 401.
fn issue_token(ctx: &AppContext, user: &users::Model) -> Result<String> {
    let jwt_secret = ctx.config.get_jwt_config()?;
    user.generate_jwt(&jwt_secret.secret, jwt_secret.expiration)
        .map_err(|err| {
            tracing::error!(
                message = err.to_string(),
                user_pid = user.pid.to_string(),
                "could not sign a session token; check that JWT_SECRET is valid base64"
            );
            Error::InternalServerError
        })
}

/// Register function creates a new user with the given parameters and sends a
/// welcome email to the user
#[debug_handler]
async fn register(
    State(ctx): State<AppContext>,
    remote_ip: RemoteIP,
    JsonValidate(params): JsonValidate<RegisterParams>,
) -> Result<Response> {
    let policy = AuthPolicy::from_config(&ctx.config)?;
    if !policy.allow_registration {
        tracing::info!("registration rejected: disabled by policy");
        return forbidden(
            "registration_disabled",
            "Public registration is disabled. Ask an administrator to create an account.",
        );
    }

    auth_rate_limit::check(&ctx, RateLimitEndpoint::Register, &remote_ip, &params.email).await?;

    let res = users::Model::create_with_password(&ctx.db, &params).await;

    let user = match res {
        Ok(user) => user,
        Err(ModelError::EntityAlreadyExists) => {
            // Keep the same response as a fresh registration. The SPA already
            // uses neutral "check your email" copy so this endpoint cannot be
            // used as an account oracle; only the server log records the fact.
            tracing::info!("duplicate registration attempt");
            return format::json(());
        }
        Err(err) => {
            tracing::info!(message = err.to_string(), "could not register user");
            return Err(Error::InternalServerError);
        }
    };

    let user = user
        .into_active_model()
        .set_email_verification_sent(&ctx.db)
        .await?;

    AuthMailer::send_welcome(&ctx, &user).await?;

    format::json(())
}

/// Verify register user. if the user not verified his email, he can't login to
/// the system.
#[debug_handler]
async fn verify(
    State(ctx): State<AppContext>,
    remote_ip: RemoteIP,
    Path(token): Path<String>,
) -> Result<Response> {
    auth_rate_limit::check(&ctx, RateLimitEndpoint::Verify, &remote_ip, &token).await?;
    let Ok(user) = users::Model::find_by_verification_token(&ctx.db, &token).await else {
        return unauthorized("invalid token");
    };

    if user.email_verified_at.is_some() {
        tracing::info!(pid = user.pid.to_string(), "user already verified");
    } else {
        let active_model = user.into_active_model();
        let user = active_model.verified(&ctx.db).await?;
        tracing::info!(pid = user.pid.to_string(), "user verified");
    }

    format::json(())
}

/// In case the user forgot his password  this endpoints generate a forgot token
/// and send email to the user. In case the email not found in our DB, we are
/// returning a valid request for security reasons (not exposing users DB
/// list).
#[debug_handler]
async fn forgot(
    State(ctx): State<AppContext>,
    remote_ip: RemoteIP,
    Json(params): Json<ForgotParams>,
) -> Result<Response> {
    auth_rate_limit::check(&ctx, RateLimitEndpoint::Forgot, &remote_ip, &params.email).await?;
    let Ok(user) = users::Model::find_by_email(&ctx.db, &params.email).await else {
        // we don't want to expose our users email. if the email is invalid we still
        // returning success to the caller
        return format::json(());
    };

    let user = user
        .into_active_model()
        .set_forgot_password_sent(&ctx.db)
        .await?;

    AuthMailer::forgot_password(&ctx, &user).await?;

    format::json(())
}

/// reset user password by the given parameters
#[debug_handler]
async fn reset(
    State(ctx): State<AppContext>,
    remote_ip: RemoteIP,
    JsonValidate(params): JsonValidate<ResetParams>,
) -> Result<Response> {
    auth_rate_limit::check(&ctx, RateLimitEndpoint::Reset, &remote_ip, &params.token).await?;
    let Ok(user) = users::Model::find_by_reset_token(&ctx.db, &params.token).await else {
        // we don't want to expose our users email. if the email is invalid we still
        // returning success to the caller
        tracing::info!("reset token not found");

        return format::json(());
    };
    user.into_active_model()
        .reset_password(&ctx.db, &params.password)
        .await?;

    format::json(())
}

/// Creates a user login and returns a token
#[debug_handler]
async fn login(
    State(ctx): State<AppContext>,
    remote_ip: RemoteIP,
    Json(params): Json<LoginParams>,
) -> Result<Response> {
    let Ok(user) = users::Model::find_by_email(&ctx.db, &params.email).await else {
        // Keep the response time close to a real password check so callers
        // cannot enumerate accounts by timing.
        let _ = hash::verify_password(&params.password, dummy_password_hash());
        auth_rate_limit::check(&ctx, RateLimitEndpoint::Login, &remote_ip, &params.email).await?;
        tracing::debug!("login attempt with non-existent email");
        return unauthorized("Invalid credentials!");
    };

    if !user.verify_password(&params.password) {
        // Only failed logins consume the budget. A correct password is always
        // accepted and clears the counter below, so knowing someone's email
        // cannot be used to keep them locked out.
        auth_rate_limit::check(&ctx, RateLimitEndpoint::Login, &remote_ip, &params.email).await?;
        return unauthorized("Invalid credentials!");
    }

    let policy = AuthPolicy::from_config(&ctx.config)?;
    if policy.require_email_verification && user.email_verified_at.is_none() {
        tracing::info!(
            pid = user.pid.to_string(),
            "login rejected: email verification required"
        );
        return forbidden(
            "email_verification_required",
            "Verify your email address before logging in.",
        );
    }

    let token = issue_token(&ctx, &user)?;
    auth_rate_limit::reset(&ctx, RateLimitEndpoint::Login, &remote_ip, &params.email).await;

    format::json(LoginResponse::new(&user, &token))
}

#[debug_handler]
async fn current(auth: auth::JWT, State(ctx): State<AppContext>) -> Result<Response> {
    let user = users::Model::find_by_pid(&ctx.db, &auth.claims.pid).await?;
    format::json(CurrentResponse::new(&user))
}

/// Public authentication policy for the SPA. Deliberately outside the ten
/// business endpoints and safe to call without a token.
#[debug_handler]
async fn auth_config(State(ctx): State<AppContext>) -> Result<Response> {
    let policy = AuthPolicy::from_config(&ctx.config)?;
    format::json(AuthConfigResponse {
        allow_registration: policy.allow_registration,
        require_email_verification: policy.require_email_verification,
    })
}

/// Magic link authentication provides a secure and passwordless way to log in to the application.
///
/// # Flow
/// 1. **Request a Magic Link**:
///    A registered user sends a POST request to `/magic-link` with their email.
///    If the email exists, a short-lived, one-time-use token is generated and sent to the user's email.
///    For security and to avoid exposing whether an email exists, the response always returns 200, even if the email is invalid.
///
/// 2. **Click the Magic Link**:
///    The user clicks the link (/magic-link/{token}), which validates the token and its expiration.
///    If valid, the server generates a JWT and responds with a [`LoginResponse`].
///    If invalid or expired, an unauthorized response is returned.
///
/// This flow enhances security by avoiding traditional passwords and providing a seamless login experience.
async fn magic_link(
    State(ctx): State<AppContext>,
    remote_ip: RemoteIP,
    Json(params): Json<MagicLinkParams>,
) -> Result<Response> {
    auth_rate_limit::check(
        &ctx,
        RateLimitEndpoint::MagicLink,
        &remote_ip,
        &params.email,
    )
    .await?;

    let Ok(user) = users::Model::find_by_email(&ctx.db, &params.email).await else {
        // we don't want to expose our users email. if the email is invalid we still
        // returning success to the caller
        tracing::debug!("user not found by email");
        return format::empty_json();
    };

    let user = user.into_active_model().create_magic_link(&ctx.db).await?;
    AuthMailer::send_magic_link(&ctx, &user).await?;

    format::empty_json()
}

/// Verifies a magic link token and authenticates the user.
async fn magic_link_verify(
    Path(magic_token): Path<String>,
    State(ctx): State<AppContext>,
    remote_ip: RemoteIP,
) -> Result<Response> {
    auth_rate_limit::check(
        &ctx,
        RateLimitEndpoint::MagicLinkVerify,
        &remote_ip,
        &magic_token,
    )
    .await?;
    let Ok(user) = users::Model::find_by_magic_token(&ctx.db, &magic_token).await else {
        // we don't want to expose our users email. if the email is invalid we still
        // returning success to the caller
        return unauthorized("unauthorized!");
    };

    let policy = AuthPolicy::from_config(&ctx.config)?;
    if policy.require_email_verification && user.email_verified_at.is_none() {
        tracing::info!(
            pid = user.pid.to_string(),
            "magic link rejected: email verification required"
        );
        // Consume the one-time link even when it cannot be used yet. Otherwise
        // verifying the address by email would make the old link valid.
        user.into_active_model().clear_magic_link(&ctx.db).await?;
        return forbidden(
            "email_verification_required",
            "Verify your email address before logging in.",
        );
    }

    let user = user.into_active_model().clear_magic_link(&ctx.db).await?;

    let token = issue_token(&ctx, &user)?;
    auth_rate_limit::reset(
        &ctx,
        RateLimitEndpoint::MagicLinkVerify,
        &remote_ip,
        &magic_token,
    )
    .await;

    format::json(LoginResponse::new(&user, &token))
}

#[debug_handler]
async fn resend_verification_email(
    State(ctx): State<AppContext>,
    remote_ip: RemoteIP,
    Json(params): Json<ResendVerificationParams>,
) -> Result<Response> {
    auth_rate_limit::check(
        &ctx,
        RateLimitEndpoint::ResendVerification,
        &remote_ip,
        &params.email,
    )
    .await?;
    let Ok(user) = users::Model::find_by_email(&ctx.db, &params.email).await else {
        tracing::info!("User not found for resend verification");
        return format::json(());
    };

    if user.email_verified_at.is_some() {
        tracing::info!(
            pid = user.pid.to_string(),
            "User already verified, skipping resend"
        );
        return format::json(());
    }

    let user = user
        .into_active_model()
        .set_email_verification_sent(&ctx.db)
        .await?;

    AuthMailer::send_welcome(&ctx, &user).await?;
    tracing::info!(pid = user.pid.to_string(), "Verification email re-sent");

    format::json(())
}

pub fn routes() -> Routes {
    Routes::new()
        .prefix("/api/auth")
        .add("/register", post(register))
        .add("/verify/{token}", get(verify))
        .add("/login", post(login))
        .add("/forgot", post(forgot))
        .add("/reset", post(reset))
        .add("/current", get(current))
        .add("/config", get(auth_config))
        .add("/magic-link", post(magic_link))
        .add("/magic-link/{token}", get(magic_link_verify))
        .add("/resend-verification-mail", post(resend_verification_email))
}
