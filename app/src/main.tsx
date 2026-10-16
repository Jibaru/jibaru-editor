import { StrictMode } from 'react'
import { createRoot } from 'react-dom/client'
import { invoke } from '@tauri-apps/api/core'
import App from './App'
import { backend } from './backend'
import * as store from './store'
import './styles.css'

import * as edit from './edit'

// Test automation hook (see `automation_loop` in src-tauri/src/main.rs): the
// store's actions plus the backend, and `log` to report back to the terminal.
// The browser dev build (mock backend, Playwright tests) exposes it too.
if ('__TAURI_INTERNALS__' in window)
  Object.assign(window, {
    __jb: { ...store, edit, backend, log: (...a: unknown[]) => invoke('automation_log', { text: a.map(x => (typeof x === 'string' ? x : JSON.stringify(x))).join(' ') }) },
  })
else if (import.meta.env.DEV) Object.assign(window, { __jb: { ...store, edit, backend, log: console.log } })

createRoot(document.getElementById('root')!).render(
  <StrictMode>
    <App />
  </StrictMode>,
)
