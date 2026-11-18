//! Message rendering.
//!
//! Both providers receive plain text. Telegram is never sent a `parse_mode`,
//! so user titles cannot inject formatting; Discord mentions are disabled by
//! the client with `allowed_mentions.parse = []`. Every message is capped to the
//! provider's length limit on a character boundary.

use chrono::{DateTime, Utc};
use chrono_tz::Tz;

use crate::models::items;

/// A ready-to-send message. It contains only user data, never credentials.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct OutgoingMessage {
    pub text: String,
}

impl OutgoingMessage {
    #[must_use]
    pub fn new(text: String) -> Self {
        Self { text }
    }
}

/// Remove control characters that could break a provider payload, keeping
/// newlines and tabs.
#[must_use]
pub fn sanitize(value: &str) -> String {
    value
        .chars()
        .filter(|character| !character.is_control() || matches!(character, '\n' | '\r' | '\t'))
        .collect::<String>()
        .trim()
        .to_string()
}

/// Truncate to at most `limit` characters, appending an ellipsis when shortened.
#[must_use]
pub fn truncate(value: &str, limit: usize) -> String {
    if value.chars().count() <= limit {
        return value.to_string();
    }
    if limit == 0 {
        return String::new();
    }
    let mut truncated: String = value.chars().take(limit.saturating_sub(1)).collect();
    truncated.push('…');
    truncated
}

fn format_due(value: Option<DateTime<Utc>>, timezone: Tz) -> Option<String> {
    value.map(|value| {
        value
            .with_timezone(&timezone)
            .format("%Y-%m-%d %H:%M")
            .to_string()
    })
}

fn item_line(item: &items::Model, timezone: Tz, limit: usize) -> String {
    let title = sanitize(&item.title);
    let title = truncate(&title, limit);
    match format_due(item.due_at.map(|value| value.with_timezone(&Utc)), timezone) {
        Some(due) => format!("• {title} — due {due}"),
        None => format!("• {title}"),
    }
}

/// A reminder for one item.
#[must_use]
pub fn reminder(item: &items::Model, timezone: Tz) -> OutgoingMessage {
    let title = truncate(&sanitize(&item.title), 180);
    let mut text = format!("Reminder: {title}");
    if let Some(due) = format_due(item.due_at.map(|value| value.with_timezone(&Utc)), timezone) {
        text.push_str(&format!("\nDue: {due}"));
    }
    if let Some(content) = item
        .content
        .as_deref()
        .map(sanitize)
        .filter(|value| !value.is_empty())
    {
        text.push_str(&format!("\n{}", truncate(&content, 300)));
    }
    OutgoingMessage::new(text)
}

/// The daily digest: due today, upcoming and overdue, in the user's timezone.
#[must_use]
pub fn digest(
    due_today: &[items::Model],
    upcoming: &[items::Model],
    overdue: &[items::Model],
    timezone: Tz,
    max_items: usize,
) -> OutgoingMessage {
    let mut sections = Vec::new();
    push_section(&mut sections, "Due today", due_today, timezone, max_items);
    push_section(&mut sections, "Upcoming", upcoming, timezone, max_items);
    push_section(&mut sections, "Overdue", overdue, timezone, max_items);

    if sections.is_empty() {
        return OutgoingMessage::new("BrainDrop daily digest: nothing due.".to_string());
    }
    OutgoingMessage::new(format!(
        "BrainDrop daily digest\n\n{}",
        sections.join("\n\n")
    ))
}

/// A grouped overdue notification.
#[must_use]
pub fn overdue(items: &[items::Model], timezone: Tz, max_items: usize) -> OutgoingMessage {
    let mut text = String::from("Overdue in BrainDrop");
    for item in items.iter().take(max_items) {
        text.push('\n');
        text.push_str(&item_line(item, timezone, 140));
    }
    if items.len() > max_items {
        text.push_str(&format!("\n…and {} more", items.len() - max_items));
    }
    OutgoingMessage::new(text)
}

/// The fixed message used by `POST /api/notifications/test`.
#[must_use]
pub fn test_message(channel: &str) -> OutgoingMessage {
    OutgoingMessage::new(format!(
        "BrainDrop test notification via {channel}. If you can read this, the channel is configured correctly."
    ))
}

fn push_section(
    sections: &mut Vec<String>,
    heading: &str,
    items: &[items::Model],
    timezone: Tz,
    max_items: usize,
) {
    if items.is_empty() {
        return;
    }
    let mut section = heading.to_string();
    for item in items.iter().take(max_items) {
        section.push('\n');
        section.push_str(&item_line(item, timezone, 140));
    }
    if items.len() > max_items {
        section.push_str(&format!("\n…and {} more", items.len() - max_items));
    }
    sections.push(section);
}

#[cfg(test)]
mod tests {
    use super::*;
    use chrono::Utc;
    use uuid::Uuid;

    fn item(title: &str) -> items::Model {
        let now = Utc::now().fixed_offset();
        items::Model {
            created_at: now,
            updated_at: now,
            id: 1,
            pid: Uuid::new_v4(),
            user_id: 1,
            item_type: "task".to_string(),
            title: title.to_string(),
            content: None,
            category: None,
            tags: None,
            status: "pending".to_string(),
            priority: "normal".to_string(),
            due_at: None,
            remind_at: None,
            completed_at: None,
            archived_at: None,
        }
    }

    #[test]
    fn truncates_on_character_boundaries() {
        assert_eq!(truncate("abcdef", 10), "abcdef");
        assert_eq!(truncate("abcdef", 4), "abc…");
        assert_eq!(truncate("éééé", 2), "é…");
        assert_eq!(truncate("abc", 0), "");
    }

    #[test]
    fn sanitizes_control_characters() {
        assert_eq!(sanitize("  hello\u{0}world\n"), "helloworld");
        assert_eq!(sanitize("a\nb"), "a\nb");
    }

    #[test]
    fn digest_omits_empty_sections_and_marks_more() {
        let items: Vec<items::Model> = (0..12).map(|i| item(&format!("Task {i}"))).collect();
        let message = digest(&items, &[], &[], chrono_tz::UTC, 10);
        assert!(message.text.contains("Due today"));
        assert!(!message.text.contains("Upcoming"));
        assert!(message.text.contains("…and 2 more"));
    }

    #[test]
    fn reminder_includes_due_and_content() {
        let mut item = item("Renew domain");
        item.content = Some("before it expires".to_string());
        item.due_at = Some(Utc::now().fixed_offset());
        let message = reminder(&item, chrono_tz::UTC);
        assert!(message.text.contains("Renew domain"));
        assert!(message.text.contains("Due:"));
        assert!(message.text.contains("before it expires"));
    }
}
