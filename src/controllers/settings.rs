use axum::http::StatusCode;
use loco_rs::{controller::ErrorDetail, prelude::*};

use crate::{
    ai::config::AiSettings,
    dtos::settings::{SettingsResponse, UpdateSettingsParams},
    models::{user_settings, users},
    notifications::config::NotificationSettings,
};

fn validation_error(message: String) -> Error {
    Error::CustomError(
        StatusCode::UNPROCESSABLE_ENTITY,
        ErrorDetail::new("validation_error", message),
    )
}

async fn current_user(ctx: &AppContext, auth: &auth::JWT) -> Result<users::Model> {
    users::Model::find_by_pid(&ctx.db, &auth.claims.pid)
        .await
        .map_err(Error::from)
}

#[debug_handler]
async fn show(auth: auth::JWT, State(ctx): State<AppContext>) -> Result<Response> {
    let user = current_user(&ctx, &auth).await?;
    let settings = user_settings::Model::get_or_create(&ctx.db, user.id).await?;
    let config = AiSettings::from_config(&ctx.config)?;
    format::json(SettingsResponse::new(&settings, &config))
}

#[debug_handler]
async fn update(
    auth: auth::JWT,
    State(ctx): State<AppContext>,
    Json(params): Json<UpdateSettingsParams>,
) -> Result<Response> {
    let user = current_user(&ctx, &auth).await?;
    let ai = AiSettings::from_config(&ctx.config)?;
    let notifications = NotificationSettings::from_config(&ctx.config)?;
    let patch = params
        .into_patch(&ai, &notifications)
        .map_err(validation_error)?;
    let settings = user_settings::Model::update_for_user(&ctx.db, user.id, patch).await?;
    format::json(SettingsResponse::new(&settings, &ai))
}

pub fn routes() -> Routes {
    Routes::new().add("/api/settings", get(show).patch(update))
}
