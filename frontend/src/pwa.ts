/**
 * Register the app-shell service worker in production builds only.
 *
 * Vite's dev server already serves modules with its own HMR client; a service
 * worker there would cache stale modules and confuse development. The worker
 * itself never caches `/api/*` or credentials.
 */
export function registerServiceWorker() {
  if (!('serviceWorker' in navigator) || !import.meta.env.PROD) {
    return
  }

  window.addEventListener('load', () => {
    navigator.serviceWorker.register('/sw.js').catch(() => {
      // Installability is a progressive enhancement; the app works without it.
    })
  })
}
