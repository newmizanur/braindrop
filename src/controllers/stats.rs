use loco_rs::prelude::*;

use crate::models::{stats, user_settings, users};

async fn current_user(ctx: &AppContext, auth: &auth::JWT) -> Result<users::Model> {
    users::Model::find_by_pid(&ctx.db, &auth.claims.pid)
        .await
        .map_err(Error::from)
}

#[debug_handler]
async fn dashboard(auth: auth::JWT, State(ctx): State<AppContext>) -> Result<Response> {
    let user = current_user(&ctx, &auth).await?;
    let settings = user_settings::Model::get_or_create(&ctx.db, user.id).await?;
    let stats = stats::compute(&ctx.db, user.id, &settings).await?;
    format::json(stats)
}

pub fn routes() -> Routes {
    Routes::new().add("/api/stats", get(dashboard))
}
