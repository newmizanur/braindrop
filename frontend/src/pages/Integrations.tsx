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
  IconSend,
  IconSparkles,
  IconTrash,
} from '@tabler/icons-react'

import type { AiSettingsResponse } from '../bindings/AiSettingsResponse'
import type { AiStatus } from '../bindings/AiStatus'
import type { DiscordSettingsResponse } from '../bindings/DiscordSettingsResponse'
import type { TelegramSettingsResponse } from '../bindings/TelegramSettingsResponse'
import type { UpdateSettingsParams } from '../bindings/UpdateSettingsParams'
import { useSendTestNotification } from '../api/notifications'
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

function channelStatus(enabled: boolean, configured: boolean) {
  if (!enabled) {
    return { label: 'Disabled', color: 'gray' }
  }
  if (!configured) {
    return { label: 'Needs setup', color: 'yellow' }
  }
  return { label: 'Enabled', color: 'teal' }
}

interface TelegramCardProps {
  telegram: TelegramSettingsResponse
}

function TelegramCard({ telegram }: TelegramCardProps) {
  const updateSettings = useUpdateSettings()
  const sendTest = useSendTestNotification()
  const [enabled, setEnabled] = useState(telegram.enabled)
  const [chatId, setChatId] = useState(telegram.chat_id)
  const [token, setToken] = useState('')
  const [tokenSet, setTokenSet] = useState(telegram.token_set)
  const [error, setError] = useState<string | null>(null)
  const [testError, setTestError] = useState<string | null>(null)

  const configured = tokenSet && chatId.trim().length > 0
  const status = channelStatus(enabled, configured)

  async function handleSave(event: React.FormEvent<HTMLFormElement>) {
    event.preventDefault()
    setError(null)
    const payload: UpdateSettingsParams = {
      telegram_enabled: enabled,
      telegram_chat_id: chatId.trim(),
    }
    if (token.trim()) {
      payload.telegram_bot_token = token.trim()
    }
    try {
      const saved = await updateSettings.mutateAsync(payload)
      setEnabled(saved.notifications.telegram.enabled)
      setChatId(saved.notifications.telegram.chat_id)
      setTokenSet(saved.notifications.telegram.token_set)
      setToken('')
      notifications.show({
        title: 'Telegram saved',
        message: 'Reminders and digests will use this chat when enabled.',
        color: 'teal',
      })
    } catch (cause) {
      setError(
        cause instanceof Error
          ? cause.message
          : 'Could not save the Telegram settings.',
      )
    }
  }

  async function handleRemove() {
    setError(null)
    try {
      const saved = await updateSettings.mutateAsync({
        telegram_bot_token: '',
        telegram_chat_id: '',
        telegram_enabled: false,
      })
      setEnabled(saved.notifications.telegram.enabled)
      setChatId(saved.notifications.telegram.chat_id)
      setTokenSet(saved.notifications.telegram.token_set)
      setToken('')
      notifications.show({
        title: 'Telegram removed',
        message: 'The bot token and chat id were deleted.',
        color: 'teal',
      })
    } catch (cause) {
      setError(
        cause instanceof Error
          ? cause.message
          : 'Could not remove the Telegram credentials.',
      )
    }
  }

  async function handleTest() {
    setTestError(null)
    try {
      await sendTest.mutateAsync('telegram')
      notifications.show({
        title: 'Test sent',
        message: 'Check the Telegram chat for the test message.',
        color: 'teal',
      })
    } catch (cause) {
      setTestError(
        cause instanceof Error
          ? cause.message
          : 'Could not send the Telegram test message.',
      )
    }
  }

  return (
    <Paper withBorder p="lg" radius="lg">
      <form onSubmit={handleSave}>
        <Stack gap="md">
          <Group justify="space-between" align="flex-start" wrap="nowrap">
            <Group align="flex-start" wrap="nowrap">
              <ThemeIcon size={44} radius="md" variant="light" color="cyan">
                <IconBrandTelegram size={24} />
              </ThemeIcon>
              <div>
                <Title order={3} size="h4">
                  Telegram
                </Title>
                <Text size="sm" c="dimmed">
                  Send reminders, daily digests and overdue summaries through a
                  bot you own.
                </Text>
              </div>
            </Group>
            <Badge variant="light" color={status.color}>
              {status.label}
            </Badge>
          </Group>

          <Text size="xs" c="dimmed">
            Create a bot with @BotFather, copy its token, then start a chat with
            the bot and use that chat id. Group chats use a negative id or
            @channel username.
          </Text>

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
          {testError && (
            <Alert
              icon={<IconAlertCircle size={16} />}
              color="red"
              variant="light"
              role="alert"
            >
              {testError}
            </Alert>
          )}

          <Switch
            label="Enable Telegram"
            description="Notifications also need to be enabled in Settings."
            checked={enabled}
            onChange={(event) => setEnabled(event.currentTarget.checked)}
          />

          <PasswordInput
            label="Bot token"
            description="Write-only. It is encrypted at rest and never returned by the API."
            placeholder={
              tokenSet
                ? 'A token is stored — enter a new one to replace it'
                : '123456789:ABC…'
            }
            value={token}
            onChange={(event) => setToken(event.currentTarget.value)}
            autoComplete="off"
            spellCheck={false}
          />

          <TextInput
            label="Chat ID"
            description="The destination chat. This value is safe to display and edit."
            placeholder="-1001234567890 or @channel"
            value={chatId}
            onChange={(event) => setChatId(event.currentTarget.value)}
            maxLength={64}
          />

          <Group justify="space-between">
            <Group>
              {(tokenSet || chatId.length > 0) && (
                <Button
                  type="button"
                  variant="subtle"
                  color="red"
                  leftSection={<IconTrash size={16} />}
                  onClick={handleRemove}
                  loading={updateSettings.isPending}
                >
                  Remove credentials
                </Button>
              )}
              <Button
                type="button"
                variant="default"
                leftSection={<IconSend size={16} />}
                onClick={handleTest}
                loading={sendTest.isPending}
                disabled={!enabled || !configured}
              >
                Send test message
              </Button>
            </Group>
            <Button type="submit" loading={updateSettings.isPending}>
              Save Telegram
            </Button>
          </Group>
        </Stack>
      </form>
    </Paper>
  )
}

interface DiscordCardProps {
  discord: DiscordSettingsResponse
}

function DiscordCard({ discord }: DiscordCardProps) {
  const updateSettings = useUpdateSettings()
  const sendTest = useSendTestNotification()
  const [enabled, setEnabled] = useState(discord.enabled)
  const [webhook, setWebhook] = useState('')
  const [webhookSet, setWebhookSet] = useState(discord.webhook_set)
  const [error, setError] = useState<string | null>(null)
  const [testError, setTestError] = useState<string | null>(null)

  const configured = webhookSet
  const status = channelStatus(enabled, configured)

  async function handleSave(event: React.FormEvent<HTMLFormElement>) {
    event.preventDefault()
    setError(null)
    const payload: UpdateSettingsParams = { discord_enabled: enabled }
    if (webhook.trim()) {
      payload.discord_webhook = webhook.trim()
    }
    try {
      const saved = await updateSettings.mutateAsync(payload)
      setEnabled(saved.notifications.discord.enabled)
      setWebhookSet(saved.notifications.discord.webhook_set)
      setWebhook('')
      notifications.show({
        title: 'Discord saved',
        message: 'Reminders and digests will use this webhook when enabled.',
        color: 'teal',
      })
    } catch (cause) {
      setError(
        cause instanceof Error
          ? cause.message
          : 'Could not save the Discord settings.',
      )
    }
  }

  async function handleRemove() {
    setError(null)
    try {
      const saved = await updateSettings.mutateAsync({
        discord_webhook: '',
        discord_enabled: false,
      })
      setEnabled(saved.notifications.discord.enabled)
      setWebhookSet(saved.notifications.discord.webhook_set)
      setWebhook('')
      notifications.show({
        title: 'Discord removed',
        message: 'The webhook was deleted.',
        color: 'teal',
      })
    } catch (cause) {
      setError(
        cause instanceof Error
          ? cause.message
          : 'Could not remove the Discord webhook.',
      )
    }
  }

  async function handleTest() {
    setTestError(null)
    try {
      await sendTest.mutateAsync('discord')
      notifications.show({
        title: 'Test sent',
        message: 'Check the Discord channel for the test message.',
        color: 'teal',
      })
    } catch (cause) {
      setTestError(
        cause instanceof Error
          ? cause.message
          : 'Could not send the Discord test message.',
      )
    }
  }

  return (
    <Paper withBorder p="lg" radius="lg">
      <form onSubmit={handleSave}>
        <Stack gap="md">
          <Group justify="space-between" align="flex-start" wrap="nowrap">
            <Group align="flex-start" wrap="nowrap">
              <ThemeIcon size={44} radius="md" variant="light" color="violet">
                <IconBrandDiscord size={24} />
              </ThemeIcon>
              <div>
                <Title order={3} size="h4">
                  Discord
                </Title>
                <Text size="sm" c="dimmed">
                  Deliver reminders and digests through a channel webhook. No bot
                  or gateway connection is required.
                </Text>
              </div>
            </Group>
            <Badge variant="light" color={status.color}>
              {status.label}
            </Badge>
          </Group>

          <Text size="xs" c="dimmed">
            In Discord, open Server Settings → Integrations → Webhooks, create a
            webhook for the target channel, and paste its URL here. Mentions are
            always disabled.
          </Text>

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
          {testError && (
            <Alert
              icon={<IconAlertCircle size={16} />}
              color="red"
              variant="light"
              role="alert"
            >
              {testError}
            </Alert>
          )}

          <Switch
            label="Enable Discord"
            description="Notifications also need to be enabled in Settings."
            checked={enabled}
            onChange={(event) => setEnabled(event.currentTarget.checked)}
          />

          <PasswordInput
            label="Webhook URL"
            description="Write-only. It is encrypted at rest and never returned by the API. Only official Discord webhook URLs are accepted."
            placeholder={
              webhookSet
                ? 'A webhook is stored — enter a new one to replace it'
                : 'https://discord.com/api/webhooks/…'
            }
            value={webhook}
            onChange={(event) => setWebhook(event.currentTarget.value)}
            autoComplete="off"
            spellCheck={false}
          />

          <Group justify="space-between">
            <Group>
              {webhookSet && (
                <Button
                  type="button"
                  variant="subtle"
                  color="red"
                  leftSection={<IconTrash size={16} />}
                  onClick={handleRemove}
                  loading={updateSettings.isPending}
                >
                  Remove webhook
                </Button>
              )}
              <Button
                type="button"
                variant="default"
                leftSection={<IconSend size={16} />}
                onClick={handleTest}
                loading={sendTest.isPending}
                disabled={!enabled || !configured}
              >
                Send test message
              </Button>
            </Group>
            <Button type="submit" loading={updateSettings.isPending}>
              Save Discord
            </Button>
          </Group>
        </Stack>
      </form>
    </Paper>
  )
}

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
        {settings.isLoading && <Text c="dimmed">Loading integrations…</Text>}
        {settings.isError && (
          <Alert color="red" variant="light">
            Could not load your integrations: {settings.error.message}
          </Alert>
        )}
        {settings.data && (
          <>
            <OpenRouterCard
              key={JSON.stringify(settings.data.ai)}
              ai={settings.data.ai}
            />

            <SimpleGrid cols={{ base: 1, md: 2 }} spacing="md">
              <TelegramCard
                key={JSON.stringify(settings.data.notifications.telegram)}
                telegram={settings.data.notifications.telegram}
              />
              <DiscordCard
                key={JSON.stringify(settings.data.notifications.discord)}
                discord={settings.data.notifications.discord}
              />
            </SimpleGrid>
          </>
        )}

        <Text size="xs" c="dimmed">
          Integrations are opt-in. Provider keys and webhook URLs are encrypted
          at rest and are never included in API responses or browser storage.
        </Text>
      </Stack>
    </>
  )
}
