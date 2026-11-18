import { useState } from 'react'
import {
  Alert,
  Badge,
  Button,
  Divider,
  Group,
  Paper,
  PasswordInput,
  SimpleGrid,
  Stack,
  Switch,
  Text,
  TextInput,
  ThemeIcon,
  Title,
} from '@mantine/core'
import { notifications } from '@mantine/notifications'
import {
  IconAlertCircle,
  IconBrandDiscord,
  IconBrandTelegram,
  IconInfoCircle,
  IconSparkles,
} from '@tabler/icons-react'

import type { AiSettingsResponse } from '../bindings/AiSettingsResponse'
import type { AiStatus } from '../bindings/AiStatus'
import type { UpdateSettingsParams } from '../bindings/UpdateSettingsParams'
import { useSettings, useUpdateSettings } from '../api/settings'
import { PageHeader } from '../components/PageHeader'

const statusLabel: Record<AiStatus, string> = {
  available: 'Available',
  disabled: 'Disabled',
  not_configured: 'Not configured',
}

const statusColor: Record<AiStatus, string> = {
  available: 'teal',
  disabled: 'gray',
  not_configured: 'yellow',
}

interface OpenRouterCardProps {
  ai: AiSettingsResponse
}

function OpenRouterCard({ ai }: OpenRouterCardProps) {
  const updateSettings = useUpdateSettings()
  const [enabled, setEnabled] = useState(ai.enabled)
  const [useServerKey, setUseServerKey] = useState(ai.use_server_key)
  const [model, setModel] = useState(ai.model)
  const [apiKey, setApiKey] = useState('')
  const [error, setError] = useState<string | null>(null)

  async function save(
    payload: UpdateSettingsParams,
    message: string,
  ): Promise<boolean> {
    setError(null)
    try {
      const saved = await updateSettings.mutateAsync(payload)
      setEnabled(saved.ai.enabled)
      setUseServerKey(saved.ai.use_server_key)
      setModel(saved.ai.model)
      setApiKey('')
      notifications.show({
        title: 'AI settings saved',
        message,
        color: 'teal',
      })
      return true
    } catch (cause) {
      setError(
        cause instanceof Error
          ? cause.message
          : 'Could not save the AI settings.',
      )
      return false
    }
  }

  async function handleSave(event: React.FormEvent<HTMLFormElement>) {
    event.preventDefault()
    const payload: UpdateSettingsParams = {
      ai_enabled: enabled,
      ai_use_server_key: useServerKey,
      ai_model: model.trim(),
    }
    if (apiKey.trim()) {
      payload.ai_api_key = apiKey.trim()
    }
    await save(payload, 'Your AI preferences were updated.')
  }

  async function handleRemoveKey() {
    await save({ ai_api_key: '' }, 'The stored personal key was removed.')
  }

  return (
    <Paper withBorder p="lg" radius="lg">
      <form onSubmit={handleSave}>
        <Stack gap="md">
          <Group justify="space-between" align="flex-start" wrap="nowrap">
            <Group align="flex-start" wrap="nowrap">
              <ThemeIcon size={44} radius="md" variant="light" color="indigo">
                <IconSparkles size={24} />
              </ThemeIcon>
              <div>
                <Title order={3} size="h4">
                  OpenRouter
                </Title>
                <Text size="sm" c="dimmed">
                  Optional AI parsing that turns unstructured capture into
                  editable item suggestions. Manual capture always works.
                </Text>
              </div>
            </Group>
            <Badge variant="light" color={statusColor[ai.status]}>
              {statusLabel[ai.status]}
            </Badge>
          </Group>

          {ai.status === 'available' ? (
            <Text size="sm">
              Using{' '}
              <Text span fw={600}>
                {ai.credential_source === 'personal'
                  ? 'your personal key'
                  : 'the server key'}
              </Text>{' '}
              with model <code>{ai.active_model}</code>.
            </Text>
          ) : (
            <Alert
              icon={<IconInfoCircle size={16} />}
              color={ai.status === 'disabled' ? 'gray' : 'yellow'}
              variant="light"
            >
              <Text size="sm">
                {ai.status_reason ??
                  'AI is unavailable. You can keep capturing manually.'}
              </Text>
            </Alert>
          )}

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

          <Switch
            label="Enable AI for this account"
            description="The server must also enable AI. Turning this off leaves manual capture untouched."
            checked={enabled}
            onChange={(event) => setEnabled(event.currentTarget.checked)}
          />

          <Switch
            label="Use the server's OpenRouter key"
            description="Turn this off to use your own key. A personal key is only used when it is stored."
            checked={useServerKey}
            onChange={(event) => setUseServerKey(event.currentTarget.checked)}
          />

          <TextInput
            label="Model"
            description="Leave blank to use the server default. The value is passed to OpenRouter as-is."
            placeholder={ai.server_model ?? 'e.g. openai/gpt-4o-mini'}
            value={model}
            onChange={(event) => setModel(event.currentTarget.value)}
            maxLength={200}
          />

          <Divider />

          <div>
            <PasswordInput
              label="Personal OpenRouter key"
              description="Stored encrypted on the server. It is never returned by the API or kept in browser storage."
              placeholder={
                ai.has_personal_key
                  ? 'A key is stored — enter a new one to replace it'
                  : 'sk-or-v1-…'
              }
              value={apiKey}
              onChange={(event) => setApiKey(event.currentTarget.value)}
              autoComplete="off"
              spellCheck={false}
            />
            <Text size="xs" c="dimmed" mt={6}>
              {ai.has_personal_key
                ? 'A personal key is stored for this account.'
                : 'No personal key is stored for this account.'}
            </Text>
          </div>

          <Group justify="space-between">
            <Group>
              {ai.has_personal_key && (
                <Button
                  type="button"
                  variant="subtle"
                  color="red"
                  onClick={handleRemoveKey}
                  loading={updateSettings.isPending}
                >
                  Remove stored key
                </Button>
              )}
            </Group>
            <Button type="submit" loading={updateSettings.isPending}>
              Save AI settings
            </Button>
          </Group>
        </Stack>
      </form>
    </Paper>
  )
}

const plannedIntegrations = [
  {
    name: 'Telegram',
    icon: IconBrandTelegram,
    color: 'cyan',
    description:
      'Receive scheduled reminders and daily digests through a Telegram bot. Credentials are encrypted at rest.',
    milestone: 'Milestone 5',
  },
  {
    name: 'Discord',
    icon: IconBrandDiscord,
    color: 'violet',
    description:
      'Deliver reminders and digests through a Discord webhook. No bot gateway connection is required.',
    milestone: 'Milestone 5',
  },
]

export function Integrations() {
  const settings = useSettings()

  return (
    <>
      <PageHeader
        title="Integrations"
        description="Optional services. BrainDrop is fully usable without any of them."
        action={<Badge variant="light">Opt-in</Badge>}
      />

      <Stack gap="lg" maw={860}>
        {settings.isLoading && <Text c="dimmed">Loading AI settings…</Text>}
        {settings.isError && (
          <Alert color="red" variant="light">
            Could not load your integrations: {settings.error.message}
          </Alert>
        )}
        {settings.data && (
          <OpenRouterCard
            key={JSON.stringify(settings.data.ai)}
            ai={settings.data.ai}
          />
        )}

        <SimpleGrid cols={{ base: 1, md: 2 }} spacing="md">
          {plannedIntegrations.map((integration) => (
            <Paper key={integration.name} withBorder p="lg" radius="lg">
              <Stack gap="sm" h="100%">
                <Group justify="space-between">
                  <ThemeIcon
                    size={44}
                    radius="md"
                    variant="light"
                    color={integration.color}
                  >
                    <integration.icon size={24} />
                  </ThemeIcon>
                  <Badge variant="light" color={integration.color}>
                    Planned
                  </Badge>
                </Group>
                <Title order={3} size="h4">
                  {integration.name}
                </Title>
                <Text size="sm" c="dimmed" style={{ flexGrow: 1 }}>
                  {integration.description}
                </Text>
                <Button variant="default" disabled>
                  Configure in {integration.milestone}
                </Button>
              </Stack>
            </Paper>
          ))}
        </SimpleGrid>

        <Text size="xs" c="dimmed">
          Integrations are opt-in. Provider keys are encrypted at rest and are
          never included in API responses or browser storage.
        </Text>
      </Stack>
    </>
  )
}
