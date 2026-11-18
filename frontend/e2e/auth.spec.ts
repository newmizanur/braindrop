import { expect, test } from '@playwright/test'

const email = process.env.E2E_EMAIL ?? 'e2e@example.com'
const password = process.env.E2E_PASSWORD ?? 'e2e-password'

test.describe('authentication smoke', () => {
  test('redirects a protected route to login', async ({ page }) => {
    await page.goto('/settings')

    await expect(page).toHaveURL(/\/login$/)
    await expect(
      page.getByRole('heading', { name: 'Welcome back' }),
    ).toBeVisible()
  })

  test('hides registration when the instance disables it', async ({ page }) => {
    await page.goto('/login')

    await expect(
      page.getByRole('heading', { name: 'Welcome back' }),
    ).toBeVisible()
    await expect(page.getByRole('link', { name: 'Sign up' })).toHaveCount(0)
    await expect(
      page.getByText('Public registration is disabled.'),
    ).toBeVisible()
  })

  test('logs in, reaches the dashboard and logs out', async ({ page }) => {
    await page.goto('/login')

    await page.getByRole('textbox', { name: 'Email' }).fill(email)
    await page.getByRole('textbox', { name: 'Password' }).fill(password)
    await page.getByRole('button', { name: 'Log in' }).click()

    await expect(page).toHaveURL(/\/dashboard$/)
    await expect(
      page.getByRole('heading', { name: 'Dashboard', level: 1 }),
    ).toBeVisible()

    await page.getByRole('button', { name: 'Account menu' }).click()
    await page.getByRole('menuitem', { name: 'Log out' }).click()

    await expect(page).toHaveURL(/\/login$/)
    await expect(
      page.getByRole('heading', { name: 'Welcome back' }),
    ).toBeVisible()
  })

  test('rejects bad credentials without a full page reload', async ({ page }) => {
    await page.goto('/login')

    await page.getByRole('textbox', { name: 'Email' }).fill(email)
    await page.getByRole('textbox', { name: 'Password' }).fill('definitely-wrong')
    await page.getByRole('button', { name: 'Log in' }).click()

    await expect(page.getByRole('alert')).toContainText(
      /permission|invalid|unauthorized/i,
    )
    await expect(page).toHaveURL(/\/login$/)
    await expect(page.getByRole('textbox', { name: 'Email' })).toHaveValue(email)
  })
})
