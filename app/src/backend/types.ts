import type { Command, MediaAsset, ProjectSettings, Snapshot } from '../core/types'

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
