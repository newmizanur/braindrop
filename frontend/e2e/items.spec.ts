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

test.describe('item lifecycle', () => {
  test('captures, completes, reopens, edits and deletes an item', async ({
    page,
  }) => {
    await login(page)

    const title = `E2E task ${Date.now()}`
    const editedTitle = `${title} edited`

    // Capture through the header dialog.
    await page.getByRole('button', { name: 'Quick capture' }).first().click()
    await page
      .getByRole('textbox', { name: 'What is on your mind?' })
      .fill(title)
    await page.getByRole('button', { name: 'Save to inbox' }).click()
    await expect(page.getByText('Captured', { exact: true })).toBeVisible()

    // It is searchable in the all-items view.
    await page.getByRole('link', { name: 'All items' }).click()
    await expect(page).toHaveURL(/\/items$/)
    await page.getByRole('textbox', { name: 'Search items' }).fill(title)
    await expect(page.getByText(title)).toBeVisible()

    // Complete it, then clear the search.
    await page.getByRole('checkbox', { name: `Complete ${title}` }).click()
    await expect(
      page.getByRole('checkbox', { name: `Reopen ${title}` }),
    ).toBeVisible()
    await page.getByRole('textbox', { name: 'Search items' }).fill('')

    // Completed page shows it; reopening moves it back out.
    await page.getByRole('link', { name: 'Completed' }).click()
    await expect(page).toHaveURL(/\/completed$/)
    await expect(page.getByText(title)).toBeVisible()
    await page.getByRole('checkbox', { name: `Reopen ${title}` }).click()
    await expect(page.getByText(title)).toHaveCount(0)

    // The inbox has it again.
    await page.getByRole('link', { name: 'Inbox' }).click()
    await expect(page).toHaveURL(/\/inbox$/)
    await expect(page.getByText(title)).toBeVisible()

    // Edit the title.
    await page.getByRole('button', { name: `Actions for ${title}` }).click()
    await page.getByRole('menuitem', { name: 'Edit' }).click()
    await expect(page.getByRole('dialog')).toContainText('Edit item')
    await page.getByRole('textbox', { name: 'Title' }).fill(editedTitle)
    await page.getByRole('button', { name: 'Save changes' }).click()
    await expect(page.getByText(editedTitle)).toBeVisible()

    // Delete it after confirming.
    await page
      .getByRole('button', { name: `Actions for ${editedTitle}` })
      .click()
    await page.getByRole('menuitem', { name: 'Delete' }).click()
    await expect(page.getByRole('dialog')).toContainText('Delete item')
    await page.getByRole('button', { name: 'Delete', exact: true }).click()
    await expect(page.getByText(editedTitle)).toHaveCount(0)
  })

  test('creates an item from the page and keeps it in the chosen view', async ({
    page,
  }) => {
    await login(page)

    const title = `E2E note ${Date.now()}`

    await page.getByRole('link', { name: 'Inbox' }).click()
    await page.getByRole('button', { name: 'New item' }).click()
    await expect(page.getByRole('dialog')).toContainText('New item')

    await page.getByRole('textbox', { name: 'Title' }).fill(title)
    await page.getByRole('button', { name: 'Create item' }).click()

    await expect(page.getByText(title)).toBeVisible()

    // Clean up so repeated runs do not accumulate items.
    await page.getByRole('button', { name: `Actions for ${title}` }).click()
    await page.getByRole('menuitem', { name: 'Delete' }).click()
    await page.getByRole('button', { name: 'Delete', exact: true }).click()
    await expect(page.getByText(title)).toHaveCount(0)
  })
})
