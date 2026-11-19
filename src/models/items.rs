use chrono::Utc;
use loco_rs::{
    model::query::{paginate, PageResponse, PaginationQuery},
    prelude::*,
};
use serde::{Deserialize, Serialize};
use serde_json::json;
use ts_rs::TS;
use uuid::Uuid;

pub use super::_entities::items::{ActiveModel, Column, Entity, Model};
pub type Items = Entity;

/// The kinds of thing a user can capture.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize, TS)]
#[serde(rename_all = "lowercase")]
#[ts(export)]
pub enum ItemType {
    Task,
    Note,
    Idea,
    Reminder,
}

impl ItemType {
    #[must_use]
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::Task => "task",
            Self::Note => "note",
            Self::Idea => "idea",
            Self::Reminder => "reminder",
        }
    }

    #[must_use]
    pub fn parse(value: &str) -> Option<Self> {
        match value {
            "task" => Some(Self::Task),
            "note" => Some(Self::Note),
            "idea" => Some(Self::Idea),
            "reminder" => Some(Self::Reminder),
            _ => None,
        }
    }
}

/// The lifecycle state of an item.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize, TS)]
#[serde(rename_all = "lowercase")]
#[ts(export)]
pub enum ItemStatus {
    Inbox,
    Pending,
    Completed,
    Archived,
}

impl ItemStatus {
    #[must_use]
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::Inbox => "inbox",
            Self::Pending => "pending",
            Self::Completed => "completed",
            Self::Archived => "archived",
        }
    }

    #[must_use]
    pub fn parse(value: &str) -> Option<Self> {
        match value {
            "inbox" => Some(Self::Inbox),
            "pending" => Some(Self::Pending),
            "completed" => Some(Self::Completed),
            "archived" => Some(Self::Archived),
            _ => None,
        }
    }

    /// Whether an item in this state still needs attention.
    #[must_use]
    pub const fn is_open(self) -> bool {
        matches!(self, Self::Inbox | Self::Pending)
    }
}

/// How urgent an item is.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize, TS)]
#[serde(rename_all = "lowercase")]
#[ts(export)]
pub enum ItemPriority {
    Low,
    Normal,
    High,
    Urgent,
}

impl ItemPriority {
    #[must_use]
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::Low => "low",
            Self::Normal => "normal",
            Self::High => "high",
            Self::Urgent => "urgent",
        }
    }

    #[must_use]
    pub fn parse(value: &str) -> Option<Self> {
        match value {
            "low" => Some(Self::Low),
            "normal" => Some(Self::Normal),
            "high" => Some(Self::High),
            "urgent" => Some(Self::Urgent),
            _ => None,
        }
    }
}

/// The fields accepted when creating an item.
#[derive(Clone, Debug)]
pub struct NewItem {
    pub item_type: ItemType,
    pub title: String,
    pub content: Option<String>,
    pub category: Option<String>,
    pub tags: Vec<String>,
    pub status: ItemStatus,
    pub priority: ItemPriority,
    pub due_at: Option<DateTimeWithTimeZone>,
    pub remind_at: Option<DateTimeWithTimeZone>,
}

/// A partial update. `None` means "leave unchanged"; `Some(None)` clears a
/// nullable column.
#[derive(Clone, Debug, Default)]
pub struct ItemPatch {
    pub item_type: Option<ItemType>,
    pub title: Option<String>,
    pub content: Option<Option<String>>,
    pub category: Option<Option<String>>,
    pub tags: Option<Vec<String>>,
    pub status: Option<ItemStatus>,
    pub priority: Option<ItemPriority>,
    pub due_at: Option<Option<DateTimeWithTimeZone>>,
    pub remind_at: Option<Option<DateTimeWithTimeZone>>,
}

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Serialize, Deserialize, TS)]
#[serde(rename_all = "snake_case")]
#[ts(export)]
pub enum ItemSort {
    #[default]
    CreatedAt,
    UpdatedAt,
    DueAt,
    Priority,
    Title,
}

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Serialize, Deserialize, TS)]
#[serde(rename_all = "lowercase")]
#[ts(export)]
pub enum SortOrder {
    Asc,
    #[default]
    Desc,
}

impl SortOrder {
    fn as_order(self) -> Order {
        match self {
            Self::Asc => Order::Asc,
            Self::Desc => Order::Desc,
        }
    }
}

/// Filters for a scoped item list. Every field is optional; the user id is
/// supplied separately and is never part of the filter.
#[derive(Clone, Debug, Default)]
pub struct ItemFilter {
    pub search: Option<String>,
    pub item_type: Option<ItemType>,
    pub statuses: Vec<ItemStatus>,
    pub priority: Option<ItemPriority>,
    pub category: Option<String>,
    pub tag: Option<String>,
    pub due_from: Option<DateTimeWithTimeZone>,
    pub due_to: Option<DateTimeWithTimeZone>,
    pub sort: ItemSort,
    pub order: SortOrder,
}

impl ItemFilter {
    fn condition(&self, user_id: i64) -> Condition {
        let mut condition = Condition::all().add(Column::UserId.eq(user_id));

        if let Some(search) = self
            .search
            .as_deref()
            .map(str::trim)
            .filter(|s| !s.is_empty())
        {
            condition = condition.add(
                Condition::any()
                    .add(Column::Title.contains(search))
                    .add(Column::Content.contains(search))
                    .add(Column::Category.contains(search)),
            );
        }
        if let Some(item_type) = self.item_type {
            condition = condition.add(Column::ItemType.eq(item_type.as_str()));
        }
        if !self.statuses.is_empty() {
            condition = condition
                .add(Column::Status.is_in(self.statuses.iter().map(|status| status.as_str())));
        }
        if let Some(priority) = self.priority {
            condition = condition.add(Column::Priority.eq(priority.as_str()));
        }
        if let Some(category) = self.category.as_deref().filter(|c| !c.is_empty()) {
            condition = condition.add(Column::Category.eq(category));
        }
        if let Some(tag) = self.tag.as_deref().filter(|t| !t.is_empty()) {
            // Tags are stored as a JSON array of strings. Matching the quoted
            // token keeps "work" from matching "homework".
            condition = condition.add(Column::Tags.contains(format!("\"{tag}\"")));
        }
        if let Some(due_from) = self.due_from {
            condition = condition.add(Column::DueAt.gte(due_from));
        }
        if let Some(due_to) = self.due_to {
            condition = condition.add(Column::DueAt.lt(due_to));
        }

        condition
    }
}

/// Normalize an incoming timestamp to UTC so storage and range comparisons do
/// not depend on the offset a client chose.
#[must_use]
pub fn to_utc(value: DateTimeWithTimeZone) -> DateTimeWithTimeZone {
    value.with_timezone(&Utc).fixed_offset()
}

/// Normalize user-supplied tags without discarding input. DTO validation applies
/// the shared count and length limits after normalization.
#[must_use]
pub fn normalize_tags(tags: Vec<String>) -> Vec<String> {
    let mut normalized: Vec<String> = tags
        .into_iter()
        .map(|tag| tag.trim().to_string())
        .filter(|tag| !tag.is_empty())
        .collect();
    normalized.sort();
    normalized.dedup();
    normalized
}

pub fn validate_tags(tags: &[String]) -> Result<(), String> {
    if tags.len() > 50 {
        return Err("an item can have at most 50 tags".to_string());
    }
    if tags.iter().any(|tag| tag.chars().count() > 50) {
        return Err("each tag must be at most 50 characters".to_string());
    }
    Ok(())
}

/// Read the tag list back out of the JSON column.
#[must_use]
pub fn tags_from_json(tags: Option<&serde_json::Value>) -> Vec<String> {
    tags.and_then(serde_json::Value::as_array)
        .map(|values| {
            values
                .iter()
                .filter_map(serde_json::Value::as_str)
                .map(ToString::to_string)
                .collect()
        })
        .unwrap_or_default()
}

#[async_trait::async_trait]
impl ActiveModelBehavior for ActiveModel {
    async fn before_save<C>(self, _db: &C, insert: bool) -> std::result::Result<Self, DbErr>
    where
        C: ConnectionTrait,
    {
        if !insert && self.updated_at.is_unchanged() {
            let mut this = self;
            this.updated_at = sea_orm::ActiveValue::Set(chrono::Utc::now().into());
            Ok(this)
        } else {
            Ok(self)
        }
    }
}

impl Model {
    /// Find one item by public id, scoped to its owner.
    ///
    /// An id that is malformed or belongs to another user is reported as not
    /// found, so the API never confirms the existence of someone else's item.
    ///
    /// # Errors
    ///
    /// Returns `EntityNotFound` for unknown ids and database errors for failed
    /// queries.
    pub async fn find_by_pid_for_user(
        db: &DatabaseConnection,
        user_id: i64,
        pid: &str,
    ) -> ModelResult<Self> {
        let Ok(pid) = Uuid::parse_str(pid) else {
            return Err(ModelError::EntityNotFound);
        };
        Entity::find()
            .filter(
                Condition::all()
                    .add(Column::UserId.eq(user_id))
                    .add(Column::Pid.eq(pid)),
            )
            .one(db)
            .await?
            .ok_or(ModelError::EntityNotFound)
    }

    /// List the user's items with filtering, sorting and pagination.
    ///
    /// # Errors
    ///
    /// Returns an error when the database query fails.
    pub async fn list_for_user(
        db: &DatabaseConnection,
        user_id: i64,
        filter: &ItemFilter,
        pagination: &PaginationQuery,
    ) -> Result<PageResponse<Self>> {
        let mut query = Entity::find();
        // Keep undated items at the end only when the list is ordered by due
        // date; other sorts should not be biased by the presence of a date.
        if matches!(filter.sort, ItemSort::DueAt) {
            query = query.order_by(Column::DueAt.is_null(), Order::Asc);
        }

        query = match filter.sort {
            ItemSort::CreatedAt => query.order_by(Column::CreatedAt, filter.order.as_order()),
            ItemSort::UpdatedAt => query.order_by(Column::UpdatedAt, filter.order.as_order()),
            ItemSort::DueAt => query.order_by(Column::DueAt, filter.order.as_order()),
            ItemSort::Title => query.order_by(Column::Title, filter.order.as_order()),
            ItemSort::Priority => query.order_by(
                sea_orm::sea_query::Expr::cust(
                    "CASE priority WHEN 'urgent' THEN 0 WHEN 'high' THEN 1 \
                     WHEN 'normal' THEN 2 ELSE 3 END",
                ),
                filter.order.as_order(),
            ),
        };
        // A stable tie-breaker keeps pages deterministic when the primary sort
        // column repeats.
        query = query.order_by(Column::Id, Order::Desc);

        paginate(db, query, Some(filter.condition(user_id)), pagination).await
    }

    /// Insert one or more items for a user in a single transaction.
    ///
    /// If any insert fails the whole batch is rolled back, so a caller never
    /// observes a partially applied request.
    ///
    /// # Errors
    ///
    /// Returns a database error when the transaction or an insert fails.
    pub async fn create_many(
        db: &DatabaseConnection,
        user_id: i64,
        items: Vec<NewItem>,
    ) -> ModelResult<Vec<Self>> {
        let txn = db.begin().await?;
        let now = Utc::now();
        let mut created = Vec::with_capacity(items.len());

        for item in items {
            let (completed_at, archived_at) = match item.status {
                ItemStatus::Completed => (Some(now), None),
                ItemStatus::Archived => (None, Some(now)),
                ItemStatus::Inbox | ItemStatus::Pending => (None, None),
            };
            let active = ActiveModel {
                pid: ActiveValue::set(Uuid::new_v4()),
                user_id: ActiveValue::set(user_id),
                item_type: ActiveValue::set(item.item_type.as_str().to_string()),
                title: ActiveValue::set(item.title),
                content: ActiveValue::set(item.content),
                category: ActiveValue::set(item.category),
                tags: ActiveValue::set(Some(json!(normalize_tags(item.tags)))),
                status: ActiveValue::set(item.status.as_str().to_string()),
                priority: ActiveValue::set(item.priority.as_str().to_string()),
                due_at: ActiveValue::set(item.due_at.map(to_utc)),
                remind_at: ActiveValue::set(item.remind_at.map(to_utc)),
                completed_at: ActiveValue::set(completed_at.map(Into::into)),
                archived_at: ActiveValue::set(archived_at.map(Into::into)),
                ..Default::default()
            };
            created.push(active.insert(&txn).await?);
        }

        txn.commit().await?;
        Ok(created)
    }

    /// Apply a partial update and return the refreshed model.
    ///
    /// State transitions keep the derived timestamps consistent: completing
    /// stamps `completed_at`, reopening clears it, and archiving stamps
    /// `archived_at` without destroying the completion history.
    ///
    /// # Errors
    ///
    /// Returns `EntityNotFound` for unknown ids and a database error when the
    /// update fails.
    pub async fn update_for_user(
        db: &DatabaseConnection,
        user_id: i64,
        pid: &str,
        patch: ItemPatch,
    ) -> ModelResult<Self> {
        let model = Self::find_by_pid_for_user(db, user_id, pid).await?;
        let mut active = model.into_active_model();
        let now = Utc::now();

        if let Some(item_type) = patch.item_type {
            active.item_type = ActiveValue::set(item_type.as_str().to_string());
        }
        if let Some(title) = patch.title {
            active.title = ActiveValue::set(title);
        }
        if let Some(content) = patch.content {
            active.content = ActiveValue::set(content);
        }
        if let Some(category) = patch.category {
            active.category = ActiveValue::set(category);
        }
        if let Some(tags) = patch.tags {
            active.tags = ActiveValue::set(Some(json!(normalize_tags(tags))));
        }
        if let Some(priority) = patch.priority {
            active.priority = ActiveValue::set(priority.as_str().to_string());
        }
        if let Some(due_at) = patch.due_at {
            active.due_at = ActiveValue::set(due_at.map(to_utc));
        }
        if let Some(remind_at) = patch.remind_at {
            active.remind_at = ActiveValue::set(remind_at.map(to_utc));
        }
        if let Some(status) = patch.status {
            active.status = ActiveValue::set(status.as_str().to_string());
            match status {
                ItemStatus::Completed => {
                    active.completed_at = ActiveValue::set(Some(now.into()));
                    active.archived_at = ActiveValue::set(None);
                }
                ItemStatus::Archived => {
                    active.archived_at = ActiveValue::set(Some(now.into()));
                }
                ItemStatus::Inbox | ItemStatus::Pending => {
                    active.completed_at = ActiveValue::set(None);
                    active.archived_at = ActiveValue::set(None);
                }
            }
        }

        active.update(db).await.map_err(ModelError::from)
    }

    /// Complete an item, stamping the completion time.
    ///
    /// # Errors
    ///
    /// Returns `EntityNotFound` for unknown ids and a database error when the
    /// update fails.
    pub async fn complete_for_user(
        db: &DatabaseConnection,
        user_id: i64,
        pid: &str,
    ) -> ModelResult<Self> {
        Self::update_for_user(
            db,
            user_id,
            pid,
            ItemPatch {
                status: Some(ItemStatus::Completed),
                ..Default::default()
            },
        )
        .await
    }

    /// Move an item back to the inbox and clear completion metadata.
    ///
    /// # Errors
    ///
    /// Returns `EntityNotFound` for unknown ids and a database error when the
    /// update fails.
    pub async fn reopen_for_user(
        db: &DatabaseConnection,
        user_id: i64,
        pid: &str,
    ) -> ModelResult<Self> {
        Self::update_for_user(
            db,
            user_id,
            pid,
            ItemPatch {
                status: Some(ItemStatus::Inbox),
                ..Default::default()
            },
        )
        .await
    }

    /// Delete an item owned by the user.
    ///
    /// # Errors
    ///
    /// Returns `EntityNotFound` for unknown ids and a database error when the
    /// delete fails.
    pub async fn delete_for_user(
        db: &DatabaseConnection,
        user_id: i64,
        pid: &str,
    ) -> ModelResult<()> {
        let model = Self::find_by_pid_for_user(db, user_id, pid).await?;
        model.delete(db).await?;
        Ok(())
    }
}
