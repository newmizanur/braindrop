import { expect, test, type Page } from '@playwright/test'

const email = process.env.E2E_EMAIL ?? 'e2e@example.com'
const password = process.env.E2E_PASSWORD ?? 'e2e-password'

type CspViolation = {
  blockedURI: string
  directive: string
  violatedDirective: string
}

declare global {
  interface Window {
    __braindropCspViolations?: CspViolation[]
  }
}

async function watchCsp(page: Page) {
  await page.addInitScript(() => {
    window.__braindropCspViolations = []
    document.addEventListener('securitypolicyviolation', (event) => {
      window.__braindropCspViolations?.push({
        blockedURI: event.blockedURI,
        directive: event.originalPolicy,
        violatedDirective: event.violatedDirective,
      })
    })
  })
}

async function cspViolations(page: Page): Promise<CspViolation[]> {
  return page.evaluate(() => window.__braindropCspViolations ?? [])
}

function header(headers: Record<string, string>, name: string): string {
  const value = Object.entries(headers).find(
    ([key]) => key.toLowerCase() === name.toLowerCase(),
  )
  return value?.[1] ?? ''
}

test.describe('browser security controls', () => {
  test('serves hardened headers without storing private responses', async ({
    request,
  }) => {
    const html = await request.get('/login')
    expect(html.ok()).toBeTruthy()
    const htmlHeaders = html.headers()

    const csp = header(htmlHeaders, 'content-security-policy')
    expect(csp).toContain("default-src 'self'")
    expect(csp).toContain("script-src 'self'")
    expect(csp).toContain("script-src-attr 'none'")
    expect(csp).toContain("frame-ancestors 'none'")
    expect(csp).not.toContain('unsafe-eval')

    expect(header(htmlHeaders, 'x-content-type-options')).toBe('nosniff')
    expect(header(htmlHeaders, 'x-frame-options')).toBe('DENY')
    expect(header(htmlHeaders, 'referrer-policy')).toBe('no-referrer')
    expect(header(htmlHeaders, 'cross-origin-opener-policy')).toBe('same-origin')
    expect(header(htmlHeaders, 'cross-origin-resource-policy')).toBe(
      'same-origin',
    )
    expect(header(htmlHeaders, 'cache-control')).toBe('no-store')

    const api = await request.get('/api/auth/config')
    expect(api.ok()).toBeTruthy()
    expect(header(api.headers(), 'cache-control')).toBe('no-store')

    const body = await html.text()
    const asset = body.match(/\/assets\/[^"']+\.js/)?.[0]
    expect(asset, 'the built SPA must reference a hashed JS asset').toBeTruthy()
    const js = await request.get(asset as string)
    expect(js.ok()).toBeTruthy()
    expect(header(js.headers(), 'cache-control')).not.toBe('no-store')
  })

  test('external theme script sets the pre-paint color scheme under CSP', async ({
    page,
  }) => {
    await watchCsp(page)
    await page.addInitScript(() => {
      localStorage.setItem('mantine-color-scheme-value', 'dark')
    })

    const response = await page.goto('/login')
    expect(response?.ok()).toBeTruthy()
    await expect(page.locator('html')).toHaveAttribute(
      'data-mantine-color-scheme',
      'dark',
    )
    expect(await cspViolations(page)).toEqual([])
  })

  test('full authenticated flow has no CSP violations', async ({ page }) => {
    test.setTimeout(90_000)
    await watchCsp(page)
    await page.goto('/login')

    await page.getByRole('textbox', { name: 'Email' }).fill(email)
    await page.getByRole('textbox', { name: 'Password' }).fill(password)
    await page.getByRole('button', { name: 'Log in' }).click()
    await expect(page).toHaveURL(/\/dashboard$/)

    // Exercise the lazy quick-capture chunk, Mantine's injected styles, and
    // the date picker, all of which must work without inline/eval scripts.
    await page.getByRole('button', { name: 'Quick capture' }).first().click()
    await expect(page.getByRole('dialog')).toBeVisible()
    await page
      .getByRole('textbox', { name: 'What is on your mind?' })
      .fill('Verify CSP with a capture')
    await page.getByRole('button', { name: 'Due' }).click()
    await expect(page.getByRole('dialog', { name: 'Due' })).toBeVisible()
    await page.keyboard.press('Escape')
    await page.getByRole('dialog', { name: 'Quick capture' }).getByRole('button', { name: 'Cancel' }).click()

    const themeToggle = page.getByRole('button', {
      name: /Switch to (dark|light) mode/,
    })
    await themeToggle.click()
    await themeToggle.click()

    for (const [link, heading] of [
      ['Inbox', 'Inbox'],
      ['Today', 'Today'],
      ['Upcoming', 'Upcoming'],
      ['All items', 'All items'],
      ['Completed', 'Completed'],
      ['Settings', 'Settings'],
      ['Integrations', 'Integrations'],
    ] as const) {
      await page.getByRole('link', { name: link, exact: true }).click()
      await expect(
        page.getByRole('heading', { level: 1, name: heading }),
      ).toBeVisible()
    }

    const swReady = await page.evaluate(async () => {
      if (!('serviceWorker' in navigator)) return false
      const registration = await navigator.serviceWorker.ready
      return registration.active?.scriptURL.endsWith('/sw.js') ?? false
    })
    expect(swReady).toBe(true)

    expect(await cspViolations(page)).toEqual([])
  })
})
