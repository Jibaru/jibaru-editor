// UI state and editing actions. The project itself lives in the backend (the
// Rust core); this store only keeps the latest snapshot plus view state such
// as selection, playhead and zoom. Every edit goes through `exec`, which
// sends one `Command` to the backend: the same API agents and the CLI use.

import { useSyncExternalStore } from 'react'
import { backend } from './backend'
import {
  clipFromMedia,
  findClip,
  findTrack,
  fits,
  frameTicks,
  mainSequence,
  newId,
  sequenceDuration,
  snapToFrame,
  ticks,
} from './core/model'
import type { Clip, Command, MediaAsset, Project, Snapshot, Ticks, Track } from './core/types'
import type { MenuItem } from './ui/ContextMenu'

export type LibraryTab = 'media' | 'stock' | 'audio' | 'titles' | 'transitions' | 'effects' | 'filters' | 'stickers' | 'templates'
export type PropsTab = 'video' | 'audio' | 'speed' | 'animation' | 'color'

export interface EditorState {
  snap: Snapshot | null
  selection: string[]
  selectedMedia: string[]
  playhead: Ticks
  playing: boolean
  /** Playback rate; negative plays backwards (J/K/L shuttle). */
  rate: number
  playerMode: 'timeline' | 'source'
  sourceMedia: string | null
  sourceTime: Ticks
  libraryTab: LibraryTab
  propsTab: PropsTab
  /** Timeline zoom in pixels per second. */
  zoom: number
  snapping: boolean
  clipboard: { trackId: string; clip: Clip }[]
  toast: { kind: 'info' | 'error'; text: string; id: number } | null
  menu: { x: number; y: number; items: MenuItem[]; bar?: string } | null
  exportOpen: boolean
  volume: number
}

type Listener = () => void

let state: EditorState = {
  snap: null,
  selection: [],
  selectedMedia: [],
  playhead: 0,
  playing: false,
  rate: 1,
  playerMode: 'timeline',
  sourceMedia: null,
  sourceTime: 0,
  libraryTab: 'media',
  propsTab: 'video',
  zoom: 8,
  snapping: true,
  clipboard: [],
  toast: null,
  menu: null,
  exportOpen: false,
  volume: 1,
}
const listeners = new Set<Listener>()

export const getState = () => state

export function setState(patch: Partial<EditorState> | ((s: EditorState) => Partial<EditorState>)) {
  const p = typeof patch === 'function' ? patch(state) : patch
  state = { ...state, ...p }
  listeners.forEach(l => l())
}

export function subscribe(l: Listener) {
  listeners.add(l)
  return () => {
    listeners.delete(l)
  }
}

export function useEditor<T>(selector: (s: EditorState) => T): T {
  return useSyncExternalStore(subscribe, () => selector(state))
}

export const project = (): Project | null => state.snap?.project ?? null

// ---------- Backend plumbing ----------

let toastId = 0
export function toast(text: string, kind: 'info' | 'error' = 'info') {
  const id = ++toastId
  setState({ toast: { kind, text, id } })
  setTimeout(() => {
    if (state.toast?.id === id) setState({ toast: null })
  }, 3200)
}

function applySnapshot(snap: Snapshot) {
  const p = snap.project
  // Drop selections that no longer exist.
  const selection = state.selection.filter(id => findClip(p, id))
  const selectedMedia = state.selectedMedia.filter(id => p.media.some(m => m.id === id))
  setState({ snap, selection, selectedMedia })
}

/** Runs one command on the backend. Returns false (and shows the error) on failure. */
export async function exec(cmd: Command): Promise<boolean> {
  try {
    applySnapshot(await backend.execute(cmd))
    return true
  } catch (e) {
    toast(String(e), 'error')
    return false
  }
}

export async function load() {
  applySnapshot(await backend.snapshot())
  backend.onExternalChange(applySnapshot)
}

export async function undo() {
  if (!state.snap?.can_undo) return
  applySnapshot(await backend.undo())
}

export async function redo() {
  if (!state.snap?.can_redo) return
  applySnapshot(await backend.redo())
}

const projectFilters = [
  { name: 'Projects', extensions: ['jbp', 'wfp'] },
  { name: 'Jibaru project', extensions: ['jbp'] },
  { name: 'Filmora project', extensions: ['wfp'] },
]

export async function newProject() {
  applySnapshot(await backend.newProject('Untitled'))
  setState({ selection: [], playhead: 0, playing: false })
}

export async function openProject(path?: string) {
  const p = path ?? (await backend.pickFiles({ title: 'Open Project', filters: projectFilters, multiple: false }))?.[0]
  if (!p) return
  try {
    applySnapshot(await backend.openProject(p))
    setState({ selection: [], playhead: 0, playing: false })
  } catch (e) {
    toast(String(e), 'error')
  }
}

export async function saveProject(as = false) {
  const snap = state.snap
  if (!snap) return
  let path = snap.path && !as && snap.path.endsWith('.jbp') ? snap.path : null
  if (!path) {
    path = await backend.pickSavePath({ title: 'Save Project', defaultPath: `${snap.project.name}.jbp`, filters: [{ name: 'Jibaru project', extensions: ['jbp'] }] })
    if (!path) return
  }
  try {
    applySnapshot(await backend.saveProject(path))
    toast('Project saved')
  } catch (e) {
    toast(String(e), 'error')
  }
}

const mediaFilters = [
  {
    name: 'Media',
    extensions: ['mp4', 'mov', 'm4v', 'mkv', 'webm', 'avi', 'mts', 'mp3', 'wav', 'm4a', 'aac', 'flac', 'ogg', 'jpg', 'jpeg', 'png', 'heic', 'webp', 'gif'],
  },
]

export async function importMedia(paths?: string[]) {
  const files = paths ?? (await backend.pickFiles({ title: 'Import Media', filters: mediaFilters, multiple: true }))
  if (!files?.length) return
  try {
    applySnapshot(await backend.importMedia(files))
  } catch (e) {
    toast(String(e), 'error')
  }
}

// ---------- Selection ----------

export function select(ids: string[], mode: 'replace' | 'toggle' | 'add' = 'replace') {
  setState(s => {
    if (mode === 'replace') return { selection: ids, selectedMedia: [] }
    const set = new Set(s.selection)
    for (const id of ids) {
      if (mode === 'toggle' && set.has(id)) set.delete(id)
      else set.add(id)
    }
    return { selection: [...set], selectedMedia: [] }
  })
}

export function selectAll() {
  const p = project()
  if (!p) return
  select(mainSequence(p).tracks.flatMap(t => t.clips.map(c => c.id)))
}

// ---------- Transport ----------

export function duration(): Ticks {
  const p = project()
  return p ? sequenceDuration(mainSequence(p)) : 0
}

export function seek(t: Ticks) {
  const p = project()
  if (!p) return
  const max = Math.max(duration(), 0)
  setState({ playhead: Math.max(0, Math.min(snapToFrame(t, p.settings.frame_rate), max)), playerMode: 'timeline' })
}

export function stepFrames(n: number) {
  const p = project()
  if (!p) return
  setState({ playing: false })
  seek(state.playhead + n * frameTicks(p.settings.frame_rate) + 1)
}

export function togglePlay() {
  if (state.playing) return setState({ playing: false })
  if (state.playhead >= duration() - 1) seek(0)
  setState({ playing: true, rate: 1 })
}

export function stop() {
  setState({ playing: false })
  seek(0)
}

/** J/K/L shuttle like Filmora: J reverse, K pause, L forward; repeated presses speed up. */
export function shuttle(key: 'j' | 'k' | 'l') {
  if (key === 'k') return setState({ playing: false, rate: 1 })
  const dir = key === 'l' ? 1 : -1
  const r = state.playing && Math.sign(state.rate) === dir ? Math.min(Math.abs(state.rate) * 2, 8) * dir : dir
  setState({ playing: true, rate: r })
}

let lastFrame = 0
function tick(now: number) {
  if (state.playing) {
    const dt = lastFrame ? now - lastFrame : 0
    const next = state.playhead + ticks((dt / 1000) * state.rate)
    const end = duration()
    if (next >= end) setState({ playhead: end, playing: false })
    else if (next <= 0) setState({ playhead: 0, playing: false })
    else setState({ playhead: next })
  }
  lastFrame = now
  requestAnimationFrame(tick)
}
requestAnimationFrame(tick)

// ---------- Editing ----------

function editableTrack(p: Project, trackId: string): Track | null {
  const t = findTrack(p, trackId)
  return t && !t.locked ? t : null
}

/** Clips the split/delete actions apply to: the selection, or clips under the playhead. */
function targetsAtPlayhead(p: Project): Clip[] {
  const at = state.playhead
  const sel = state.selection.map(id => findClip(p, id)?.clip).filter((c): c is Clip => !!c)
  const pool = sel.length ? sel : mainSequence(p).tracks.filter(t => !t.locked && !t.hidden).flatMap(t => t.clips)
  return pool.filter(c => at > c.start && at < c.end)
}

export async function splitAtPlayhead() {
  const p = project()
  if (!p) return
  const targets = targetsAtPlayhead(p)
  if (!targets.length) return toast('Move the playhead over a clip to split it')
  // Linked clips split together in the core; split each group only once.
  const seen = new Set<string>()
  const commands: Command[] = []
  for (const c of targets) {
    const key = c.link_group ?? c.id
    if (seen.has(key)) continue
    seen.add(key)
    commands.push({ op: 'split_clip', clip_id: c.id, at: state.playhead })
  }
  await exec(commands.length === 1 ? commands[0] : { op: 'batch', commands })
}

export async function deleteSelection(ripple = false) {
  const p = project()
  if (!p) return
  if (state.selectedMedia.length && !state.selection.length) {
    const ok = await exec({ op: 'batch', commands: state.selectedMedia.map(media_id => ({ op: 'remove_media', media_id })) })
    if (ok) setState({ selectedMedia: [] })
    return
  }
  if (!state.selection.length) return
  // Ripple deletes from right to left so earlier gaps don't shift later targets.
  const clips = state.selection
    .map(id => findClip(p, id)?.clip)
    .filter((c): c is Clip => !!c)
    .sort((a, b) => b.start - a.start)
  const seen = new Set<string>()
  const commands: Command[] = []
  for (const c of clips) {
    const key = c.link_group ?? c.id
    if (seen.has(key)) continue
    seen.add(key)
    commands.push({ op: 'remove_clip', clip_id: c.id, ripple })
  }
  if (await exec({ op: 'batch', commands })) setState({ selection: [] })
}

export function copySelection() {
  const p = project()
  if (!p || !state.selection.length) return
  const items = state.selection
    .map(id => findClip(p, id))
    .filter((x): x is { track: Track; clip: Clip } => !!x)
    .map(({ track, clip }) => ({ trackId: track.id, clip: structuredClone(clip) }))
  setState({ clipboard: items })
}

export async function cutSelection() {
  copySelection()
  await deleteSelection(false)
}

/** Pastes the clipboard at the playhead, keeping relative offsets and tracks. */
export async function paste() {
  const p = project()
  if (!p || !state.clipboard.length) return
  const base = Math.min(...state.clipboard.map(c => c.clip.start))
  const groups = new Map<string, string>()
  const commands: Command[] = []
  const placed: Record<string, { start: Ticks; end: Ticks }[]> = {}
  const ids: string[] = []
  for (const { trackId, clip } of state.clipboard) {
    const track = editableTrack(p, trackId) ?? mainSequence(p).tracks.find(t => t.kind === findTrack(p, trackId)?.kind && !t.locked)
    if (!track) continue
    const len = clip.end - clip.start
    let start = state.playhead + (clip.start - base)
    // Find the first free slot at or after the target time.
    const busy = [...track.clips.map(c => ({ start: c.start, end: c.end })), ...(placed[track.id] ?? [])].sort((a, b) => a.start - b.start)
    for (const b of busy) if (start < b.end && start + len > b.start) start = b.end
    const id = newId()
    ids.push(id)
    const link = clip.link_group ? (groups.get(clip.link_group) ?? groups.set(clip.link_group, newId()).get(clip.link_group)) : undefined
    const next: Clip = { ...structuredClone(clip), id, start, end: start + len, link_group: link }
    if (!link) delete next.link_group
    ;(placed[track.id] ??= []).push({ start, end: start + len })
    commands.push({ op: 'add_clip', track_id: track.id, clip: next })
  }
  if (commands.length && (await exec({ op: 'batch', commands }))) select(ids)
}

export async function duplicateSelection() {
  copySelection()
  const p = project()
  if (!p || !state.clipboard.length) return
  const end = Math.max(...state.clipboard.map(c => c.clip.end))
  const saved = state.playhead
  setState({ playhead: end })
  await paste()
  setState({ playhead: saved })
}

/** Adds media to the timeline at `start` on `trackId` (or the first fitting track). */
export async function addMediaToTimeline(m: MediaAsset, start: Ticks, trackId?: string) {
  const p = project()
  if (!p) return
  const seq = mainSequence(p)
  const kind = m.kind === 'audio' ? 'audio' : 'video'
  const clip = clipFromMedia(m, Math.max(0, start))
  const target = trackId ? findTrack(p, trackId) : null
  let trackForClip: string | null = null
  if (target && target.kind === kind && !target.locked && fits(target, clip.start, clip.end)) trackForClip = target.id
  else trackForClip = seq.tracks.find(t => t.kind === kind && !t.locked && fits(t, clip.start, clip.end))?.id ?? null
  if (!trackForClip) {
    // No room: put it on a new track, like Filmora does when dropping over clips.
    // Track ids are created by the core, so this is a separate undo step.
    const before = new Set(seq.tracks.map(t => t.id))
    if (!(await exec({ op: 'add_track', sequence_id: null, kind, index: kind === 'video' ? seq.tracks.filter(t => t.kind === 'video').length : null }))) return
    trackForClip = mainSequence(project()!).tracks.find(t => !before.has(t.id))?.id ?? null
    if (!trackForClip) return
  }
  if (await exec({ op: 'add_clip', track_id: trackForClip, clip })) select([clip.id])
}

/** Appends media at the end of the main video/audio track (the "+" on library cards). */
export async function appendMedia(m: MediaAsset) {
  const p = project()
  if (!p) return
  const kind = m.kind === 'audio' ? 'audio' : 'video'
  const track = mainSequence(p).tracks.find(t => t.kind === kind && !t.locked)
  const end = track ? Math.max(0, ...track.clips.map(c => c.end)) : 0
  await addMediaToTimeline(m, end, track?.id)
}

export async function addTrack(kind: 'video' | 'audio') {
  const p = project()
  if (!p) return
  const seq = mainSequence(p)
  const index = kind === 'video' ? seq.tracks.filter(t => t.kind === 'video').length : null
  await exec({ op: 'add_track', sequence_id: null, kind, index })
}

export const clamp = (v: number, a: number, b: number) => Math.min(b, Math.max(a, v))

// ---------- Timeline zoom ----------

export const MIN_ZOOM = 0.05
export const MAX_ZOOM = 400
let timelineViewWidth = 800
export function setTimelineViewWidth(w: number) {
  timelineViewWidth = w
}
export function zoomBy(f: number) {
  setState(s => ({ zoom: clamp(s.zoom * f, MIN_ZOOM, MAX_ZOOM) }))
}
export function zoomToFit() {
  const d = duration() / 10_000_000
  setState({ zoom: clamp(d > 0 ? (timelineViewWidth - 60) / d : 8, MIN_ZOOM, MAX_ZOOM) })
}

export function openMenu(x: number, y: number, items: MenuItem[], bar?: string) {
  setState({ menu: { x, y, items, bar } })
}
