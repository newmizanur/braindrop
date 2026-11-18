import { expect, test } from '@playwright/test'

const email = process.env.E2E_EMAIL ?? 'e2e@example.com'
const password = process.env.E2E_PASSWORD ?? 'e2e-password'

async function login(page: import('@playwright/test').Page) {
  await page.goto('/login')
  await page.getByRole('textbox', { name: 'Email' }).fill(email)
  await page.getByRole('textbox', { name: 'Password' }).fill(password)
  await page.getByRole('button', { name: 'Log in' }).click()
  await expect(page).toHaveURL(/\/dashboard$/)
}

async function openCapture(page: import('@playwright/test').Page) {
  await page.getByRole('button', { name: 'Quick capture' }).first().click()
  await expect(page.getByRole('dialog')).toContainText('Quick capture')
}

test.describe('AI-assisted capture', () => {
  test('organizes, edits, removes and saves reviewed suggestions', async ({
    page,
  }) => {
    await login(page)
    await openCapture(page)

    const title = `E2E AI item ${Date.now()}`
    await page
      .getByRole('textbox', { name: 'What is on your mind?' })
      .fill('buy oat milk and renew the domain')
    await page.getByRole('button', { name: 'Organize with AI' }).click()

    // The mock provider returns two suggestions; both start selected.
    await expect(page.getByText('AI suggestions')).toBeVisible()
    await expect(
      page.getByRole('textbox', { name: 'Suggestion 1 title' }),
    ).toHaveValue('Buy oat milk')
    await expect(page.getByText('2 selected')).toBeVisible()

    await page
      .getByRole('textbox', { name: 'Suggestion 1 title' })
      .fill(title)
    await page
      .getByRole('textbox', { name: 'Suggestion 1 tags' })
      .fill('home, errands')

    // Remove the second suggestion so only the edited one is saved.
    await page.getByRole('button', { name: 'Remove suggestion 2' }).click()
    await expect(
      page.getByRole('textbox', { name: 'Suggestion 2 title' }),
    ).toHaveCount(0)

    await page.getByRole('button', { name: 'Save selected' }).click()
    await expect(page.getByText('Suggestion saved')).toBeVisible()

    await page.getByRole('link', { name: 'Inbox', exact: true }).click()
    await expect(page).toHaveURL(/\/inbox$/)
    await expect(page.getByText(title)).toBeVisible()
    await expect(page.getByText('Renew the domain')).toHaveCount(0)

    // Clean up so repeated runs do not accumulate items.
    await page.getByRole('button', { name: `Actions for ${title}` }).click()
    await page.getByRole('menuitem', { name: 'Delete' }).click()
    await page.getByRole('button', { name: 'Delete', exact: true }).click()
    await expect(page.getByText(title)).toHaveCount(0)
  })

  test('keeps manual capture usable when the provider fails', async ({
    page,
  }) => {
    await login(page)
    await openCapture(page)

    const title = `E2E manual fallback ${Date.now()}`
    await page
      .getByRole('textbox', { name: 'What is on your mind?' })
      .fill(`E2E_AI_FAIL ${title}`)
    await page.getByRole('button', { name: 'Organize with AI' }).click()

    await expect(
      page.getByText('The AI provider is unavailable right now.'),
    ).toBeVisible()

    // The dialog stays in capture mode and the manual save path still works.
    await page.getByRole('textbox', { name: 'What is on your mind?' }).fill(title)
    await page.getByRole('button', { name: 'Save to inbox' }).click()
    await expect(page.getByText('Captured', { exact: true })).toBeVisible()

    await page.getByRole('link', { name: 'Inbox', exact: true }).click()
    await expect(page.getByText(title)).toBeVisible()

    await page.getByRole('button', { name: `Actions for ${title}` }).click()
    await page.getByRole('menuitem', { name: 'Delete' }).click()
    await page.getByRole('button', { name: 'Delete', exact: true }).click()
    await expect(page.getByText(title)).toHaveCount(0)
  })
})
