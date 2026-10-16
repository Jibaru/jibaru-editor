// Library content. Transitions, effects, filters and animations come from the
// renderer's registry (`jb-render`), so every item shown is one it can draw.

import type { FxEntry, PreviewRef } from './backend'
import { fits, frameTicks, mainSequence, newId } from './core/model'
import { TICKS_PER_SECOND, type Clip, type Command, type TitleData } from './core/types'
import { targetClips } from './edit'
import { exec, getState, project, select, toast, type LibraryTab, type PreviewTarget } from './store'

export interface LibraryItem {
  type: 'media' | 'title' | 'transition' | 'effect'
  id: string
  label: string
  /** Library sidebar groups listing the item, with its position in each. */
  groups?: { group: string; order: number }[]
  sample?: string
}

/** Title templates, under Titles > Title Templates. */
export const titleItems: LibraryItem[] = [
  { type: 'title', id: 'default', label: 'Default Title', groups: [{ group: 'Default', order: 0 }] },
  { type: 'title', id: 'basic', label: 'Basic Title', groups: [{ group: 'Default', order: 1 }] },
  { type: 'title', id: 'lower_third', label: 'Lower Third', groups: [{ group: 'Lower Thirds', order: 0 }] },
  { type: 'title', id: 'subtitle', label: 'Subtitle', groups: [{ group: 'Subtitles', order: 0 }] },
]

const TAB_KIND: Partial<Record<LibraryTab, FxEntry['kind']>> = { transitions: 'transition', effects: 'effect', filters: 'filter' }

/**
 * Every item of a library tab: the registry items listed in one of its
 * groups (jb_render::library; properties-panel effects are in none).
 */
export function tabItems(tab: LibraryTab, fx: FxEntry[]): LibraryItem[] {
  if (tab === 'titles') return titleItems
  const kind = TAB_KIND[tab]
  if (!kind) return []
  return fx
    .filter(f => f.kind === kind && f.groups?.length)
    .map(f => ({ type: kind === 'transition' ? 'transition' : 'effect', id: f.id, label: f.name, groups: f.groups, sample: kind }))
}

/** Sidebar groups of a tab, in Filmora's order. */
export function tabGroups(tab: LibraryTab, fx: FxEntry[]): string[] {
  if (tab === 'titles') return [...new Set(titleItems.flatMap(i => (i.groups ?? []).map(g => g.group)))]
  const kind = TAB_KIND[tab]
  const index = new Map<string, number>()
  for (const f of fx) if (f.kind === kind) for (const g of f.groups ?? []) index.set(g.group, g.index)
  return [...index.keys()].sort((a, b) => index.get(a)! - index.get(b)!)
}

/** The items of one group, in display order. */
export function groupItems(tab: LibraryTab, fx: FxEntry[], group: string): LibraryItem[] {
  const at = (i: LibraryItem) => i.groups?.find(g => g.group === group)?.order ?? -1
  return tabItems(tab, fx)
    .filter(i => at(i) >= 0)
    .sort((a, b) => at(a) - at(b))
}

/** Entry (in) vs exit (out) animations, following the registry descriptions. */
export function animationsFor(fx: FxEntry[], dir: 'in' | 'out'): FxEntry[] {
  return fx.filter(f => f.kind === 'animation').filter(f => {
    const out = f.description.startsWith('Out:') || f.id.endsWith('_out')
    return dir === 'out' ? out : !out
  })
}

export function catalogLabel(kind: string): string {
  return getState().fx.find(f => f.id === kind)?.name ?? kind.replace(/^(jb|filmora)[.:]/, '').replace(/[._]/g, ' ')
}

// ---------- Previews (card thumbnails, hover animations, player) ----------

export function previewRef(item: LibraryItem): PreviewRef {
  return item.type === 'title' ? { title: titleData(item.id, item.label) } : { fx: item.id }
}

export function previewTarget(item: LibraryItem): PreviewTarget {
  const kind: PreviewTarget['kind'] =
    item.type === 'title' ? 'title' : item.type === 'transition' ? 'transition' : item.sample === 'filter' ? 'filter' : item.sample === 'animation' ? 'animation' : 'effect'
  return { key: favKey(item), label: item.label, kind, ref: previewRef(item) }
}

// ---------- Favorites (per user) ----------

const FAV_KEY = 'jb.favorites'

export function favorites(): Set<string> {
  try {
    return new Set(JSON.parse(localStorage.getItem(FAV_KEY) ?? '[]') as string[])
  } catch {
    return new Set()
  }
}

export const favKey = (i: LibraryItem) => `${i.type}:${i.id}`

export function toggleFavorite(i: LibraryItem) {
  const f = favorites()
  const k = favKey(i)
  if (f.has(k)) f.delete(k)
  else f.add(k)
  try {
    localStorage.setItem(FAV_KEY, JSON.stringify([...f]))
  } catch {
    toast('Favorites cannot be saved (storage unavailable)', 'error')
  }
  window.dispatchEvent(new Event('jb-favorites'))
}

// ---------- Applying an item without dragging ("+" button, double-click) ----------

const TITLE_BASE: TitleData = { text: 'Title', font_family: 'Helvetica Neue', font_size: 0.09, color: [1, 1, 1, 1], position: [0.5, 0.5], bold: true }

export function titleData(id: string, label: string): TitleData {
  const t = { ...TITLE_BASE, text: id === 'default' ? 'Title' : label }
  if (id === 'lower_third') return { ...t, position: [0.3, 0.82], font_size: 0.06 }
  if (id === 'subtitle') return { ...t, position: [0.5, 0.9], font_size: 0.05, bold: false }
  return t
}

/** Puts a title at the playhead, an effect on the selected clips, a transition on the nearest cut. */
export async function applyLibraryItem(item: LibraryItem) {
  const p = project()
  if (!p) return
  const seq = mainSequence(p)
  const at = getState().playhead
  if (item.type === 'title') {
    const len = 5 * TICKS_PER_SECOND
    const clip: Clip = { id: newId(), name: item.label, source: { type: 'title', ...titleData(item.id, item.label) }, start: at, end: at + len }
    const videos = seq.tracks.filter(t => t.kind === 'video')
    let track = videos.slice(1).find(t => !t.locked && fits(t, at, at + len))
    if (!track) {
      const before = new Set(seq.tracks.map(t => t.id))
      if (!(await exec({ op: 'add_track', sequence_id: null, kind: 'video', index: videos.length }))) return
      track = mainSequence(project()!).tracks.find(t => !before.has(t.id))
    }
    if (track && (await exec({ op: 'add_clip', track_id: track.id, clip }))) select([clip.id])
    return
  }
  if (item.type === 'effect') {
    const clips = targetClips('video')
    if (!clips.length) return toast('Select a video clip (or move the playhead over one) to apply it')
    const commands: Command[] = clips.map(c => ({ op: 'add_effect', clip_id: c.id, effect: { id: newId(), kind: item.id, enabled: true, params: {} } }))
    if (await exec({ op: 'batch', commands })) select(clips.map(c => c.id))
    return
  }
  if (item.type === 'transition') {
    // The cut nearest to the playhead between two touching clips (the selected clip's track first).
    const frame = frameTicks(p.settings.frame_rate)
    const sel = getState().selection
    const has = (t: (typeof seq.tracks)[number]) => Number(t.clips.some(c => sel.includes(c.id)))
    const tracks = seq.tracks.filter(t => t.kind === 'video').sort((a, b) => has(b) - has(a))
    let best: Clip | null = null
    for (const t of tracks) {
      for (let i = 0; i + 1 < t.clips.length; i++) {
        const a = t.clips[i]
        if (t.clips[i + 1].start - a.end > frame) continue
        if (!best || Math.abs(a.end - at) < Math.abs(best.end - at)) best = a
      }
      if (best) break
    }
    if (!best) return toast('Place two clips next to each other to add a transition')
    const dur = Math.min(TICKS_PER_SECOND, Math.floor((best.end - best.start) / 2))
    if (await exec({ op: 'set_transition', clip_id: best.id, transition: { id: newId(), kind: item.id, duration: dur, alignment: 'center', params: {} } })) select([best.id])
  }
}
