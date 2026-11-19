import { expect, test } from '@playwright/test'

test.describe('PWA shell', () => {
  test('serves a valid manifest', async ({ request }) => {
    const response = await request.get('/manifest.webmanifest')

    expect(response.ok()).toBeTruthy()
    const manifest = await response.json()
    expect(manifest.name).toBe('BrainDrop')
    expect(manifest.start_url).toBe('/')
    expect(manifest.icons.length).toBeGreaterThan(0)
  })

  test('registers the service worker without caching API routes', async ({
    page,
  }) => {
    await page.goto('/login')

    const registered = await page.evaluate(async () => {
      if (!('serviceWorker' in navigator)) {
        return false
      }
      const registration = await navigator.serviceWorker.ready
      return registration.active?.scriptURL.endsWith('/sw.js') ?? false
    })
    expect(registered).toBe(true)

    const cacheContents = await page.evaluate(async () => {
      const names = await caches.keys()
      const paths: string[] = []
      for (const name of names) {
        const cache = await caches.open(name)
        for (const request of await cache.keys()) {
          paths.push(new URL(request.url).pathname)
        }
      }
      return { names, paths }
    })

    expect(cacheContents.names.length).toBeGreaterThan(0)
    expect(
      cacheContents.names.every((name) => name.startsWith('braindrop-shell-')),
    ).toBe(true)
    expect(cacheContents.paths.length).toBeGreaterThan(0)
    expect(
      cacheContents.paths.filter(
        (path) => path.startsWith('/api/') || path === '/api' || path.startsWith('/_'),
      ),
    ).toEqual([])
  })
})
