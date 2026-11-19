import { loadEnv } from 'vite'
import { defineConfig } from 'vitest/config'
import react from '@vitejs/plugin-react'

// The backend reads its port from `PORT` (see `config/development.yaml`:
// `get_env(name="PORT", default="5150")`), so this proxy has to read the same
// variable. Hardcoding the default silently sends every `/api` call to 5150 the
// moment anyone overrides it.
//
// `loadEnv` is Vite's own reader; the empty prefix takes shell variables too.
// It is used here for the dev server only — nothing is exposed to the client.
export default defineConfig(({ mode }) => {
  const env = loadEnv(mode, '..', '')
  const backendPort = env.PORT || '5150'

  return {
    plugins: [react()],
    build: {
      outDir: 'dist',
      rolldownOptions: {
        output: {
          codeSplitting: {
            groups: [
              {
                name: 'react-vendor',
                test: /node_modules\/(?:react|react-dom|react-router|scheduler)\//,
              },
              {
                name: 'ui-vendor',
                test: /node_modules\/(?:@mantine|@tabler|@floating-ui)\//,
              },
              {
                name: 'data-vendor',
                test: /node_modules\/(?:@tanstack|dayjs)\//,
              },
            ],
          },
        },
      },
    },
    server: {
      port: 5173,
      proxy: { '/api': `http://localhost:${backendPort}` },
    },
    test: {
      environment: 'jsdom',
      setupFiles: './src/test/setup.ts',
      css: false,
      // Playwright owns `e2e/`; Vitest must not try to run those files.
      include: ['src/**/*.{test,spec}.{ts,tsx}'],
      exclude: ['e2e/**', 'node_modules/**', 'dist/**'],
    },
  }
})
