import { expect, test } from '@playwright/test'

const email = process.env.E2E_EMAIL ?? 'e2e@example.com'
const password = process.env.E2E_PASSWORD ?? 'e2e-password'

// Values only need to pass server-side validation. Nothing is sent to a real
// provider: the workflow saves configuration and preferences without pressing
// "Send test message", so CI never needs live Telegram or Discord credentials.
const TELEGRAM_TOKEN = '123456789:abcdefghijklmnopqrstuvwxyz'
const TELEGRAM_CHAT = '-1001234567890'
const DISCORD_WEBHOOK =
  'https://discord.com/api/webhooks/123456789012345678/abcdefghijklmnopqrstuvwxyz0123456789ABCD'

async function login(page: import('@playwright/test').Page) {
  await page.goto('/login')
  await page.getByRole('textbox', { name: 'Email' }).fill(email)
  await page.getByRole('textbox', { name: 'Password' }).fill(password)
  await page.getByRole('button', { name: 'Log in' }).click()
  await expect(page).toHaveURL(/\/dashboard$/)
}

test.describe('notification configuration', () => {
  test('configures both channels and the notification preferences', async ({
    page,
  }) => {
    await login(page)

    await page.getByRole('link', { name: 'Integrations' }).click()
    await expect(page).toHaveURL(/\/integrations$/)

    const telegram = page
      .locator('form')
      .filter({ has: page.getByRole('heading', { name: 'Telegram' }) })
    await telegram
      .getByRole('switch', { name: /Enable Telegram/ })
      .check()
    await telegram
      .getByLabel('Bot token')
      .fill(TELEGRAM_TOKEN)
    await telegram.getByLabel('Chat ID').fill(TELEGRAM_CHAT)
    await telegram.getByRole('button', { name: 'Save Telegram' }).click()
    await expect(page.getByText('Telegram saved')).toBeVisible()
    await expect(telegram.getByText('Enabled', { exact: true })).toBeVisible()

    const discord = page
      .locator('form')
      .filter({ has: page.getByRole('heading', { name: 'Discord' }) })
    await discord
      .getByRole('switch', { name: /Enable Discord/ })
      .check()
    await discord.getByLabel('Webhook URL').fill(DISCORD_WEBHOOK)
    await discord.getByRole('button', { name: 'Save Discord' }).click()
    await expect(page.getByText('Discord saved')).toBeVisible()
    await expect(discord.getByText('Enabled', { exact: true })).toBeVisible()

    // A reload proves the configuration persisted without the API ever
    // returning the stored secret.
    await page.reload()
    const telegramAfterReload = page
      .locator('form')
      .filter({ has: page.getByRole('heading', { name: 'Telegram' }) })
    const discordAfterReload = page
      .locator('form')
      .filter({ has: page.getByRole('heading', { name: 'Discord' }) })
    await expect(
      telegramAfterReload.getByText('Enabled', { exact: true }),
    ).toBeVisible()
    await expect(
      discordAfterReload.getByText('Enabled', { exact: true }),
    ).toBeVisible()
    await expect(telegramAfterReload.getByLabel('Bot token')).toHaveValue('')
    await expect(discordAfterReload.getByLabel('Webhook URL')).toHaveValue('')
    await expect(telegramAfterReload.getByLabel('Chat ID')).toHaveValue(
      TELEGRAM_CHAT,
    )

    await page.getByRole('link', { name: 'Settings' }).click()
    await expect(page).toHaveURL(/\/settings$/)

    const preferences = page
      .locator('form')
      .filter({ has: page.getByRole('heading', { name: 'Notifications' }) })
    // Set an explicit end state so the test is deterministic across runs; the
    // notification switches all default to off on a fresh account.
    // Keep the master switch off so the running worker can never attempt a
    // provider request with the fake credentials saved above.
    await preferences
      .getByRole('switch', { name: /Enable notifications/ })
      .uncheck()
    await preferences
      .getByRole('switch', { name: /Daily digest/ })
      .uncheck()
    await preferences
      .getByLabel('Daily digest delivery time')
      .fill('18:30')
    await preferences
      .getByRole('switch', { name: /Overdue notifications/ })
      .check()
    await preferences
      .getByRole('button', { name: 'Save notifications' })
      .click()
    await expect(page.getByText('Notification settings saved')).toBeVisible()

    await page.reload()
    const preferencesAfterReload = page
      .locator('form')
      .filter({ has: page.getByRole('heading', { name: 'Notifications' }) })
    await expect(
      preferencesAfterReload.getByLabel('Daily digest delivery time'),
    ).toHaveValue('18:30')
    await expect(
      preferencesAfterReload.getByRole('switch', {
        name: /Enable notifications/,
      }),
    ).not.toBeChecked()
    await expect(
      preferencesAfterReload.getByRole('switch', { name: /Daily digest/ }),
    ).not.toBeChecked()
    await expect(
      preferencesAfterReload.getByRole('switch', {
        name: /Overdue notifications/,
      }),
    ).toBeChecked()

    // Leave the account without stored provider credentials for repeat runs.
    await page.getByRole('link', { name: 'Integrations' }).click()
    const telegramCleanup = page
      .locator('form')
      .filter({ has: page.getByRole('heading', { name: 'Telegram' }) })
    await telegramCleanup
      .getByRole('button', { name: 'Remove credentials' })
      .click()
    await expect(page.getByText('Telegram removed')).toBeVisible()

    const discordCleanup = page
      .locator('form')
      .filter({ has: page.getByRole('heading', { name: 'Discord' }) })
    await discordCleanup
      .getByRole('button', { name: 'Remove webhook' })
      .click()
    await expect(page.getByText('Discord removed')).toBeVisible()
  })
})
