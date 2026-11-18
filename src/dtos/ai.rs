use loco_rs::prelude::DateTimeWithTimeZone;
use serde::{Deserialize, Serialize};
use ts_rs::TS;

use crate::models::items::{ItemPriority, ItemType};

/// Whether AI capture can be used right now.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize, TS)]
#[serde(rename_all = "snake_case")]
#[ts(export, export_to = "../frontend/src/bindings/")]
pub enum AiStatus {
    Available,
    Disabled,
    NotConfigured,
}

/// The body of `POST /api/ai/parse`.
#[derive(Clone, Debug, Deserialize, Serialize, TS)]
#[ts(export, export_to = "../frontend/src/bindings/")]
pub struct ParseItemsParams {
    pub text: String,
}

/// One reviewable item suggested by the model. It has the same shape as an item
/// creation payload, minus status: suggestions always start in the inbox.
#[derive(Clone, Debug, Deserialize, Serialize, TS)]
#[ts(export, export_to = "../frontend/src/bindings/")]
pub struct AiSuggestion {
    #[serde(rename = "type")]
    pub item_type: ItemType,
    pub title: String,
    pub content: Option<String>,
    pub category: Option<String>,
    pub tags: Vec<String>,
    pub priority: ItemPriority,
    #[ts(type = "string | null")]
    pub due_at: Option<DateTimeWithTimeZone>,
    #[ts(type = "string | null")]
    pub remind_at: Option<DateTimeWithTimeZone>,
}

/// The response to a successful parse. Suggestions are never persisted.
#[derive(Clone, Debug, Deserialize, Serialize, TS)]
#[ts(export, export_to = "../frontend/src/bindings/")]
pub struct AiParseResponse {
    pub items: Vec<AiSuggestion>,
}
