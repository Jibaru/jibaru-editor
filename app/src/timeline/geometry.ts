// Layout constants and pure helpers for the timeline, kept apart from React
// so drag logic can be reasoned about (and tested) on its own.

import { TICKS_PER_SECOND, type Clip, type Command, type Sequence, type Ticks, type Track } from '../core/types'

export const HEADER_W = 114
export const RULER_H = 30
export const TOP_GAP = 48
export const BOTTOM_GAP = 40
export const VIDEO_H = 70
export const AUDIO_H = 45
export const ROW_GAP = 0
export const SNAP_PX = 8

export const trackHeight = (t: Track, compact = false) => (compact ? (t.kind === 'video' ? 48 : 32) : (t.height ?? (t.kind === 'video' ? VIDEO_H : AUDIO_H)))

export const tx = (t: Ticks, pps: number) => (t / TICKS_PER_SECOND) * pps
export const xt = (x: number, pps: number) => Math.round((x / pps) * TICKS_PER_SECOND)

export interface Row {
  track: Track
  top: number
  height: number
}

/** Rows in display order with their y offsets inside the scroll content. */
export function layoutRows(tracks: Track[], compact = false): { rows: Row[]; height: number } {
  let y = RULER_H + 2 + TOP_GAP
  const rows = tracks.map(track => {
    const r = { track, top: y, height: trackHeight(track, compact) }
    y += r.height + ROW_GAP
    return r
  })
  return { rows, height: y + BOTTOM_GAP }
}

/** Candidate times to snap to: clip edges and the playhead. */
export function snapPoints(seq: Sequence, exclude: Set<string>, playhead: Ticks): Ticks[] {
  const pts = [0, playhead]
  for (const t of seq.tracks) for (const c of t.clips) if (!exclude.has(c.id)) pts.push(c.start, c.end)
  return pts
}

/** Returns the snapped value (and the snap target) if any candidate is within `px`. */
export function snap(values: Ticks[], points: Ticks[], pps: number, px = SNAP_PX): { delta: Ticks; at: Ticks } | null {
  const limit = xt(px, pps)
  let best: { delta: Ticks; at: Ticks } | null = null
  for (const v of values)
    for (const p of points) {
      const d = p - v
      if (Math.abs(d) <= limit && (!best || Math.abs(d) < Math.abs(best.delta))) best = { delta: d, at: p }
    }
  return best
}

export function overlaps(track: Track, start: Ticks, end: Ticks, ignore: Set<string>): Clip | null {
  return track.clips.find(c => !ignore.has(c.id) && c.start < end && c.end > start) ?? null
}

/**
 * Commands that insert `clip` into `track` at `at`, pushing later clips right
 * (Filmora's insert drop). If the clip currently lives on the same track, its
 * old slot is closed first. Returned as one batch so it is one undo step.
 */
export function insertCommands(track: Track, clip: Clip, fromTrack: Track, at: Ticks): Command[] {
  const len = clip.end - clip.start
  const cmds: Command[] = []
  const farAway = Math.max(...track.clips.map(c => c.end), ...fromTrack.clips.map(c => c.end), at) + len * 4 + TICKS_PER_SECOND * 3600
  // 1. Park the clip out of the way (on its own track).
  cmds.push({ op: 'move_clip', clip_id: clip.id, start: farAway, track_id: null })
  let target = at
  const sameTrack = track.id === fromTrack.id
  if (sameTrack) {
    // 2. Close the gap it left.
    const after = track.clips.filter(c => c.id !== clip.id && c.start >= clip.end).sort((a, b) => a.start - b.start)
    for (const c of after) cmds.push({ op: 'move_clip', clip_id: c.id, start: c.start - len, track_id: null })
    if (at >= clip.end) target = at - len
  }
  // 3. Push clips at or after the insert point to the right (right-most first).
  const shifted = track.clips
    .filter(c => c.id !== clip.id)
    .map(c => (sameTrack && c.start >= clip.end ? { ...c, start: c.start - len, end: c.end - len } : c))
    .filter(c => c.start >= target)
    .sort((a, b) => b.start - a.start)
  for (const c of shifted) cmds.push({ op: 'move_clip', clip_id: c.id, start: c.start + len, track_id: null })
  // 4. Drop the clip into the hole.
  cmds.push({ op: 'move_clip', clip_id: clip.id, start: target, track_id: sameTrack ? null : track.id })
  return cmds
}

/**
 * Ruler label step and subdivisions, picked like the reference: the smallest
 * step from a fixed list whose labels are at least ~95 px apart.
 */
export function rulerStep(pps: number, fps = 30): { major: number; subs: number } {
  const f = 1 / Math.max(1, Math.round(fps))
  const steps: [number, number][] = [
    [f, 1],
    [5 * f, 5],
    [1, 6],
    [2, 6],
    [5, 5],
    [10, 5],
    [30, 6],
    [60, 6],
    [120, 4],
    [300, 5],
    [600, 5],
    [1800, 6],
    [3600, 6],
  ]
  const [major, subs] = steps.find(([s]) => s * pps >= 95) ?? steps[steps.length - 1]
  return { major, subs }
}

/** MM:SS (HH:MM:SS past an hour) on whole seconds, `Nf` between them. */
export function rulerLabel(sec: number, fps: number): string {
  const p = (n: number) => String(n).padStart(2, '0')
  const rate = Math.max(1, Math.round(fps))
  const frames = Math.round(sec * rate)
  const s = Math.floor(frames / rate)
  const fr = frames - s * rate
  if (fr) return `${fr}f`
  const h = Math.floor(s / 3600)
  const m = Math.floor(s / 60) % 60
  return h > 0 ? `${p(h)}:${p(m)}:${p(s % 60)}` : `${p(m)}:${p(s % 60)}`
}

const filmoraIndex = (t: Track): number | null => {
  const i = (t.extra as { filmora?: { index?: unknown } } | undefined)?.filmora?.index
  return typeof i === 'number' ? i : null
}

/**
 * Tracks shown on the timeline, top to bottom. Camera audio (audio clips
 * linked to a video clip with the same span) is drawn inside the video clip,
 * so its track is not listed, like the reference; so is the empty audio slot
 * Filmora keeps under each of its video tracks.
 */
export function timelineTracks(seq: Sequence): Track[] {
  const video = seq.tracks.filter(t => t.kind === 'video')
  const spans = new Set(video.flatMap(t => t.clips.filter(c => c.link_group).map(c => `${c.link_group}:${c.start}:${c.end}`)))
  const videoSlots = new Set(video.map(filmoraIndex).filter((i): i is number => i != null))
  const embedded = (t: Track) => {
    if (t.kind !== 'audio') return false
    if (t.clips.length) return t.clips.every(c => !!c.link_group && spans.has(`${c.link_group}:${c.start}:${c.end}`))
    const i = filmoraIndex(t)
    return i != null && videoSlots.has(i + 1)
  }
  return [...video.slice().reverse(), ...seq.tracks.filter(t => t.kind === 'audio' && !embedded(t))]
}
