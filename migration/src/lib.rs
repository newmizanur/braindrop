#![allow(elided_lifetimes_in_paths)]
#![allow(clippy::wildcard_imports)]
pub use sea_orm_migration::prelude::*;
mod m20220101_000001_users;

mod m20261118_145040_items;
mod m20261118_145151_user_settings;
mod m20261118_161319_add_ai_settings;
mod m20261119_000001_add_notification_settings;
mod m20261119_000002_notification_deliveries;
pub struct Migrator;

#[async_trait::async_trait]
impl MigratorTrait for Migrator {
    fn migrations() -> Vec<Box<dyn MigrationTrait>> {
        vec![
            Box::new(m20220101_000001_users::Migration),
            Box::new(m20261118_145040_items::Migration),
            Box::new(m20261118_145151_user_settings::Migration),
            Box::new(m20261118_161319_add_ai_settings::Migration),
            Box::new(m20261119_000001_add_notification_settings::Migration),
            Box::new(m20261119_000002_notification_deliveries::Migration),
            // inject-above (do not remove this comment)
        ]
    }
}
