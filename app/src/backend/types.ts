import type { Command, MediaAsset, ProjectSettings, Rational, Snapshot, Ticks } from '../core/types'

/** An effect, filter, transition or animation implemented by jb-render. */
export interface FxEntry {
  id: string
  name: string
  category: string
  kind: 'transition' | 'effect' | 'filter' | 'animation'
  description: string
  params: { name: string; label: string; ty: 'float' | 'int' | 'bool' | 'color' | 'vec2' | 'text'; min: number; max: number; default: [number, number, number, number]; unit: string }[]
}

export interface FileFilter {
  name: string
  extensions: string[]
}

/**
 * Everything the UI needs from the engine. The Tauri implementation calls
 * into the Rust core (`jb-core` History + command bus); the mock runs a
 * TypeScript port in the browser for development and UI tests.
 */
export interface Backend {
  readonly kind: 'tauri' | 'mock'
  snapshot(): Promise<Snapshot>
  newProject(name: string, settings?: Partial<ProjectSettings>): Promise<Snapshot>
  /** Opens a native `.jbp` project, or a Filmora `.wfp` when supported. */
  openProject(path: string): Promise<Snapshot>
  /** Saves to `path`, or to the current path when omitted. */
  saveProject(path?: string): Promise<Snapshot>
  /** Runs one command as one undo step. Rejects with the core's error message. */
  execute(cmd: Command): Promise<Snapshot>
  undo(): Promise<Snapshot>
  redo(): Promise<Snapshot>
  probeMedia(paths: string[]): Promise<MediaAsset[]>
  /** Probes files and adds them to the project as one undo step. */
  importMedia(paths: string[]): Promise<Snapshot>
  pickFiles(opts: { title: string; filters?: FileFilter[]; multiple?: boolean; directory?: boolean }): Promise<string[] | null>
  pickSavePath(opts: { title: string; defaultPath?: string; filters?: FileFilter[] }): Promise<string | null>
  /** The renderer's effect registry (drives the Transitions/Effects/Filters tabs). */
  fxCatalog(): Promise<FxEntry[]>
  /** Called when the project changes from outside this window (agents, CLI). */
  onExternalChange(cb: (s: Snapshot) => void): () => void
  media: MediaUrls
  /** The native engine (real-time preview, export, proxies); Tauri only. */
  engine?: NativeEngine
}

export interface PlaybackStats {
  rendered: number
  dropped: number
  fps: number
  render_ms: number
  audio_underruns: number
  audio_clock: boolean
  width: number
  height: number
}

export interface ExportRequest {
  path: string
  codec: 'h264' | 'hevc'
  width: number
  height: number
  frame_rate: Rational | null
  quality: 'low' | 'medium' | 'high'
  bit_rate?: number | null
  range: [Ticks, Ticks] | null
  audio: boolean
}

export interface ExportProgress {
  frame: number
  total: number
  fps: number
  elapsed: number
  eta: number
}

export interface ExportDone {
  ok: boolean
  cancelled: boolean
  error: string | null
  path: string
  encoder: string
  hardware_encoder: boolean
  width: number
  height: number
  frames: number
  fps: number
  elapsed: number
  unknown_effects: string[]
  warnings: string[]
}

export interface ProxyStatus {
  media_id: string
  path: string
  state: 'queued' | 'generating' | 'ready' | 'failed'
  progress: number
  proxy: string | null
  source: 'camera' | 'generated' | null
  error: string | null
}

/** A decoded preview frame from the `live` route. */
export interface LiveFrame {
  seq: number
  time: Ticks
  width: number
  height: number
  playing: boolean
  pixels: Uint8ClampedArray
}

export interface NativeEngine {
  play(time: Ticks, rate: number): Promise<void>
  /** Pauses and resolves with the time playback stopped at. */
  pause(): Promise<Ticks>
  seek(time: Ticks, scrub: boolean): Promise<void>
  setSize(width: number, height: number): Promise<void>
  setVolume(volume: number): Promise<void>
  stats(): Promise<PlaybackStats>
  report(displayedFps: number, frames: number): Promise<void>
  /** Waits (long poll) for a frame newer than `after`; null on timeout. */
  nextFrame(after: number, signal?: AbortSignal): Promise<LiveFrame | null>
  onEnded(cb: (t: Ticks) => void): () => void
  exportCheck(): Promise<string[]>
  exportStart(req: ExportRequest): Promise<void>
  exportCancel(): Promise<void>
  onExportProgress(cb: (p: ExportProgress) => void): () => void
  onExportDone(cb: (d: ExportDone) => void): () => void
  reveal(path: string): Promise<void>
  defaultExportDir(): Promise<string>
  proxyStatuses(): Promise<ProxyStatus[]>
  onProxyStatus(cb: (s: ProxyStatus) => void): () => void
  onAutosaved(cb: (path: string) => void): () => void
  /** The window or app is closing with unsaved changes. */
  onCloseRequested(cb: () => void): () => void
  /** Exits without asking again. */
  quit(): Promise<void>
  /** Save / Don't Save / Cancel. */
  askUnsaved(name: string): Promise<'save' | 'discard' | 'cancel'>
}

/** URLs for media served by the app (`jb://` protocol) or the dev server. */
export interface MediaUrls {
  /** A JPEG frame of a media file at `t` seconds, `h` pixels tall. */
  thumb(path: string, t: number, h: number): string
  /** JSON `{ pps, peaks }` audio peaks for a media file. */
  wave(path: string, pps: number): string
  /** The raw file, streamable by <video>/<audio>. */
  file(path: string): string
  /** A rendered frame of the main sequence at `t` seconds, `h` pixels tall. */
  frame(t: number, h: number, revision: number): string
}
