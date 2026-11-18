import { useState } from 'react'
import {
  Alert,
  Button,
  Group,
  Modal,
  Select,
  Stack,
  Textarea,
  TextInput,
} from '@mantine/core'
import { DateTimePicker } from '@mantine/dates'
import { IconAlertCircle } from '@tabler/icons-react'

import type { ItemPriority } from '../bindings/ItemPriority'
import type { ItemResponse } from '../bindings/ItemResponse'
import type { ItemStatus } from '../bindings/ItemStatus'
import type { ItemType } from '../bindings/ItemType'
import type { ItemInput } from '../api/items'
import {
  ITEM_PRIORITIES,
  ITEM_STATUSES,
  ITEM_TYPES,
  formatTags,
  itemTypeLabel,
  parseTags,
  priorityLabel,
  statusLabel,
  toIso,
} from '../lib/items'

interface ItemEditorProps {
  opened: boolean
  onClose: () => void
  /** The item being edited, or `null` to create a new one. */
  item: ItemResponse | null
  defaults?: { type?: ItemType; priority?: ItemPriority }
  onSubmit: (input: ItemInput, pid?: string) => Promise<void>
}

type PickerValue = string | Date | null

const typeOptions = ITEM_TYPES.map((value) => ({
  value,
  label: itemTypeLabel(value),
}))
const priorityOptions = ITEM_PRIORITIES.map((value) => ({
  value,
  label: priorityLabel(value),
}))
const statusOptions = ITEM_STATUSES.map((value) => ({
  value,
  label: statusLabel(value),
}))

export function ItemEditor({
  opened,
  onClose,
  item,
  defaults,
  onSubmit,
}: ItemEditorProps) {
  // The parent remounts this component when the dialog opens (via `key`), so
  // initial state is enough — no effect is needed to copy props into state.
  const [title, setTitle] = useState(item?.title ?? '')
  const [type, setType] = useState<ItemType>(item?.type ?? defaults?.type ?? 'task')
  const [priority, setPriority] = useState<ItemPriority>(
    item?.priority ?? defaults?.priority ?? 'normal',
  )
  const [status, setStatus] = useState<ItemStatus>(item?.status ?? 'inbox')
  const [content, setContent] = useState(item?.content ?? '')
  const [category, setCategory] = useState(item?.category ?? '')
  const [tags, setTags] = useState(item ? formatTags(item.tags) : '')
  const [dueAt, setDueAt] = useState<PickerValue>(item?.due_at ?? null)
  const [remindAt, setRemindAt] = useState<PickerValue>(item?.remind_at ?? null)
  const [error, setError] = useState<string | null>(null)
  const [submitting, setSubmitting] = useState(false)

  async function handleSubmit(event: React.FormEvent<HTMLFormElement>) {
    event.preventDefault()
    const trimmed = title.trim()
    if (!trimmed) {
      setError('A title is required.')
      return
    }

    setSubmitting(true)
    setError(null)
    try {
      const input: ItemInput = {
        title: trimmed,
        type,
        priority,
        content: content.trim() ? content.trim() : null,
        category: category.trim() ? category.trim() : null,
        tags: parseTags(tags),
        due_at: toIso(dueAt),
        remind_at: toIso(remindAt),
      }
      if (item) {
        await onSubmit({ ...input, status }, item.pid)
      } else {
        await onSubmit(input)
      }
      onClose()
    } catch (cause) {
      setError(
        cause instanceof Error ? cause.message : 'Could not save the item.',
      )
    } finally {
      setSubmitting(false)
    }
  }

  return (
    <Modal
      opened={opened}
      onClose={onClose}
      title={item ? 'Edit item' : 'New item'}
      size="lg"
      centered
    >
      <form onSubmit={handleSubmit}>
        <Stack gap="md">
          {error && (
            <Alert
              icon={<IconAlertCircle size={16} />}
              color="red"
              variant="light"
              role="alert"
            >
              {error}
            </Alert>
          )}

          <TextInput
            label="Title"
            placeholder="What needs doing?"
            value={title}
            onChange={(event) => setTitle(event.currentTarget.value)}
            required
            data-autofocus
          />

          <Group grow align="flex-start">
            <Select
              label="Type"
              data={typeOptions}
              value={type}
              onChange={(value) => value && setType(value as ItemType)}
              allowDeselect={false}
            />
            <Select
              label="Priority"
              data={priorityOptions}
              value={priority}
              onChange={(value) => value && setPriority(value as ItemPriority)}
              allowDeselect={false}
            />
          </Group>

          {item && (
            <Select
              label="Status"
              data={statusOptions}
              value={status}
              onChange={(value) => value && setStatus(value as ItemStatus)}
              allowDeselect={false}
            />
          )}

          <Textarea
            label="Notes"
            placeholder="Optional details"
            autosize
            minRows={3}
            maxRows={8}
            value={content}
            onChange={(event) => setContent(event.currentTarget.value)}
          />

          <Group grow align="flex-start">
            <TextInput
              label="Category"
              placeholder="work, home…"
              value={category}
              onChange={(event) => setCategory(event.currentTarget.value)}
            />
            <TextInput
              label="Tags"
              description="Comma separated"
              placeholder="deep, writing"
              value={tags}
              onChange={(event) => setTags(event.currentTarget.value)}
            />
          </Group>

          <Group grow align="flex-start">
            <DateTimePicker
              label="Due"
              placeholder="Pick a date and time"
              value={dueAt}
              onChange={setDueAt}
              clearable
              valueFormat="DD MMM YYYY HH:mm"
            />
            <DateTimePicker
              label="Remind"
              placeholder="Optional reminder"
              value={remindAt}
              onChange={setRemindAt}
              clearable
              valueFormat="DD MMM YYYY HH:mm"
            />
          </Group>

          <Group justify="flex-end" mt="xs">
            <Button variant="default" onClick={onClose} type="button">
              Cancel
            </Button>
            <Button type="submit" loading={submitting}>
              {item ? 'Save changes' : 'Create item'}
            </Button>
          </Group>
        </Stack>
      </form>
    </Modal>
  )
}
