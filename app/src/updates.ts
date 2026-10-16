// In-app updates (tauri-plugin-updater). The app checks
// releases/latest/download/latest.json on GitHub at startup and every 24 h,
// shows a notice when a newer version is out, and on "Update now" downloads
// the package (the plugin verifies its signature against the public key in
// tauri.conf.json), installs it and relaunches. Installing waits while an
// export runs. See docs/updates.md.

import { useSyncExternalStore } from 'react'
import { invoke } from '@tauri-apps/api/core'
import { confirmDiscard, getState as editorState, subscribe as subscribeEditor } from './store'

const inTauri = typeof window !== 'undefined' && '__TAURI_INTERNALS__' in window
const DAY = 24 * 60 * 60 * 1000
const AUTO_KEY = 'jb.updates.auto'
const SKIP_KEY = 'jb.updates.skip'

export interface UpdateInfo {
  version: string
  current: string
  notes: string
  date?: string
}

export type UpdatePhase =
  | { kind: 'hidden' }
  | { kind: 'checking' }
  | { kind: 'current'; version: string }
  | { kind: 'available'; info: UpdateInfo }
  /** "Update now" pressed while an export runs: installs when it ends. */
  | { kind: 'waiting-export'; info: UpdateInfo }
  | { kind: 'downloading'; info: UpdateInfo; done: number; total: number | null }
  | { kind: 'installing'; info: UpdateInfo }
  | { kind: 'error'; message: string; info?: UpdateInfo }

type Plugin = typeof import('@tauri-apps/plugin-updater')
type Update = NonNullable<Awaited<ReturnType<Plugin['check']>>>

let phase: UpdatePhase = { kind: 'hidden' }
let pending: Update | null = null
let busy = false
const listeners = new Set<() => void>()

function set(p: UpdatePhase) {
  phase = p
  listeners.forEach(l => l())
}

export const updatePhase = () => phase

export function useUpdatePhase(): UpdatePhase {
  return useSyncExternalStore(
    l => {
      listeners.add(l)
      return () => listeners.delete(l)
    },
    () => phase,
  )
}

const read = (k: string) => {
  try {
    return localStorage.getItem(k)
  } catch {
    return null
  }
}
const write = (k: string, v: string | null) => {
  try {
    if (v == null) localStorage.removeItem(k)
    else localStorage.setItem(k, v)
  } catch {
    /* storage unavailable */
  }
}

/** The "Check for updates automatically" setting (on by default). */
export const autoCheckEnabled = () => read(AUTO_KEY) !== 'off'

export function setAutoCheck(on: boolean) {
  write(AUTO_KEY, on ? null : 'off')
  syncMenu()
  listeners.forEach(l => l())
}

function syncMenu() {
  if (inTauri) invoke('set_menu_checked', { id: 'help.auto_updates', checked: autoCheckEnabled() }).catch(() => {})
}

const exporting = () => editorState().exportJob?.state === 'running'

/** Resolves once no export is running. */
function exportIdle(): Promise<void> {
  if (!exporting()) return Promise.resolve()
  return new Promise(resolve => {
    const un = subscribeEditor(() => {
      if (!exporting()) {
        un()
        resolve()
      }
    })
  })
}

export async function currentVersion(): Promise<string> {
  if (!inTauri) return '0.0.0'
  const { getVersion } = await import('@tauri-apps/api/app')
  return getVersion()
}

function infoOf(u: Update): UpdateInfo {
  return { version: u.version, current: u.currentVersion, notes: (u.body ?? '').trim(), date: u.date }
}

/**
 * Checks for a newer version. `manual` (Help > Check for Updates...) always
 * reports the result, including "up to date" and errors; automatic checks stay
 * silent unless a version the user did not skip is available, and wait for a
 * running export to finish before showing the notice.
 */
export async function checkForUpdates(manual = false): Promise<UpdateInfo | null> {
  if (busy) return pending ? infoOf(pending) : null
  if (!inTauri) {
    if (manual) set({ kind: 'error', message: 'Updates are available in the desktop app.' })
    return null
  }
  // A notice is already up (or a download is running): keep it.
  if (pending && ['available', 'waiting-export', 'downloading', 'installing'].includes(phase.kind)) return infoOf(pending)
  busy = true
  if (manual) set({ kind: 'checking' })
  try {
    const { check } = await import('@tauri-apps/plugin-updater')
    const update = await check({ timeout: 30_000 })
    if (pending && pending !== update) pending.close().catch(() => {})
    pending = update
    if (!update) {
      if (manual) set({ kind: 'current', version: await currentVersion() })
      return null
    }
    const info = infoOf(update)
    if (!manual) {
      if (read(SKIP_KEY) === info.version) return info
      if (exporting()) {
        // Don't block manual checks while waiting for the export.
        busy = false
        await exportIdle()
        if (pending !== update || phase.kind !== 'hidden') return info
      }
    }
    if (phase.kind === 'hidden' || phase.kind === 'checking' || phase.kind === 'current' || phase.kind === 'error')
      set({ kind: 'available', info })
    return info
  } catch (e) {
    if (manual) set({ kind: 'error', message: `Could not check for updates: ${e}` })
    else console.warn('update check failed', e)
    return null
  } finally {
    busy = false
  }
}

/** Hides the notice; it comes back on the next automatic check. */
export function remindLater() {
  if (phase.kind === 'downloading' || phase.kind === 'installing') return
  set({ kind: 'hidden' })
}

/** Automatic checks stop announcing this version (a manual check still shows it). */
export function skipVersion() {
  const v = phase.kind === 'available' || phase.kind === 'waiting-export' ? phase.info.version : pending?.version
  if (v) write(SKIP_KEY, v)
  set({ kind: 'hidden' })
}

export function dismiss() {
  if (phase.kind === 'downloading' || phase.kind === 'installing') return
  set({ kind: 'hidden' })
}

/**
 * Downloads, verifies and installs the pending update, then relaunches.
 * Asks to save unsaved changes first (`prompt: false` skips that, for
 * automation) and never starts the install while an export is running.
 */
export async function updateNow({ prompt = true }: { prompt?: boolean } = {}): Promise<void> {
  const update = pending
  if (!update || busy) return
  const info = infoOf(update)
  busy = true
  try {
    if (prompt && !(await confirmDiscard())) return
    if (exporting()) {
      set({ kind: 'waiting-export', info })
      await exportIdle()
    }
    set({ kind: 'downloading', info, done: 0, total: null })
    let done = 0
    let total: number | null = null
    // `download` fails if the signature does not match our public key.
    await update.download(ev => {
      if (ev.event === 'Started') total = ev.data.contentLength ?? null
      else if (ev.event === 'Progress') done += ev.data.chunkLength
      set({ kind: 'downloading', info, done, total })
    })
    // An export started during the download: install once it is done.
    if (exporting()) {
      set({ kind: 'waiting-export', info })
      await exportIdle()
    }
    set({ kind: 'installing', info })
    await invoke('allow_exit')
    // Windows: runs the installer and quits; it relaunches the new version.
    await update.install()
    const { relaunch } = await import('@tauri-apps/plugin-process')
    await relaunch()
  } catch (e) {
    set({ kind: 'error', message: `The update failed: ${e}`, info })
  } finally {
    busy = false
  }
}

let started = false
/** Startup check (after a short delay) and one every 24 h while running. */
export function startUpdateChecks() {
  if (!inTauri || started) return
  started = true
  syncMenu()
  const auto = () => {
    if (autoCheckEnabled()) checkForUpdates(false)
  }
  setTimeout(auto, 8_000)
  setInterval(auto, DAY)
}
