// Layout constants and pure helpers for the timeline, kept apart from React
// so drag logic can be reasoned about (and tested) on its own.

import { TICKS_PER_SECOND, type Clip, type Command, type Sequence, type Ticks, type Track } from '../core/types'

export const HEADER_W = 114
export const RULER_H = 30
export const TOP_GAP = 48
export const BOTTOM_GAP = 40
export const VIDEO_H = 68
export const AUDIO_H = 44
export const ROW_GAP = 2
export const SNAP_PX = 8

export const trackHeight = (t: Track) => (t.kind === 'video' ? VIDEO_H : AUDIO_H)

export const tx = (t: Ticks, pps: number) => (t / TICKS_PER_SECOND) * pps
export const xt = (x: number, pps: number) => Math.round((x / pps) * TICKS_PER_SECOND)

export interface Row {
  track: Track
  top: number
  height: number
}

/** Rows in display order with their y offsets inside the scroll content. */
export function layoutRows(tracks: Track[]): { rows: Row[]; height: number } {
  let y = RULER_H + 2 + TOP_GAP
  const rows = tracks.map(track => {
    const r = { track, top: y, height: trackHeight(track) }
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

export function rulerStep(pps: number): { major: number; minor: number } {
  // Seconds between labels, chosen so labels are ~150-250 px apart.
  const steps = [1 / 30, 1 / 10, 0.5, 1, 2, 5, 10, 15, 30, 60, 120, 150, 300, 600, 900, 1800, 3600]
  const major = steps.find(s => s * pps >= 150) ?? 3600
  const minor = major / 5
  return { major, minor }
}

export function rulerLabel(sec: number, major: number, fps: number): string {
  const p = (n: number) => String(n).padStart(2, '0')
  const s = Math.floor(sec + 1e-6)
  const h = Math.floor(s / 3600)
  const m = Math.floor(s / 60) % 60
  const base = h > 0 ? `${p(h)}:${p(m)}:${p(s % 60)}` : `${p(m)}:${p(s % 60)}`
  if (major < 1) return `${base}:${p(Math.round((sec - s) * fps))}`
  return base
}
