import { MantineProvider } from '@mantine/core'
import { render, screen } from '@testing-library/react'
import userEvent from '@testing-library/user-event'
import { beforeEach, describe, expect, it, vi } from 'vitest'

import type { SettingsResponse } from '../bindings/SettingsResponse'
import { Integrations } from './Integrations'

const mocks = vi.hoisted(() => ({
  update: vi.fn(),
  send: vi.fn(),
  settings: { data: undefined as unknown },
}))

vi.mock('../api/settings', () => ({
  useSettings: () => ({
    data: mocks.settings.data,
    isLoading: false,
    isError: false,
    error: null,
  }),
  useUpdateSettings: () => ({
    mutateAsync: mocks.update,
    isPending: false,
  }),
}))

vi.mock('../api/notifications', () => ({
  useSendTestNotification: () => ({
    mutateAsync: mocks.send,
    isPending: false,
  }),
}))

vi.mock('@mantine/notifications', () => ({
  notifications: { show: vi.fn() },
}))

function settings(overrides: Partial<SettingsResponse['notifications']> = {}) {
  return {
    timezone: 'UTC',
    theme: 'auto',
    default_item_type: 'task',
    default_priority: 'normal',
    week_start: 1,
    ai: {
      enabled: false,
      use_server_key: true,
      model: '',
      has_personal_key: false,
      status: 'disabled',
      status_reason: 'AI is disabled on this server.',
      active_model: null,
      credential_source: null,
      server_model: null,
    },
    notifications: {
      enabled: true,
      telegram: {
        enabled: true,
        configured: true,
        token_set: true,
        chat_id: '-1001234567890',
      },
      discord: {
        enabled: true,
        configured: true,
        webhook_set: true,
      },
      reminders_enabled: true,
      daily_digest_enabled: true,
      daily_digest_time: '09:00',
      overdue_notifications_enabled: true,
      ...overrides,
    },
  } satisfies SettingsResponse
}

function renderPage(data: SettingsResponse) {
  mocks.settings.data = data
  render(
    <MantineProvider>
      <Integrations />
    </MantineProvider>,
  )
}

describe('Integrations notifications', () => {
  beforeEach(() => {
    mocks.update.mockReset()
    mocks.send.mockReset()
    mocks.update.mockResolvedValue(settings())
  })

  it('shows independent Telegram and Discord sections', () => {
    renderPage(settings())

    expect(screen.getByRole('heading', { name: 'Telegram' })).toBeVisible()
    expect(screen.getByRole('heading', { name: 'Discord' })).toBeVisible()
    expect(screen.getAllByText('Enabled').length).toBeGreaterThanOrEqual(2)
    expect(
      screen.getByText(/Create a bot with @BotFather/),
    ).toBeInTheDocument()
    expect(screen.getByText(/Server Settings → Integrations/)).toBeInTheDocument()
  })

  it('saves a write-only Telegram token and chat id', async () => {
    const user = userEvent.setup()
    renderPage(
      settings({
        telegram: {
          enabled: false,
          configured: false,
          token_set: false,
          chat_id: '',
        },
      }),
    )

    await user.click(
      screen.getByRole('switch', { name: /Enable Telegram/ }),
    )
    await user.type(
      screen.getByLabelText('Bot token'),
      '123456789:abcdefghijklmnopqrstuvwxyz',
    )
    await user.type(screen.getByLabelText('Chat ID'), '-1001234567890')
    await user.click(screen.getByRole('button', { name: 'Save Telegram' }))

    expect(mocks.update).toHaveBeenCalledWith({
      telegram_enabled: true,
      telegram_chat_id: '-1001234567890',
      telegram_bot_token: '123456789:abcdefghijklmnopqrstuvwxyz',
    })
  })

  it('sends a test through the configured channel', async () => {
    const user = userEvent.setup()
    renderPage(settings())

    await user.click(
      screen.getAllByRole('button', { name: 'Send test message' })[0],
    )

    expect(mocks.send).toHaveBeenCalledWith('telegram')
  })

  it('removes stored credentials without ever reading them back', async () => {
    const user = userEvent.setup()
    renderPage(settings())

    await user.click(
      screen.getByRole('button', { name: 'Remove credentials' }),
    )

    expect(mocks.update).toHaveBeenCalledWith({
      telegram_bot_token: '',
      telegram_chat_id: '',
      telegram_enabled: false,
    })
  })
})
