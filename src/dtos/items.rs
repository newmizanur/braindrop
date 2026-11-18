use loco_rs::model::query::PaginationQuery;
use loco_rs::prelude::DateTimeWithTimeZone;
use serde::{Deserialize, Deserializer, Serialize};
use ts_rs::TS;

use crate::models::items::{
    normalize_tags, tags_from_json, ItemFilter, ItemPatch, ItemPriority, ItemSort, ItemStatus,
    ItemType, Model, NewItem, SortOrder,
};

/// The public JSON representation of an item. The database's internal id is
/// never exposed; `pid` is the stable public identifier.
#[derive(Clone, Debug, Serialize, Deserialize, TS)]
#[ts(export, export_to = "../frontend/src/bindings/")]
pub struct ItemResponse {
    pub pid: String,
    #[serde(rename = "type")]
    pub item_type: ItemType,
    pub title: String,
    pub content: Option<String>,
    pub category: Option<String>,
    pub tags: Vec<String>,
    pub status: ItemStatus,
    pub priority: ItemPriority,
    #[ts(type = "string | null")]
    pub due_at: Option<DateTimeWithTimeZone>,
    #[ts(type = "string | null")]
    pub remind_at: Option<DateTimeWithTimeZone>,
    #[ts(type = "string | null")]
    pub completed_at: Option<DateTimeWithTimeZone>,
    #[ts(type = "string | null")]
    pub archived_at: Option<DateTimeWithTimeZone>,
    #[ts(type = "string")]
    pub created_at: DateTimeWithTimeZone,
    #[ts(type = "string")]
    pub updated_at: DateTimeWithTimeZone,
}

impl From<&Model> for ItemResponse {
    fn from(item: &Model) -> Self {
        Self {
            pid: item.pid.to_string(),
            item_type: ItemType::parse(&item.item_type).unwrap_or(ItemType::Task),
            title: item.title.clone(),
            content: item.content.clone(),
            category: item.category.clone(),
            tags: tags_from_json(item.tags.as_ref()),
            status: ItemStatus::parse(&item.status).unwrap_or(ItemStatus::Inbox),
            priority: ItemPriority::parse(&item.priority).unwrap_or(ItemPriority::Normal),
            due_at: item.due_at,
            remind_at: item.remind_at,
            completed_at: item.completed_at,
            archived_at: item.archived_at,
            created_at: item.created_at,
            updated_at: item.updated_at,
        }
    }
}

impl From<Model> for ItemResponse {
    fn from(item: Model) -> Self {
        Self::from(&item)
    }
}

/// One item in a create request.
#[derive(Clone, Debug, Deserialize, Serialize, TS)]
#[ts(export, export_to = "../frontend/src/bindings/")]
pub struct CreateItemParams {
    #[serde(rename = "type", default)]
    pub item_type: Option<ItemType>,
    pub title: String,
    #[serde(default)]
    pub content: Option<String>,
    #[serde(default)]
    pub category: Option<String>,
    #[serde(default)]
    pub tags: Vec<String>,
    #[serde(default)]
    pub status: Option<ItemStatus>,
    #[serde(default)]
    pub priority: Option<ItemPriority>,
    #[serde(default)]
    #[ts(type = "string | null")]
    pub due_at: Option<DateTimeWithTimeZone>,
    #[serde(default)]
    #[ts(type = "string | null")]
    pub remind_at: Option<DateTimeWithTimeZone>,
}

impl CreateItemParams {
    /// Validate and convert to the model's input type.
    ///
    /// # Errors
    ///
    /// Returns a human-readable message for invalid input.
    pub fn into_new_item(
        self,
        default_type: ItemType,
        default_priority: ItemPriority,
    ) -> Result<NewItem, String> {
        let title = self.title.trim().to_string();
        if title.is_empty() {
            return Err("title must not be empty".to_string());
        }
        if title.chars().count() > 500 {
            return Err("title must be at most 500 characters".to_string());
        }
        let content = normalize_optional_text(self.content, 50_000, "content")?;
        let category = normalize_optional_text(self.category, 100, "category")?;
        let tags = normalize_tags(self.tags);
        if tags.iter().any(|tag| tag.chars().count() > 50) {
            return Err("each tag must be at most 50 characters".to_string());
        }
        if tags.len() > 50 {
            return Err("an item can have at most 50 tags".to_string());
        }

        Ok(NewItem {
            item_type: self.item_type.unwrap_or(default_type),
            title,
            content,
            category,
            tags,
            status: self.status.unwrap_or(ItemStatus::Inbox),
            priority: self.priority.unwrap_or(default_priority),
            due_at: self.due_at,
            remind_at: self.remind_at,
        })
    }
}

/// `POST /api/items` accepts either a single item or `{ "items": [ ... ] }`.
#[derive(Clone, Debug, Deserialize)]
#[serde(untagged)]
pub enum CreateItemsRequest {
    Batch { items: Vec<CreateItemParams> },
    Single(CreateItemParams),
}

impl CreateItemsRequest {
    /// Normalize the envelope into a list of items.
    #[must_use]
    pub fn into_params(self) -> Vec<CreateItemParams> {
        match self {
            Self::Batch { items } => items,
            Self::Single(item) => vec![item],
        }
    }
}

/// The response to a create request, for one item or many.
#[derive(Clone, Debug, Serialize, Deserialize, TS)]
#[ts(export, export_to = "../frontend/src/bindings/")]
pub struct CreateItemsResponse {
    pub items: Vec<ItemResponse>,
}

/// A present value in a PATCH body. `Patch::default()` (absent) means "leave
/// unchanged"; `Patch(Some(None))` means "clear this nullable field".
#[derive(Clone, Debug)]
pub struct Patch<T>(pub Option<T>);

impl<T> Default for Patch<T> {
    fn default() -> Self {
        Self(None)
    }
}

impl<'de, T: Deserialize<'de>> Deserialize<'de> for Patch<T> {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        T::deserialize(deserializer).map(|value| Self(Some(value)))
    }
}

/// A partial item update. Absent fields are left unchanged; nullable fields
/// can be cleared with an explicit `null`.
#[derive(Clone, Debug, Default, Deserialize, TS)]
#[ts(export, export_to = "../frontend/src/bindings/")]
pub struct UpdateItemParams {
    #[serde(rename = "type", default)]
    #[ts(optional, type = "string")]
    pub item_type: Patch<ItemType>,
    #[serde(default)]
    #[ts(optional, type = "string")]
    pub title: Patch<String>,
    #[serde(default)]
    #[ts(optional, type = "string | null")]
    pub content: Patch<Option<String>>,
    #[serde(default)]
    #[ts(optional, type = "string | null")]
    pub category: Patch<Option<String>>,
    #[serde(default)]
    #[ts(optional, type = "string[] | null")]
    pub tags: Patch<Option<Vec<String>>>,
    #[serde(default)]
    #[ts(optional, type = "string")]
    pub status: Patch<ItemStatus>,
    #[serde(default)]
    #[ts(optional, type = "string")]
    pub priority: Patch<ItemPriority>,
    #[serde(default)]
    #[ts(optional, type = "string | null")]
    pub due_at: Patch<Option<DateTimeWithTimeZone>>,
    #[serde(default)]
    #[ts(optional, type = "string | null")]
    pub remind_at: Patch<Option<DateTimeWithTimeZone>>,
}

impl UpdateItemParams {
    /// Validate and convert to the model's patch type.
    ///
    /// # Errors
    ///
    /// Returns a human-readable message for invalid input.
    pub fn into_patch(self) -> Result<ItemPatch, String> {
        let title = match self.title.0 {
            Some(title) => {
                let title = title.trim().to_string();
                if title.is_empty() {
                    return Err("title must not be empty".to_string());
                }
                if title.chars().count() > 500 {
                    return Err("title must be at most 500 characters".to_string());
                }
                Some(title)
            }
            None => None,
        };

        let content = match self.content.0 {
            Some(content) => Some(normalize_optional_text(content, 50_000, "content")?),
            None => None,
        };
        let category = match self.category.0 {
            Some(category) => Some(normalize_optional_text(category, 100, "category")?),
            None => None,
        };
        let tags = match self.tags.0 {
            Some(tags) => {
                let tags = normalize_tags(tags.unwrap_or_default());
                if tags.iter().any(|tag| tag.chars().count() > 50) {
                    return Err("each tag must be at most 50 characters".to_string());
                }
                Some(tags)
            }
            None => None,
        };

        Ok(ItemPatch {
            item_type: self.item_type.0,
            title,
            content,
            category,
            tags,
            status: self.status.0,
            priority: self.priority.0,
            due_at: self.due_at.0,
            remind_at: self.remind_at.0,
        })
    }
}

/// Query parameters for `GET /api/items`.
#[derive(Clone, Debug, Default, Deserialize)]
pub struct ListItemsQuery {
    #[serde(default)]
    pub page: Option<u64>,
    #[serde(default)]
    pub page_size: Option<u64>,
    #[serde(default)]
    pub q: Option<String>,
    #[serde(rename = "type", default)]
    pub item_type: Option<ItemType>,
    /// A comma-separated list, e.g. `inbox,pending`.
    #[serde(default)]
    pub status: Option<String>,
    #[serde(default)]
    pub priority: Option<ItemPriority>,
    #[serde(default)]
    pub category: Option<String>,
    #[serde(default)]
    pub tag: Option<String>,
    #[serde(default)]
    pub due_from: Option<DateTimeWithTimeZone>,
    #[serde(default)]
    pub due_to: Option<DateTimeWithTimeZone>,
    #[serde(default)]
    pub sort: Option<ItemSort>,
    #[serde(default)]
    pub order: Option<SortOrder>,
}

impl ListItemsQuery {
    /// Split the filter from the pagination options and validate values that
    /// serde cannot check on its own.
    ///
    /// # Errors
    ///
    /// Returns a message naming the invalid status value.
    pub fn into_filter_and_pagination(self) -> Result<(ItemFilter, PaginationQuery), String> {
        let statuses = match self
            .status
            .as_deref()
            .map(str::trim)
            .filter(|s| !s.is_empty())
        {
            Some(raw) => raw
                .split(',')
                .map(str::trim)
                .filter(|s| !s.is_empty())
                .map(|s| ItemStatus::parse(s).ok_or_else(|| format!("unknown item status: {s}")))
                .collect::<Result<Vec<_>, _>>()?,
            None => Vec::new(),
        };

        let filter = ItemFilter {
            search: clean(self.q),
            item_type: self.item_type,
            statuses,
            priority: self.priority,
            category: clean(self.category),
            tag: clean(self.tag),
            due_from: self.due_from.map(crate::models::items::to_utc),
            due_to: self.due_to.map(crate::models::items::to_utc),
            sort: self.sort.unwrap_or_default(),
            order: self.order.unwrap_or_default(),
        };
        let pagination = PaginationQuery {
            page: self.page.unwrap_or(1).max(1),
            page_size: self.page_size.unwrap_or(25).clamp(1, 100),
        };
        Ok((filter, pagination))
    }
}

fn clean(value: Option<String>) -> Option<String> {
    value
        .map(|value| value.trim().to_string())
        .filter(|value| !value.is_empty())
}

fn normalize_optional_text(
    value: Option<String>,
    max: usize,
    field: &str,
) -> Result<Option<String>, String> {
    match value {
        Some(value) => {
            let value = value.trim().to_string();
            if value.is_empty() {
                return Ok(None);
            }
            if value.chars().count() > max {
                return Err(format!("{field} must be at most {max} characters"));
            }
            Ok(Some(value))
        }
        None => Ok(None),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn absent_and_null_are_different_in_a_patch() {
        let absent: UpdateItemParams = serde_json::from_str(r#"{"title":"Renamed"}"#).unwrap();
        assert_eq!(absent.title.0.as_deref(), Some("Renamed"));
        assert_eq!(absent.content.0, None);

        let cleared: UpdateItemParams = serde_json::from_str(r#"{"content":null}"#).unwrap();
        assert_eq!(cleared.content.0, Some(None));

        let set: UpdateItemParams = serde_json::from_str(r#"{"content":"hello"}"#).unwrap();
        assert_eq!(set.content.0, Some(Some("hello".to_string())));
    }

    #[test]
    fn statuses_can_be_a_comma_separated_list() {
        let query: ListItemsQuery = serde_json::from_str(r#"{"status":"inbox,pending"}"#).unwrap();
        let (filter, _) = query.into_filter_and_pagination().unwrap();
        assert_eq!(
            filter.statuses,
            vec![ItemStatus::Inbox, ItemStatus::Pending]
        );
    }
}
