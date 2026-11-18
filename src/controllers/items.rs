use axum::http::StatusCode;
use loco_rs::{controller::ErrorDetail, prelude::*};

use crate::{
    dtos::{
        common::Page,
        items::{
            CreateItemsRequest, CreateItemsResponse, ItemResponse, ListItemsQuery, UpdateItemParams,
        },
    },
    models::{items, user_settings, users},
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
async fn list(
    auth: auth::JWT,
    State(ctx): State<AppContext>,
    Query(params): Query<ListItemsQuery>,
) -> Result<Response> {
    let user = current_user(&ctx, &auth).await?;
    let (filter, pagination) = params
        .into_filter_and_pagination()
        .map_err(validation_error)?;
    let page = items::Model::list_for_user(&ctx.db, user.id, &filter, &pagination).await?;
    format::json(Page::<ItemResponse>::from_query(page))
}

#[debug_handler]
async fn create(
    auth: auth::JWT,
    State(ctx): State<AppContext>,
    Json(payload): Json<CreateItemsRequest>,
) -> Result<Response> {
    let user = current_user(&ctx, &auth).await?;
    let settings = user_settings::Model::get_or_create(&ctx.db, user.id).await?;

    let params = payload.into_params();
    if params.is_empty() {
        return Err(validation_error(
            "provide at least one item to create".to_string(),
        ));
    }
    if params.len() > 100 {
        return Err(validation_error(
            "a batch can contain at most 100 items".to_string(),
        ));
    }

    let new_items = params
        .into_iter()
        .map(|params| {
            params.into_new_item(settings.default_item_type(), settings.default_priority())
        })
        .collect::<Result<Vec<_>, _>>()
        .map_err(validation_error)?;

    let created = items::Model::create_many(&ctx.db, user.id, new_items).await?;
    let response = CreateItemsResponse {
        items: created.iter().map(ItemResponse::from).collect(),
    };

    format::render().status(StatusCode::CREATED).json(response)
}

#[debug_handler]
async fn show(
    auth: auth::JWT,
    State(ctx): State<AppContext>,
    Path(pid): Path<String>,
) -> Result<Response> {
    let user = current_user(&ctx, &auth).await?;
    let item = items::Model::find_by_pid_for_user(&ctx.db, user.id, &pid).await?;
    format::json(ItemResponse::from(item))
}

#[debug_handler]
async fn update(
    auth: auth::JWT,
    State(ctx): State<AppContext>,
    Path(pid): Path<String>,
    Json(params): Json<UpdateItemParams>,
) -> Result<Response> {
    let user = current_user(&ctx, &auth).await?;
    let patch = params.into_patch().map_err(validation_error)?;
    let item = items::Model::update_for_user(&ctx.db, user.id, &pid, patch).await?;
    format::json(ItemResponse::from(item))
}

#[debug_handler]
async fn remove(
    auth: auth::JWT,
    State(ctx): State<AppContext>,
    Path(pid): Path<String>,
) -> Result<Response> {
    let user = current_user(&ctx, &auth).await?;
    items::Model::delete_for_user(&ctx.db, user.id, &pid).await?;
    format::render().status(StatusCode::NO_CONTENT).empty()
}

pub fn routes() -> Routes {
    Routes::new()
        .add("/api/items", get(list).post(create))
        .add("/api/items/{pid}", get(show).patch(update).delete(remove))
}
