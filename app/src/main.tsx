import { StrictMode } from 'react'
import { createRoot } from 'react-dom/client'
import { invoke } from '@tauri-apps/api/core'
import App from './App'
import { backend } from './backend'
import * as store from './store'
import './styles.css'

// Test automation hook (see `automation_loop` in src-tauri/src/main.rs): the
// store's actions plus the backend, and `log` to report back to the terminal.
if ('__TAURI_INTERNALS__' in window)
  Object.assign(window, {
    __jb: { ...store, backend, log: (...a: unknown[]) => invoke('automation_log', { text: a.map(x => (typeof x === 'string' ? x : JSON.stringify(x))).join(' ') }) },
  })

createRoot(document.getElementById('root')!).render(
  <StrictMode>
    <App />
  </StrictMode>,
)
