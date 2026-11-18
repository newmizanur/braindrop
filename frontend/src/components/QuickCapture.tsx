import { useState } from 'react'
import {
  Alert,
  Button,
  Group,
  Modal,
  Select,
  Stack,
  Textarea,
} from '@mantine/core'
import { DateTimePicker } from '@mantine/dates'
import { notifications } from '@mantine/notifications'
import { IconAlertCircle } from '@tabler/icons-react'

import type { ItemPriority } from '../bindings/ItemPriority'
import type { ItemType } from '../bindings/ItemType'
import { useCreateItems } from '../api/items'
import {
  ITEM_PRIORITIES,
  ITEM_TYPES,
  itemTypeLabel,
  priorityLabel,
  toIso,
} from '../lib/items'

interface QuickCaptureProps {
  opened: boolean
  onClose: () => void
  defaults?: { type?: ItemType; priority?: ItemPriority }
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
 * The fastest path into the inbox: one field, an optional type and due date,
 * and a single button. Manual capture always works without an AI provider.
 */
export function QuickCapture({
  opened,
  onClose,
  defaults,
}: QuickCaptureProps) {
  // The shell remounts this dialog on each open, so the account defaults can be
  // read once here instead of syncing props into state with an effect.
  const defaultType = defaults?.type ?? 'task'
  const defaultPriority = defaults?.priority ?? 'normal'

  const [title, setTitle] = useState('')
  const [type, setType] = useState<ItemType>(defaultType)
  const [priority, setPriority] = useState<ItemPriority>(defaultPriority)
  const [dueAt, setDueAt] = useState<string | Date | null>(null)
  const [error, setError] = useState<string | null>(null)
  const createItems = useCreateItems()

  function reset() {
    setTitle('')
    setType(defaultType)
    setPriority(defaultPriority)
    setDueAt(null)
    setError(null)
  }

  async function handleSave() {
    const trimmed = title.trim()
    if (!trimmed) {
      setError('Write something to capture first.')
      return
    }

    setError(null)
    try {
      await createItems.mutateAsync([
        {
          title: trimmed,
          type,
          priority,
          due_at: toIso(dueAt),
        },
      ])
      notifications.show({
        title: 'Captured',
        message: 'Your item is waiting in the inbox.',
        color: 'teal',
      })
      handleClose()
    } catch (cause) {
      setError(
        cause instanceof Error ? cause.message : 'Could not save the item.',
      )
    }
  }

  function handleClose() {
    reset()
    onClose()
  }

  return (
    <Modal
      opened={opened}
      onClose={handleClose}
      title="Quick capture"
      size="lg"
      centered
    >
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

        <Textarea
          label="What is on your mind?"
          description="Capture it now, organise it later."
          placeholder="Buy milk tomorrow, renew my domain on December 15…"
          autosize
          minRows={4}
          maxRows={10}
          value={title}
          onChange={(event) => setTitle(event.currentTarget.value)}
          classNames={{ input: 'prewrap' }}
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

        <DateTimePicker
          label="Due"
          placeholder="Optional due date"
          value={dueAt}
          onChange={setDueAt}
          clearable
          valueFormat="DD MMM YYYY HH:mm"
        />

        <Group justify="space-between">
          <Button
            variant="subtle"
            color="gray"
            onClick={handleClose}
          >
            Cancel
          </Button>
          <Button onClick={handleSave} loading={createItems.isPending}>
            Save to inbox
          </Button>
        </Group>
      </Stack>
    </Modal>
  )
}
