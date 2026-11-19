import { mkdir } from 'node:fs/promises'
import { fileURLToPath } from 'node:url'
import path from 'node:path'
import { expect, test, type Page } from '@playwright/test'

const email = process.env.E2E_EMAIL ?? 'e2e@example.com'
const password = process.env.E2E_PASSWORD ?? 'e2e-password'
const outputDir = path.resolve(
  path.dirname(fileURLToPath(import.meta.url)),
  '../../docs/screenshots',
)

test.describe('release screenshots', () => {
  test.skip(
    !process.env.CAPTURE_SCREENSHOTS,
    'set CAPTURE_SCREENSHOTS=1 to regenerate the committed application screenshots',
  )

  test('captures real dashboard, inbox, quick capture and integrations views', async ({
    page,
  }) => {
    test.setTimeout(120_000)
    await mkdir(outputDir, { recursive: true })
    await page.setViewportSize({ width: 1440, height: 900 })

    await page.goto('/login')
    await page.getByRole('textbox', { name: 'Email' }).fill(email)
    await page.getByRole('textbox', { name: 'Password' }).fill(password)
    await page.getByRole('button', { name: 'Log in' }).click()
    await expect(page).toHaveURL(/\/dashboard$/)

    await seedDemoItems(page)

    await expect(
      page.getByRole('heading', { name: 'Dashboard', level: 1 }),
    ).toBeVisible()
    await expect(page.getByText('Review Q1 launch checklist')).toBeVisible()
    await page.screenshot({
      path: path.join(outputDir, 'dashboard.png'),
      animations: 'disabled',
      caret: 'hide',
    })

    await page.getByRole('link', { name: 'Inbox', exact: true }).click()
    await expect(page).toHaveURL(/\/inbox$/)
    await expect(page.getByText('Reply to Maya about release notes')).toBeVisible()
    await page.screenshot({
      path: path.join(outputDir, 'inbox.png'),
      animations: 'disabled',
      caret: 'hide',
    })

    await page.getByRole('button', { name: 'Quick capture' }).first().click()
    const dialog = page.getByRole('dialog')
    await expect(dialog).toBeVisible()
    await dialog
      .getByRole('textbox', { name: 'What is on your mind?' })
      .fill('Book flights for the team offsite')
    await page.screenshot({
      path: path.join(outputDir, 'quick-capture.png'),
      animations: 'disabled',
      caret: 'hide',
    })
    await dialog.getByRole('button', { name: 'Cancel' }).click()

    await page.getByRole('link', { name: 'Integrations', exact: true }).click()
    await expect(page).toHaveURL(/\/integrations$/)
    await expect(
      page.getByRole('heading', { name: 'Integrations', level: 1 }),
    ).toBeVisible()
    await page.screenshot({
      path: path.join(outputDir, 'integrations.png'),
      animations: 'disabled',
      caret: 'hide',
    })
  })
})

async function seedDemoItems(page: Page) {
  const token = await page.evaluate(() => localStorage.getItem('auth_token'))
  expect(token, 'the screenshot user must be logged in').toBeTruthy()

  const tomorrow = new Date(Date.now() + 24 * 60 * 60 * 1000).toISOString()
  const response = await page.request.post('/api/items', {
    headers: { Authorization: `Bearer ${token as string}` },
    data: {
      items: [
        {
          title: 'Review Q1 launch checklist',
          type: 'task',
          priority: 'high',
          due_at: tomorrow,
        },
        {
          title: 'Reply to Maya about release notes',
          type: 'task',
          priority: 'normal',
        },
        {
          title: 'Renew the domain before it expires',
          type: 'reminder',
          priority: 'urgent',
        },
        {
          title: 'Read the Loco deployment guide',
          type: 'note',
          priority: 'low',
        },
      ],
    },
  })
  expect(response.ok()).toBeTruthy()
  await page.reload()
}
