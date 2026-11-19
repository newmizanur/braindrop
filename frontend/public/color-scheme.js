/*
 * Resolve the Mantine color scheme before first paint to avoid a flash.
 *
 * This lives in a separate file so the production CSP can stay free of
 * `unsafe-inline` for scripts.
 */
;(() => {
  try {
    const stored = localStorage.getItem('mantine-color-scheme-value')
    const scheme =
      stored === 'light' || stored === 'dark'
        ? stored
        : window.matchMedia('(prefers-color-scheme: dark)').matches
          ? 'dark'
          : 'light'
    document.documentElement.setAttribute('data-mantine-color-scheme', scheme)
  } catch {
    document.documentElement.setAttribute('data-mantine-color-scheme', 'light')
  }
})()
