// UI tests against the browser build with the mock backend (src/backend/mock.ts)
// and the dev media server (dev/mediaServer.ts). Run: `pnpm test:ui`.
// Needs ffmpeg/ffprobe on PATH (fixtures are generated in tests/setup.ts).

import { defineConfig } from '@playwright/test'

const port = Number(process.env.JB_UI_PORT ?? 1430)

export default defineConfig({
  testDir: 'tests',
  globalSetup: './tests/setup.ts',
  timeout: 30_000,
  workers: process.env.CI ? 1 : 3,
  reporter: [['list']],
  use: {
    baseURL: `http://localhost:${port}`,
    viewport: { width: 1512, height: 944 },
    headless: true,
  },
  webServer: {
    command: `pnpm exec vite --port ${port} --strictPort`,
    url: `http://localhost:${port}`,
    reuseExistingServer: true,
    timeout: 60_000,
  },
})
