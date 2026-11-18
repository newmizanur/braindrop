//! Strict validation of the model's JSON.
//!
//! The provider is untrusted: every field is checked, unknown fields are
//! rejected, enums must be known values and dates must be real. Nothing here
//! touches the database, so a malformed response can never create an item.

use chrono::{DateTime, Datelike, NaiveDate, NaiveDateTime, TimeZone, Utc};
use chrono_tz::Tz;
use loco_rs::prelude::DateTimeWithTimeZone;
use serde::Deserialize;

use crate::{
    ai::error::AiError,
    dtos::ai::AiSuggestion,
    models::items::{normalize_tags, ItemPriority, ItemType},
    time,
};

const MAX_TITLE_CHARS: usize = 500;
const MAX_CONTENT_CHARS: usize = 10_000;
const MAX_CATEGORY_CHARS: usize = 100;
const MAX_TAGS: usize = 20;
const MAX_TAG_CHARS: usize = 50;

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
struct RawEnvelope {
    items: Vec<RawSuggestion>,
}

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
struct RawSuggestion {
    #[serde(rename = "type")]
    item_type: ItemType,
    title: String,
    #[serde(default)]
    content: Option<String>,
    #[serde(default)]
    category: Option<String>,
    #[serde(default)]
    tags: Vec<String>,
    priority: ItemPriority,
    #[serde(default)]
    due_at: Option<String>,
    #[serde(default)]
    remind_at: Option<String>,
}

/// Parse and validate the model's content.
///
/// # Errors
///
/// Returns `InvalidResponse` for malformed JSON, unknown fields, bad enums,
/// empty titles, invalid dates or over-long text, and `TooLarge` when the model
/// returns more items than the configured limit.
pub fn parse(content: &str, timezone: Tz, max_items: usize) -> Result<Vec<AiSuggestion>, AiError> {
    let content = strip_code_fence(content);
    let envelope: RawEnvelope = serde_json::from_str(content).map_err(|_| {
        AiError::InvalidResponse("the JSON did not match the required schema".to_string())
    })?;

    if envelope.items.len() > max_items {
        return Err(AiError::TooLarge(format!(
            "the model returned more than {max_items} items"
        )));
    }

    envelope
        .items
        .into_iter()
        .map(|raw| suggestion(raw, timezone))
        .collect()
}

fn suggestion(raw: RawSuggestion, timezone: Tz) -> Result<AiSuggestion, AiError> {
    let title = raw.title.trim().to_string();
    if title.is_empty() {
        return Err(AiError::InvalidResponse(
            "an item had an empty title".to_string(),
        ));
    }
    if title.chars().count() > MAX_TITLE_CHARS {
        return Err(AiError::InvalidResponse(
            "an item title was too long".to_string(),
        ));
    }

    let content = clean_optional(raw.content, MAX_CONTENT_CHARS, "content")?;
    let category = clean_optional(raw.category, MAX_CATEGORY_CHARS, "category")?;

    let mut tags = normalize_tags(raw.tags);
    tags.retain(|tag| tag.chars().count() <= MAX_TAG_CHARS);
    tags.truncate(MAX_TAGS);

    Ok(AiSuggestion {
        item_type: raw.item_type,
        title,
        content,
        category,
        tags,
        priority: raw.priority,
        due_at: parse_datetime(raw.due_at.as_deref(), timezone)?,
        remind_at: parse_datetime(raw.remind_at.as_deref(), timezone)?,
    })
}

fn clean_optional(
    value: Option<String>,
    max: usize,
    field: &str,
) -> Result<Option<String>, AiError> {
    let Some(value) = value else {
        return Ok(None);
    };
    let value = value.trim().to_string();
    if value.is_empty() {
        return Ok(None);
    }
    if value.chars().count() > max {
        return Err(AiError::InvalidResponse(format!(
            "an item {field} was too long"
        )));
    }
    Ok(Some(value))
}

fn parse_datetime(
    value: Option<&str>,
    timezone: Tz,
) -> Result<Option<DateTimeWithTimeZone>, AiError> {
    let Some(value) = value.map(str::trim).filter(|value| !value.is_empty()) else {
        return Ok(None);
    };

    let parsed = DateTime::parse_from_rfc3339(value)
        .map(|datetime| datetime.with_timezone(&Utc))
        .or_else(|_| {
            NaiveDateTime::parse_from_str(value, "%Y-%m-%dT%H:%M:%S")
                .map_err(|_| ())
                .and_then(|naive| {
                    timezone
                        .from_local_datetime(&naive)
                        .earliest()
                        .ok_or(())
                        .map(|local| local.with_timezone(&Utc))
                })
        })
        .or_else(|()| {
            NaiveDate::parse_from_str(value, "%Y-%m-%d")
                .map_err(|_| ())
                .map(|date| time::start_of_day(timezone, date))
        })
        .map_err(|()| AiError::InvalidResponse("an item had an invalid date".to_string()))?;

    if !(1900..=2200).contains(&parsed.year()) {
        return Err(AiError::InvalidResponse(
            "an item date was outside the supported range".to_string(),
        ));
    }
    Ok(Some(parsed.fixed_offset()))
}

/// Models sometimes wrap JSON in a markdown fence even when asked not to.
fn strip_code_fence(content: &str) -> &str {
    let trimmed = content.trim();
    let Some(rest) = trimmed
        .strip_prefix("```json")
        .or_else(|| trimmed.strip_prefix("```JSON"))
        .or_else(|| trimmed.strip_prefix("```"))
    else {
        return trimmed;
    };
    rest.trim_end()
        .strip_suffix("```")
        .map_or(trimmed, str::trim)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn timezone() -> Tz {
        "Asia/Tokyo".parse().unwrap()
    }

    #[test]
    fn parses_multiple_suggestions_and_normalizes_fields() {
        let content = r#"{
            "items": [
                {
                    "type": "task",
                    "title": "  Buy milk  ",
                    "content": "  semi-skimmed  ",
                    "category": " errands ",
                    "tags": ["home", " home ", ""],
                    "priority": "high",
                    "due_at": "2026-12-15T09:00:00+09:00",
                    "remind_at": null
                },
                {
                    "type": "note",
                    "title": "Learn Rust async",
                    "content": null,
                    "category": null,
                    "tags": [],
                    "priority": "normal",
                    "due_at": null,
                    "remind_at": null
                }
            ]
        }"#;

        let items = parse(content, timezone(), 20).unwrap();
        assert_eq!(items.len(), 2);
        assert_eq!(items[0].title, "Buy milk");
        assert_eq!(items[0].content.as_deref(), Some("semi-skimmed"));
        assert_eq!(items[0].category.as_deref(), Some("errands"));
        assert_eq!(items[0].tags, vec!["home"]);
        assert_eq!(
            items[0].due_at.unwrap().to_rfc3339(),
            "2026-12-15T00:00:00+00:00"
        );
        assert!(items[0].remind_at.is_none());
        assert_eq!(items[1].item_type, ItemType::Note);
        assert!(items[1].due_at.is_none());
    }

    #[test]
    fn resolves_date_only_values_in_the_user_timezone() {
        let content = r#"{"items":[{"type":"task","title":"Renew domain","content":null,"category":null,"tags":[],"priority":"normal","due_at":"2026-12-15","remind_at":null}]}"#;
        let items = parse(content, timezone(), 20).unwrap();
        assert_eq!(
            items[0].due_at.unwrap().to_rfc3339(),
            "2026-12-14T15:00:00+00:00"
        );
    }

    #[test]
    fn rejects_unknown_fields_and_bad_enums() {
        let unknown = r#"{"items":[{"type":"task","title":"x","content":null,"category":null,"tags":[],"priority":"normal","due_at":null,"remind_at":null,"status":"inbox"}]}"#;
        assert!(parse(unknown, timezone(), 20).is_err());

        let bad_type = r#"{"items":[{"type":"meeting","title":"x","content":null,"category":null,"tags":[],"priority":"normal","due_at":null,"remind_at":null}]}"#;
        assert!(parse(bad_type, timezone(), 20).is_err());

        let bad_priority = r#"{"items":[{"type":"task","title":"x","content":null,"category":null,"tags":[],"priority":"soon","due_at":null,"remind_at":null}]}"#;
        assert!(parse(bad_priority, timezone(), 20).is_err());
    }

    #[test]
    fn rejects_empty_titles_and_invalid_dates() {
        let empty = r#"{"items":[{"type":"task","title":"   ","content":null,"category":null,"tags":[],"priority":"normal","due_at":null,"remind_at":null}]}"#;
        assert!(parse(empty, timezone(), 20).is_err());

        let bad_date = r#"{"items":[{"type":"task","title":"x","content":null,"category":null,"tags":[],"priority":"normal","due_at":"tomorrow","remind_at":null}]}"#;
        assert!(parse(bad_date, timezone(), 20).is_err());
    }

    #[test]
    fn enforces_the_item_limit() {
        let items = (0..3)
            .map(|index| {
                format!(
                    r#"{{"type":"task","title":"item {index}","content":null,"category":null,"tags":[],"priority":"normal","due_at":null,"remind_at":null}}"#
                )
            })
            .collect::<Vec<_>>()
            .join(",");
        let content = format!(r#"{{"items":[{items}]}}"#);
        assert!(matches!(
            parse(&content, timezone(), 2),
            Err(AiError::TooLarge(_))
        ));
    }

    #[test]
    fn strips_markdown_fences() {
        let content = "```json\n{\"items\":[]}\n```";
        assert!(parse(content, timezone(), 20).unwrap().is_empty());
    }

    #[test]
    fn bounds_tags_and_drops_over_long_ones() {
        let tags = (0..30)
            .map(|index| format!("\"tag{index}\""))
            .chain(std::iter::once(format!("\"{}\"", "x".repeat(60))))
            .collect::<Vec<_>>()
            .join(",");
        let content = format!(
            r#"{{"items":[{{"type":"task","title":"x","content":null,"category":null,"tags":[{tags}],"priority":"normal","due_at":null,"remind_at":null}}]}}"#
        );
        let items = parse(&content, timezone(), 20).unwrap();
        assert_eq!(items[0].tags.len(), 20);
        assert!(items[0].tags.iter().all(|tag| tag.len() <= 50));
    }

    #[test]
    fn missing_deadlines_stay_missing() {
        let content = r#"{"items":[{"type":"idea","title":"Learn Rust","content":null,"category":null,"tags":[],"priority":"low","due_at":null,"remind_at":null}]}"#;
        let items = parse(content, timezone(), 20).unwrap();
        assert!(items[0].due_at.is_none());
        assert!(items[0].remind_at.is_none());
    }
}
