// In-browser backend for development without Tauri and for UI tests. It keeps
// an undo history of whole-project snapshots, like `jb_core::History`, and
// serves media through the Vite dev server (dev/mediaServer.ts).

import { newId } from '../core/model'
import type { Command, MediaAsset, Project, ProjectSettings, Snapshot } from '../core/types'
import { apply } from './applyCommand'
import catalog from '../fx-catalog.json'
import type { Backend, FxEntry, MediaUrls } from './types'

const defaultSettings: ProjectSettings = {
  width: 1920,
  height: 1080,
  frame_rate: { num: 30, den: 1 },
  sample_rate: 48000,
  background: [0, 0, 0, 1],
}

function emptyProject(name: string, settings?: Partial<ProjectSettings>): Project {
  const seq = newId()
  const track = (kind: 'video' | 'audio', n: number) => ({ id: newId(), kind, name: `${kind === 'video' ? 'Video' : 'Audio'} ${n}`, locked: false, muted: false, hidden: false, clips: [] })
  return {
    id: newId(),
    name,
    settings: { ...defaultSettings, ...settings },
    media: [],
    sequences: [{ id: seq, name: 'Main', tracks: [track('video', 1), track('audio', 1)] }],
    main_sequence: seq,
  }
}

class MockHistory {
  project: Project
  undoStack: Project[] = []
  redoStack: Project[] = []
  path: string | null = null
  revision = 0
  savedRevision = 0

  constructor(p: Project) {
    this.project = p
  }

  execute(cmd: Command) {
    const next = structuredClone(this.project)
    apply(next, cmd) // throws without touching the current project
    this.undoStack.push(this.project)
    if (this.undoStack.length > 200) this.undoStack.shift()
    this.redoStack = []
    this.project = next
    this.revision++
  }

  undo() {
    const prev = this.undoStack.pop()
    if (!prev) return
    this.redoStack.push(this.project)
    this.project = prev
    this.revision++
  }

  redo() {
    const next = this.redoStack.pop()
    if (!next) return
    this.undoStack.push(this.project)
    this.project = next
    this.revision++
  }

  snapshot(): Snapshot {
    return {
      project: structuredClone(this.project),
      path: this.path,
      can_undo: this.undoStack.length > 0,
      can_redo: this.redoStack.length > 0,
      dirty: this.revision !== this.savedRevision,
      revision: this.revision,
    }
  }
}

const q = (o: Record<string, string | number>) => new URLSearchParams(Object.entries(o).map(([k, v]) => [k, String(v)])).toString()

const media: MediaUrls = {
  thumb: (path, t, h) => `/__jb/thumb?${q({ path, t: t.toFixed(2), h })}`,
  wave: (path, pps) => `/__jb/wave?${q({ path, pps })}`,
  file: path => `/__jb/file?${q({ path })}`,
  frame: () => '',
}

export function createMockBackend(): Backend {
  let h = new MockHistory(emptyProject('Untitled'))
  const listeners = new Set<(s: Snapshot) => void>()
  const files = new Map<string, string>() // fake file system for save/open
  const pickQueue: (string[] | null)[] = []

  const ok = () => Promise.resolve(h.snapshot())
  const probe = async (paths: string[]): Promise<MediaAsset[]> =>
    Promise.all(
      paths.map(async path => {
        const r = await fetch(`/__jb/probe?${q({ path })}`)
        if (!r.ok) throw new Error(`cannot probe ${path}`)
        return (await r.json()) as MediaAsset
      }),
    )

  const backend: Backend = {
    kind: 'mock',
    snapshot: ok,
    async newProject(name, settings) {
      h = new MockHistory(emptyProject(name, settings))
      return h.snapshot()
    },
    async openProject(path) {
      const text = files.get(path)
      if (!text) throw new Error(`cannot open ${path}`)
      h = new MockHistory(JSON.parse(text).project)
      h.path = path
      return h.snapshot()
    },
    async saveProject(path) {
      const target = path ?? h.path
      if (!target) throw new Error('no path to save to')
      files.set(target, JSON.stringify({ format: 'jibaru-project', version: 1, project: h.project }))
      h.path = target
      h.savedRevision = h.revision
      return h.snapshot()
    },
    async execute(cmd) {
      try {
        h.execute(cmd)
      } catch (e) {
        return Promise.reject((e as Error).message)
      }
      return h.snapshot()
    },
    async undo() {
      h.undo()
      return h.snapshot()
    },
    async redo() {
      h.redo()
      return h.snapshot()
    },
    probeMedia: probe,
    // Snapshot of `jibaru-editor --fx-catalog`; the app asks jb-render directly.
    fxCatalog: async () => catalog as FxEntry[],
    async importMedia(paths) {
      const known = new Set(h.project.media.map(m => m.path))
      const assets = (await probe(paths.filter(p => !known.has(p)))).filter(a => !h.project.media.some(m => m.id === a.id))
      if (assets.length) h.execute({ op: 'batch', commands: assets.map(asset => ({ op: 'add_media', asset })) })
      return h.snapshot()
    },
    async pickFiles({ title }) {
      if (pickQueue.length) return pickQueue.shift()!
      const answer = window.prompt(`${title}\nAbsolute paths, separated by ";"`)
      return answer ? answer.split(';').map(s => s.trim()).filter(Boolean) : null
    },
    async pickSavePath({ title, defaultPath }) {
      if (pickQueue.length) return pickQueue.shift()?.[0] ?? null
      return window.prompt(title, defaultPath ?? '/tmp/Untitled.jbp')
    },
    onExternalChange(cb) {
      listeners.add(cb)
      return () => listeners.delete(cb)
    },
    media,
  }

  // Test hooks: load a project, queue dialog answers, simulate an agent edit.
  ;(window as unknown as { __jbMock: unknown }).__jbMock = {
    setProject(p: Project) {
      h = new MockHistory(structuredClone(p))
      listeners.forEach(l => l(h.snapshot()))
    },
    project: () => structuredClone(h.project),
    queuePick: (paths: string[] | null) => pickQueue.push(paths),
    external(cmd: Command) {
      h.execute(cmd)
      listeners.forEach(l => l(h.snapshot()))
    },
    probe,
  }

  return backend
}
