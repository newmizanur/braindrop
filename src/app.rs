use async_trait::async_trait;
use loco_rs::{
    app::{AppContext, Hooks, Initializer},
    bgworker::{BackgroundWorker, Queue},
    boot::{create_app, BootResult, StartMode},
    config::Config,
    controller::AppRoutes,
    db::{self, truncate_table},
    environment::Environment,
    task::Tasks,
    Result,
};
use migration::Migrator;
use std::path::Path;

#[allow(unused_imports)]
use crate::{
    controllers,
    models::_entities::users,
    tasks,
    workers::{downloader::DownloadWorker, notification_delivery::NotificationDeliveryWorker},
};

pub struct App;
#[async_trait]
impl Hooks for App {
    fn app_name() -> &'static str {
        env!("CARGO_CRATE_NAME")
    }

    fn app_version() -> String {
        format!(
            "{} ({})",
            env!("CARGO_PKG_VERSION"),
            option_env!("BUILD_SHA")
                .or(option_env!("GITHUB_SHA"))
                .unwrap_or("dev")
        )
    }

    async fn boot(
        mode: StartMode,
        environment: &Environment,
        config: Config,
    ) -> Result<BootResult> {
        create_app::<Self, Migrator>(mode, environment, config).await
    }

    async fn initializers(_ctx: &AppContext) -> Result<Vec<Box<dyn Initializer>>> {
        Ok(vec![Box::new(
            crate::initializers::notifications::Notifications,
        )])
    }

    fn routes(_ctx: &AppContext) -> AppRoutes {
        AppRoutes::with_default_routes() // controller routes below
            .add_route(controllers::auth::routes())
            .add_route(controllers::ai::routes())
            .add_route(controllers::items::routes())
            .add_route(controllers::notifications::routes())
            .add_route(controllers::stats::routes())
            .add_route(controllers::settings::routes())
            .add_route(controllers::api_fallback::routes())
    }
    async fn connect_workers(ctx: &AppContext, queue: &Queue) -> Result<()> {
        queue.register(DownloadWorker::build(ctx)).await?;
        queue
            .register(NotificationDeliveryWorker::build(ctx))
            .await?;
        Ok(())
    }

    #[allow(unused_variables)]
    fn register_tasks(tasks: &mut Tasks) {
        // tasks-inject (do not remove)
        tasks.register(tasks::admin_create::AdminCreate);
        tasks.register(tasks::user_create::UserCreate);
        tasks.register(tasks::user_delete::UserDelete);
        tasks.register(tasks::notifications::NotificationReminders);
        tasks.register(tasks::notifications::NotificationDigests);
        tasks.register(tasks::notifications::NotificationOverdue);
        tasks.register(tasks::notifications::NotificationRecover);
        tasks.register(tasks::notifications::NotificationCleanup);
    }
    async fn truncate(ctx: &AppContext) -> Result<()> {
        truncate_table(
            &ctx.db,
            crate::models::_entities::notification_deliveries::Entity,
        )
        .await?;
        truncate_table(&ctx.db, crate::models::_entities::items::Entity).await?;
        truncate_table(&ctx.db, crate::models::_entities::user_settings::Entity).await?;
        truncate_table(&ctx.db, users::Entity).await?;
        Ok(())
    }
    async fn seed(ctx: &AppContext, base: &Path) -> Result<()> {
        db::seed::<users::ActiveModel>(&ctx.db, &base.join("users.yaml").display().to_string())
            .await?;
        Ok(())
    }
}
