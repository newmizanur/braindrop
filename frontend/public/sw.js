/*
 * BrainDrop service worker.
 *
 * It caches only the application shell and static assets. It never caches
 * `/api/*`, monitoring endpoints, or anything carrying credentials: a shared
 * or stolen device must not be able to read another session's private data
 * from the cache. API requests are always handled by the network.
 */

// Bumped for v0.1.0 so older shells (and any stale cached responses) are
// discarded when the release service worker activates.
const CACHE = 'braindrop-shell-v2'
const SHELL = [
  '/',
  '/index.html',
  '/manifest.webmanifest',
  '/favicon.svg',
  '/icons/icon-192.png',
  '/icons/icon-512.png',
]

self.addEventListener('install', (event) => {
  event.waitUntil(
    caches
      .open(CACHE)
      .then((cache) => cache.addAll(SHELL))
      .then(() => self.skipWaiting()),
  )
})

self.addEventListener('activate', (event) => {
  event.waitUntil(
    caches
      .keys()
      .then((keys) =>
        Promise.all(
          keys.filter((key) => key !== CACHE).map((key) => caches.delete(key)),
        ),
      )
      .then(() => self.clients.claim()),
  )
})

function isPrivatePath(pathname) {
  return (
    pathname.startsWith('/api/') ||
    pathname.startsWith('/_') ||
    pathname === '/api'
  )
}

self.addEventListener('fetch', (event) => {
  const request = event.request
  if (request.method !== 'GET') {
    return
  }

  const url = new URL(request.url)
  if (url.origin !== self.location.origin || isPrivatePath(url.pathname)) {
    return
  }

  // Navigations: network first so a deploy is picked up, app shell as fallback
  // when the network is unavailable.
  if (request.mode === 'navigate') {
    event.respondWith(
      fetch(request)
        .then((response) => {
          if (response.ok) {
            const copy = response.clone()
            event.waitUntil(
              caches
                .open(CACHE)
                .then((cache) => cache.put('/index.html', copy)),
            )
          }
          return response
        })
        .catch(() => caches.match('/index.html')),
    )
    return
  }

  // Static assets: cache first, refresh in the background.
  if (
    url.pathname.startsWith('/assets/') ||
    /\.(?:css|js|mjs|woff2?|png|svg|ico|webmanifest)$/.test(url.pathname)
  ) {
    event.respondWith(
      caches.match(request).then((cached) => {
        const network = fetch(request)
          .then((response) => {
            if (response.ok) {
              const copy = response.clone()
              event.waitUntil(
                caches.open(CACHE).then((cache) => cache.put(request, copy)),
              )
            }
            return response
          })
          .catch(() => cached)
        return cached || network
      }),
    )
  }
})
