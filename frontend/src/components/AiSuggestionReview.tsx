import {
  ActionIcon,
  Badge,
  Checkbox,
  Group,
  Paper,
  Select,
  Stack,
  Text,
  Textarea,
  TextInput,
  Tooltip,
} from '@mantine/core'
import { DateTimePicker } from '@mantine/dates'
import { IconTrash } from '@tabler/icons-react'

import type { ItemPriority } from '../bindings/ItemPriority'
import type { ItemType } from '../bindings/ItemType'
import type { AiSuggestionDraft } from '../lib/ai'
import {
  ITEM_PRIORITIES,
  ITEM_TYPES,
  itemTypeLabel,
  priorityLabel,
} from '../lib/items'

interface AiSuggestionReviewProps {
  drafts: AiSuggestionDraft[]
  onChange: (drafts: AiSuggestionDraft[]) => void
}

const typeOptions = ITEM_TYPES.map((value) => ({
  value,
  label: itemTypeLabel(value),
}))
const priorityOptions = ITEM_PRIORITIES.map((value) => ({
  value,
  label: priorityLabel(value),
}))

/**
 * Editable review cards for AI suggestions. Every field the model returned can
 * be corrected before saving, and each suggestion can be excluded or removed.
 */
export function AiSuggestionReview({
  drafts,
  onChange,
}: AiSuggestionReviewProps) {
  if (drafts.length === 0) {
    return (
      <Text size="sm" c="dimmed">
        No suggestions are left. Go back to the text or try again.
      </Text>
    )
  }

  function update(key: string, patch: Partial<AiSuggestionDraft>) {
    onChange(
      drafts.map((draft) =>
        draft.key === key ? { ...draft, ...patch } : draft,
      ),
    )
  }

  function remove(key: string) {
    onChange(drafts.filter((draft) => draft.key !== key))
  }

  return (
    <Stack gap="sm">
      {drafts.map((draft, index) => {
        const number = index + 1
        return (
          <Paper
            key={draft.key}
            withBorder
            p="md"
            radius="md"
            opacity={draft.selected ? 1 : 0.6}
            data-testid="ai-suggestion"
          >
            <Stack gap="sm">
              <Group justify="space-between" align="center" wrap="nowrap">
                <Group gap="xs" wrap="nowrap">
                  <Checkbox
                    checked={draft.selected}
                    aria-label={`Include suggestion ${number}`}
                    onChange={(event) =>
                      update(draft.key, {
                        selected: event.currentTarget.checked,
                      })
                    }
                  />
                  <Badge variant="light" color="indigo">
                    Suggestion {number}
                  </Badge>
                </Group>
                <Tooltip label="Remove suggestion" withArrow>
                  <ActionIcon
                    variant="subtle"
                    color="red"
                    aria-label={`Remove suggestion ${number}`}
                    onClick={() => remove(draft.key)}
                  >
                    <IconTrash size={16} />
                  </ActionIcon>
                </Tooltip>
              </Group>

              <TextInput
                label="Title"
                aria-label={`Suggestion ${number} title`}
                value={draft.title}
                onChange={(event) =>
                  update(draft.key, { title: event.currentTarget.value })
                }
                required
              />

              <Group grow align="flex-start">
                <Select
                  label="Type"
                  aria-label={`Suggestion ${number} type`}
                  data={typeOptions}
                  value={draft.type}
                  onChange={(value) =>
                    value && update(draft.key, { type: value as ItemType })
                  }
                  allowDeselect={false}
                />
                <Select
                  label="Priority"
                  aria-label={`Suggestion ${number} priority`}
                  data={priorityOptions}
                  value={draft.priority}
                  onChange={(value) =>
                    value &&
                    update(draft.key, { priority: value as ItemPriority })
                  }
                  allowDeselect={false}
                />
              </Group>

              <Group grow align="flex-start">
                <DateTimePicker
                  label="Due"
                  aria-label={`Suggestion ${number} due`}
                  placeholder="No due date"
                  value={draft.dueAt}
                  onChange={(value) => update(draft.key, { dueAt: value })}
                  clearable
                  valueFormat="DD MMM YYYY HH:mm"
                />
                <DateTimePicker
                  label="Reminder"
                  aria-label={`Suggestion ${number} reminder`}
                  placeholder="No reminder"
                  value={draft.remindAt}
                  onChange={(value) => update(draft.key, { remindAt: value })}
                  clearable
                  valueFormat="DD MMM YYYY HH:mm"
                />
              </Group>

              <Group grow align="flex-start">
                <TextInput
                  label="Category"
                  aria-label={`Suggestion ${number} category`}
                  placeholder="Optional"
                  value={draft.category}
                  onChange={(event) =>
                    update(draft.key, { category: event.currentTarget.value })
                  }
                />
                <TextInput
                  label="Tags"
                  aria-label={`Suggestion ${number} tags`}
                  description="Comma-separated"
                  placeholder="Optional"
                  value={draft.tags}
                  onChange={(event) =>
                    update(draft.key, { tags: event.currentTarget.value })
                  }
                />
              </Group>

              <Textarea
                label="Notes"
                aria-label={`Suggestion ${number} notes`}
                placeholder="Optional details"
                autosize
                minRows={2}
                maxRows={5}
                value={draft.content}
                onChange={(event) =>
                  update(draft.key, { content: event.currentTarget.value })
                }
              />
            </Stack>
          </Paper>
        )
      })}
    </Stack>
  )
}
