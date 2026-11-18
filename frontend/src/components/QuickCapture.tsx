import { useState } from 'react'
import {
  Alert,
  Badge,
  Button,
  Group,
  Loader,
  Modal,
  Select,
  Stack,
  Text,
  Textarea,
  Tooltip,
} from '@mantine/core'
import { DateTimePicker } from '@mantine/dates'
import { notifications } from '@mantine/notifications'
import {
  IconAlertCircle,
  IconArrowLeft,
  IconSparkles,
} from '@tabler/icons-react'

import type { AiSettingsResponse } from '../bindings/AiSettingsResponse'
import type { ItemPriority } from '../bindings/ItemPriority'
import type { ItemType } from '../bindings/ItemType'
import { useParseItems } from '../api/ai'
import { useCreateItems } from '../api/items'
import {
  aiErrorMessage,
  draftsFromSuggestions,
  selectedItemInputs,
  type AiSuggestionDraft,
} from '../lib/ai'
import {
  ITEM_PRIORITIES,
  ITEM_TYPES,
  itemTypeLabel,
  priorityLabel,
  toIso,
} from '../lib/items'
import { AiSuggestionReview } from './AiSuggestionReview'

interface QuickCaptureProps {
  opened: boolean
  onClose: () => void
  defaults?: { type?: ItemType; priority?: ItemPriority }
  ai?: AiSettingsResponse
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
 * The fastest path into the inbox. Manual capture is always available; AI is
 * an optional second step that returns reviewable suggestions the user can
 * edit, remove or discard before anything is saved.
 */
export function QuickCapture({
  opened,
  onClose,
  defaults,
  ai,
}: QuickCaptureProps) {
  // The shell remounts this dialog on each open, so the account defaults can be
  // read once here instead of syncing props into state with an effect.
  const defaultType = defaults?.type ?? 'task'
  const defaultPriority = defaults?.priority ?? 'normal'

  const [text, setText] = useState('')
  const [type, setType] = useState<ItemType>(defaultType)
  const [priority, setPriority] = useState<ItemPriority>(defaultPriority)
  const [dueAt, setDueAt] = useState<string | Date | null>(null)
  const [drafts, setDrafts] = useState<AiSuggestionDraft[] | null>(null)
  const [error, setError] = useState<string | null>(null)
  const [aiError, setAiError] = useState<string | null>(null)
  const createItems = useCreateItems()
  const parseItems = useParseItems()

  const aiAvailable = ai?.status === 'available'
  const aiStatusText = ai
    ? (ai.status_reason ?? 'AI is unavailable right now.')
    : 'Checking AI availability…'
  const selectedCount = drafts?.filter((draft) => draft.selected).length ?? 0

  function reset() {
    setText('')
    setType(defaultType)
    setPriority(defaultPriority)
    setDueAt(null)
    setDrafts(null)
    setError(null)
    setAiError(null)
  }

  async function handleSaveManual() {
    const trimmed = text.trim()
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

  async function handleOrganize() {
    const trimmed = text.trim()
    if (!trimmed) {
      setError('Write something to capture first.')
      return
    }

    setError(null)
    setAiError(null)
    try {
      const response = await parseItems.mutateAsync(trimmed)
      setDrafts(draftsFromSuggestions(response.items))
      if (response.items.length === 0) {
        setAiError(
          'The AI provider did not find any items. Try rephrasing, or save it manually.',
        )
      }
    } catch (cause) {
      // Stay in capture mode so the manual save button remains available.
      setAiError(aiErrorMessage(cause))
    }
  }

  async function handleSaveSelected() {
    if (!drafts) {
      return
    }
    const items = selectedItemInputs(drafts)
    if (items.length === 0) {
      setError('Select at least one suggestion with a title.')
      return
    }

    setError(null)
    try {
      await createItems.mutateAsync(items)
      notifications.show({
        title: items.length === 1 ? 'Suggestion saved' : 'Suggestions saved',
        message:
          items.length === 1
            ? 'Your item is waiting in the inbox.'
            : `${items.length} items are waiting in the inbox.`,
        color: 'teal',
      })
      handleClose()
    } catch (cause) {
      setError(
        cause instanceof Error ? cause.message : 'Could not save the items.',
      )
    }
  }

  function handleBackToText() {
    setDrafts(null)
    setAiError(null)
    setError(null)
  }

  function handleClose() {
    reset()
    onClose()
  }

  const busy = createItems.isPending || parseItems.isPending

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

        {drafts === null ? (
          <>
            <Textarea
              label="What is on your mind?"
              description="Capture it now, organise it later. AI can suggest structure, but you stay in control."
              placeholder="Buy milk tomorrow, renew my domain on December 15…"
              autosize
              minRows={4}
              maxRows={10}
              value={text}
              onChange={(event) => setText(event.currentTarget.value)}
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
                onChange={(value) =>
                  value && setPriority(value as ItemPriority)
                }
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

            {!aiAvailable && (
              <Text size="xs" c="dimmed">
                {aiStatusText} Manual capture always works.
              </Text>
            )}

            {parseItems.isPending && (
              <Group gap="xs">
                <Loader size="xs" />
                <Text size="xs" c="dimmed">
                  Organizing your capture…
                </Text>
              </Group>
            )}

            {aiError && (
              <Alert
                icon={<IconAlertCircle size={16} />}
                color="orange"
                variant="light"
                role="alert"
              >
                <Text size="sm">{aiError}</Text>
                <Group mt="xs">
                  <Button
                    size="xs"
                    variant="light"
                    onClick={handleOrganize}
                    loading={parseItems.isPending}
                  >
                    Try again
                  </Button>
                </Group>
              </Alert>
            )}

            <Group justify="space-between">
              <Button variant="subtle" color="gray" onClick={handleClose}>
                Cancel
              </Button>
              <Group gap="xs">
                <Tooltip
                  label={
                    aiAvailable
                      ? 'Turn this text into editable suggestions'
                      : `${aiStatusText} Configure AI in Integrations.`
                  }
                  withArrow
                >
                  <Button
                    variant="light"
                    leftSection={<IconSparkles size={16} />}
                    onClick={handleOrganize}
                    loading={parseItems.isPending}
                    disabled={!aiAvailable || createItems.isPending}
                  >
                    Organize with AI
                  </Button>
                </Tooltip>
                <Button
                  onClick={handleSaveManual}
                  loading={createItems.isPending}
                  disabled={parseItems.isPending}
                >
                  Save to inbox
                </Button>
              </Group>
            </Group>
          </>
        ) : (
          <>
            <Group justify="space-between" align="center" wrap="wrap">
              <div>
                <Text fw={600}>AI suggestions</Text>
                <Text size="xs" c="dimmed">
                  Review, edit or remove anything before saving.
                </Text>
              </div>
              <Badge variant="light" color="indigo">
                {selectedCount} selected
              </Badge>
            </Group>

            {aiError && (
              <Alert
                icon={<IconAlertCircle size={16} />}
                color="orange"
                variant="light"
                role="alert"
              >
                <Text size="sm">{aiError}</Text>
                <Group mt="xs">
                  <Button
                    size="xs"
                    variant="light"
                    onClick={handleOrganize}
                    loading={parseItems.isPending}
                  >
                    Try again
                  </Button>
                </Group>
              </Alert>
            )}

            <AiSuggestionReview drafts={drafts} onChange={setDrafts} />

            <Group justify="space-between">
              <Button
                variant="subtle"
                color="gray"
                leftSection={<IconArrowLeft size={16} />}
                onClick={handleBackToText}
                disabled={busy}
              >
                Back to text
              </Button>
              <Button
                onClick={handleSaveSelected}
                loading={createItems.isPending}
                disabled={selectedCount === 0 || parseItems.isPending}
              >
                Save selected
              </Button>
            </Group>
          </>
        )}
      </Stack>
    </Modal>
  )
}
