import { convertFileSrc, invoke } from '@tauri-apps/api/core'
import { listen } from '@tauri-apps/api/event'
import { message, open, save } from '@tauri-apps/plugin-dialog'
import type { Command, MediaAsset, ProjectSettings, Snapshot, Ticks } from '../core/types'
import type { Backend, ExportDone, ExportProgress, FxEntry, LiveFrame, MediaUrls, NativeEngine, PlaybackStats, ProxyStatus } from './types'

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

/** Subscribes to a backend event; returns a synchronous unsubscribe. */
function on<T>(event: string, cb: (payload: T) => void): () => void {
  let stop: (() => void) | null = null
  let cancelled = false
  listen<T>(event, e => cb(e.payload)).then(un => (cancelled ? un() : (stop = un)))
  return () => {
    cancelled = true
    stop?.()
  }
}

/** Parses a `live` response: 32-byte `JBLF` header, then RGBA pixels. */
function parseLive(buf: ArrayBuffer): LiveFrame | null {
  if (buf.byteLength < 32) return null
  const v = new DataView(buf)
  if (v.getUint32(0, false) !== 0x4a424c46) return null // "JBLF"
  const width = v.getUint32(4, true)
  const height = v.getUint32(8, true)
  const flags = v.getUint32(12, true)
  const seq = Number(v.getBigUint64(16, true))
  const time = Number(v.getBigInt64(24, true))
  if (buf.byteLength < 32 + width * height * 4) return null
  return { seq, time, width, height, playing: (flags & 1) !== 0, pixels: new Uint8ClampedArray(buf, 32, width * height * 4) }
}

const engine: NativeEngine = {
  play: (time: Ticks, rate: number) => invoke('playback_play', { time: Math.round(time), rate }),
  pause: () => invoke<Ticks>('playback_pause'),
  seek: (time: Ticks, scrub: boolean) => invoke('playback_seek', { time: Math.round(time), scrub }),
  setSize: (width, height) => invoke('playback_size', { width: Math.round(width), height: Math.round(height) }),
  setVolume: volume => invoke('playback_volume', { volume }),
  stats: () => invoke<PlaybackStats>('playback_stats'),
  report: (displayedFps, frames) => invoke('playback_report', { displayedFps, frames }),
  async nextFrame(after, signal) {
    const r = await fetch(jb('live', { after }), { signal, cache: 'no-store' })
    if (r.status !== 200) return null
    return parseLive(await r.arrayBuffer())
  },
  onEnded: cb => on<Ticks>('playback-ended', cb),
  exportCheck: () => invoke<string[]>('export_check'),
  exportStart: request => invoke('export_start', { request }),
  exportCancel: () => invoke('export_cancel'),
  onExportProgress: cb => on<ExportProgress>('export-progress', cb),
  onExportDone: cb => on<ExportDone>('export-done', cb),
  reveal: path => invoke('reveal_path', { path }),
  defaultExportDir: () => invoke<string>('default_export_dir'),
  proxyStatuses: () => invoke<ProxyStatus[]>('proxy_statuses'),
  onProxyStatus: cb => on<ProxyStatus>('proxy-status', cb),
  onAutosaved: cb => on<string>('autosaved', cb),
  onCloseRequested: cb => on<null>('close-requested', () => cb()),
  quit: () => invoke('quit_app'),
  async askUnsaved(name) {
    const r = await message(`Do you want to save the changes you made to "${name}"?\n\nYour changes will be lost if you don't save them.`, {
      title: 'Unsaved changes',
      kind: 'warning',
      buttons: { yes: 'Save', no: "Don't Save", cancel: 'Cancel' },
    })
    return r === 'Save' || r === 'Yes' ? 'save' : r === "Don't Save" || r === 'No' ? 'discard' : 'cancel'
  },
}

export const tauriBackend: Backend = {
  kind: 'tauri',
  snapshot: () => invoke<Snapshot>('get_project'),
  newProject: (name: string, settings?: Partial<ProjectSettings>) => invoke<Snapshot>('new_project', { name, settings: settings ?? null }),
  openProject: (path: string) => invoke<Snapshot>('open_project', { path }),
  async saveProject(path?: string) {
    const r = await invoke<{ snapshot: Snapshot; warnings: string[]; notes?: string[] }>('save_project', { path: path ?? null })
    return { ...r.snapshot, save_warnings: r.warnings, save_notes: r.notes ?? [] }
  },
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
  normalizeClip: (clipId, targetLufs) => invoke<Snapshot>('normalize_clip', { clipId, targetLufs }),
  analyzeStabilization: (clipId, effectId) => invoke<Snapshot>('analyze_stabilization', { clipId, effectId }),
  async saveSnapshot(png, name) {
    const bytes = new Uint8Array(await png.arrayBuffer())
    let bin = ''
    for (let i = 0; i < bytes.length; i += 0x8000) bin += String.fromCharCode(...bytes.subarray(i, i + 0x8000))
    return invoke<Snapshot>('save_snapshot', { png: btoa(bin), name })
  },
  openUrl: url => invoke('open_url', { url }),
  onExternalChange: cb => on<Snapshot>('project-changed', cb),
  media,
  engine,
}
