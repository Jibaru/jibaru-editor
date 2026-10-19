import { StrictMode } from 'react'
import { createRoot } from 'react-dom/client'
import { invoke } from '@tauri-apps/api/core'
import App from './App'
import { backend } from './backend'
import * as store from './store'
import './styles.css'

import * as edit from './edit'
import * as updates from './updates'

// Test automation hook (see `automation_loop` in src-tauri/src/main.rs): the
// store's actions plus the backend, `log` to report back to the terminal and
// `snapshot` to save a screenshot of the webview.
// The browser dev build (mock backend, Playwright tests) exposes it too.
if ('__TAURI_INTERNALS__' in window)
  Object.assign(window, {
    __jb: {
      ...store,
      edit,
      updates,
      backend,
      log: (...a: unknown[]) => invoke('automation_log', { text: a.map(x => (typeof x === 'string' ? x : JSON.stringify(x))).join(' ') }),
      /** Saves a TIFF snapshot of the window's webview (macOS, automation only). */
      snapshot: (path: string) => invoke('automation_snapshot', { path }),
    },
  })
else if (import.meta.env.DEV) Object.assign(window, { __jb: { ...store, edit, updates, backend, log: console.log } })

createRoot(document.getElementById('root')!).render(
  <StrictMode>
    <App />
  </StrictMode>,
)
