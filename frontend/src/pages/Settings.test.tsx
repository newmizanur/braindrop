import { MantineProvider } from '@mantine/core'
import { render, screen } from '@testing-library/react'
import userEvent from '@testing-library/user-event'
import { beforeEach, describe, expect, it, vi } from 'vitest'

import type { SettingsResponse } from '../bindings/SettingsResponse'
import { Settings } from './Settings'

const mocks = vi.hoisted(() => ({
  update: vi.fn(),
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

vi.mock('../api/auth', () => ({
  useCurrentUser: () => ({
    data: { name: 'Test User', email: 'test@example.com' },
    isLoading: false,
  }),
}))

vi.mock('@mantine/notifications', () => ({
  notifications: { show: vi.fn() },
}))

function settings(
  overrides: Partial<SettingsResponse['notifications']> = {},
): SettingsResponse {
  return {
    timezone: 'Europe/Berlin',
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
        enabled: false,
        configured: false,
        token_set: false,
        chat_id: '',
      },
      discord: {
        enabled: false,
        configured: false,
        webhook_set: false,
      },
      reminders_enabled: true,
      daily_digest_enabled: true,
      daily_digest_time: '09:00',
      overdue_notifications_enabled: false,
      ...overrides,
    },
  }
}

function renderPage(data: SettingsResponse) {
  mocks.settings.data = data
  render(
    <MantineProvider>
      <Settings />
    </MantineProvider>,
  )
}

describe('Settings notifications', () => {
  beforeEach(() => {
    mocks.update.mockReset()
    mocks.update.mockResolvedValue(settings())
  })

  it('shows the account timezone and independent notification switches', () => {
    renderPage(settings())

    expect(screen.getByRole('heading', { name: 'Notifications' })).toBeVisible()
    expect(screen.getAllByText('Europe/Berlin').length).toBeGreaterThan(0)
    expect(
      screen.getByRole('switch', { name: /Enable notifications/ }),
    ).toBeChecked()
    expect(
      screen.getByRole('switch', { name: /Scheduled reminders/ }),
    ).toBeChecked()
    expect(
      screen.getByRole('switch', { name: /Daily digest/ }),
    ).toBeChecked()
    expect(
      screen.getByRole('switch', { name: /Overdue notifications/ }),
    ).not.toBeChecked()
  })

  it('saves the master switch, digest time and overdue preference', async () => {
    const user = userEvent.setup()
    renderPage(settings())

    await user.click(
      screen.getByRole('switch', { name: /Overdue notifications/ }),
    )
    const time = screen.getByLabelText('Daily digest delivery time')
    await user.clear(time)
    await user.type(time, '18:30')
    await user.click(screen.getByRole('button', { name: 'Save notifications' }))

    expect(mocks.update).toHaveBeenCalledWith({
      notifications_enabled: true,
      reminders_enabled: true,
      daily_digest_enabled: true,
      daily_digest_time: '18:30',
      overdue_notifications_enabled: true,
    })
  })

  it('can turn notifications off without touching channel credentials', async () => {
    const user = userEvent.setup()
    renderPage(settings({ enabled: true }))

    await user.click(
      screen.getByRole('switch', { name: /Enable notifications/ }),
    )
    await user.click(screen.getByRole('button', { name: 'Save notifications' }))

    expect(mocks.update).toHaveBeenCalledWith(
      expect.objectContaining({ notifications_enabled: false }),
    )
  })
})
