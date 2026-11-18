import { defineConfig, devices } from '@playwright/test'

// In Docker CI the app is already running and BASE_URL points at its published
// port. Locally, Playwright starts an isolated server through the webServer
// command, which builds the SPA and seeds a dedicated e2e database.
const baseURL = process.env.BASE_URL ?? 'http://127.0.0.1:5199'

export default defineConfig({
  testDir: './e2e',
  outputDir: './test-results',
  // Starts a local OpenRouter-compatible mock so the AI flow needs no real
  // credentials. It listens on 127.0.0.1, which Docker CI shares with the app.
  globalSetup: './e2e/global-setup.ts',
  fullyParallel: false,
  forbidOnly: Boolean(process.env.CI),
  retries: process.env.CI ? 1 : 0,
  workers: 1,
  reporter: [['list']],
  use: {
    baseURL,
    trace: 'retain-on-failure',
    screenshot: 'only-on-failure',
  },
  projects: [
    {
      name: 'chromium',
      use: { ...devices['Desktop Chrome'] },
    },
  ],
  webServer: process.env.BASE_URL
    ? undefined
    : {
        command: 'bash ../scripts/e2e-server.sh',
        url: 'http://127.0.0.1:5199/_health',
        reuseExistingServer: !process.env.CI,
        timeout: 240_000,
      },
})
