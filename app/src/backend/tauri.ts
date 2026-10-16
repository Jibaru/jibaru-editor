import { convertFileSrc, invoke } from '@tauri-apps/api/core'
import { listen } from '@tauri-apps/api/event'
import { open, save } from '@tauri-apps/plugin-dialog'
import type { Command, MediaAsset, ProjectSettings, Snapshot } from '../core/types'
import type { Backend, FxEntry, MediaUrls } from './types'

const isWindows = navigator.userAgent.includes('Windows')

/** URL for the app's custom `jb` protocol (see src-tauri/src/media.rs). */
function jb(route: string, query: Record<string, string | number>): string {
  const qs = new URLSearchParams(Object.entries(query).map(([k, v]) => [k, String(v)])).toString()
  return isWindows ? `http://jb.localhost/${route}?${qs}` : `jb://localhost/${route}?${qs}`
}

const media: MediaUrls = {
  thumb: (path, t, h) => jb('thumb', { path, t: t.toFixed(2), h }),
  wave: (path, pps) => jb('wave', { path, pps }),
  file: path => convertFileSrc(path),
  frame: (t, h, revision) => jb('frame', { t: t.toFixed(4), h, r: revision }),
}

export const tauriBackend: Backend = {
  kind: 'tauri',
  snapshot: () => invoke<Snapshot>('get_project'),
  newProject: (name: string, settings?: Partial<ProjectSettings>) => invoke<Snapshot>('new_project', { name, settings: settings ?? null }),
  openProject: (path: string) => invoke<Snapshot>('open_project', { path }),
  saveProject: (path?: string) => invoke<Snapshot>('save_project', { path: path ?? null }),
  execute: (command: Command) => invoke<Snapshot>('execute', { command }),
  undo: () => invoke<Snapshot>('undo'),
  redo: () => invoke<Snapshot>('redo'),
  probeMedia: (paths: string[]) => invoke<MediaAsset[]>('probe_media', { paths }),
  importMedia: (paths: string[]) => invoke<Snapshot>('import_media', { paths }),
  fxCatalog: () => invoke<FxEntry[]>('fx_catalog'),
  async pickFiles({ title, filters, multiple = true, directory = false }) {
    const r = await open({ title, filters, multiple, directory })
    if (r == null) return null
    return Array.isArray(r) ? r : [r]
  },
  pickSavePath: ({ title, defaultPath, filters }) => save({ title, defaultPath, filters }),
  onExternalChange(cb) {
    let stop: (() => void) | null = null
    let cancelled = false
    listen<Snapshot>('project-changed', e => cb(e.payload)).then(un => (cancelled ? un() : (stop = un)))
    return () => {
      cancelled = true
      stop?.()
    }
  },
  media,
}
