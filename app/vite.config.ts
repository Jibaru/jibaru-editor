import react from '@vitejs/plugin-react'
import { defineConfig } from 'vite'
import { devMediaServer } from './dev/mediaServer'

// Tauri expects a fixed port and fails if it is taken.
const host = process.env.TAURI_DEV_HOST

export default defineConfig({
  plugins: [react(), devMediaServer()],
  clearScreen: false,
  server: {
    port: 1420,
    strictPort: true,
    host: host || false,
    hmr: host ? { protocol: 'ws', host, port: 1421 } : undefined,
    watch: { ignored: ['**/src-tauri/**'] },
  },
  envPrefix: ['VITE_', 'TAURI_ENV_'],
  build: {
    target: process.env.TAURI_ENV_PLATFORM === 'windows' ? 'chrome105' : 'safari15',
    minify: !process.env.TAURI_ENV_DEBUG,
    sourcemap: !!process.env.TAURI_ENV_DEBUG,
  },
})
