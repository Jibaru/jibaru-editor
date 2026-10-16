// Helpers over the jb-core model: defaults, lookups and time math.

import {
  TICKS_PER_SECOND,
  type AudioSettings,
  type Clip,
  type MediaAsset,
  type Param,
  type Project,
  type Rational,
  type Sequence,
  type Ticks,
  type Track,
  type Transform,
} from './types'

export const float = (v: number): Param => ({ value: { type: 'float', value: v } })

export const paramValue = (p: Param | undefined, fallback = 0): number =>
  p && p.value.type === 'float' ? p.value.value : fallback

export const defaultTransform = (): Transform => ({
  position_x: float(0),
  position_y: float(0),
  scale_x: float(1),
  scale_y: float(1),
  rotation: float(0),
  flip_h: false,
  flip_v: false,
  crop: [0, 0, 0, 0],
  fit: 'fit',
})

export const defaultAudio = (): AudioSettings => ({
  gain_db: float(0),
  pan: float(0),
  fade_in: 0,
  fade_out: 0,
  pitch: 0,
  muted: false,
})

export function newId(): string {
  return crypto.randomUUID()
}

export const seconds = (t: Ticks) => t / TICKS_PER_SECOND
export const ticks = (s: number) => Math.round(s * TICKS_PER_SECOND)

export const fps = (r: Rational) => r.num / r.den

/** Duration of one frame in ticks (rounded like jb-core). */
export const frameTicks = (r: Rational) => Math.round((TICKS_PER_SECOND * r.den) / r.num)

/** Snaps a time to the start of the frame that contains it. */
export function snapToFrame(t: Ticks, r: Rational): Ticks {
  const frame = Math.floor((t * r.num) / (r.den * TICKS_PER_SECOND) + 1e-9)
  return Math.ceil((frame * r.den * TICKS_PER_SECOND) / r.num)
}

/** HH:MM:SS:FF like Filmora's player and timeline. */
export function timecode(t: Ticks, r: Rational): string {
  const rate = Math.round(fps(r)) || 30
  const totalFrames = Math.floor((Math.max(0, t) * r.num) / (r.den * TICKS_PER_SECOND) + 1e-6)
  const ff = totalFrames % rate
  const totalSeconds = Math.floor(totalFrames / rate)
  const p = (n: number) => String(n).padStart(2, '0')
  return `${p(Math.floor(totalSeconds / 3600))}:${p(Math.floor(totalSeconds / 60) % 60)}:${p(totalSeconds % 60)}:${p(ff)}`
}

/** Short HH:MM:SS used on media cards. */
export function shortDuration(t: Ticks): string {
  const s = Math.round(seconds(t))
  const p = (n: number) => String(n).padStart(2, '0')
  return `${p(Math.floor(s / 3600))}:${p(Math.floor(s / 60) % 60)}:${p(s % 60)}`
}

export function mainSequence(p: Project): Sequence {
  return p.sequences.find(s => s.id === p.main_sequence) ?? p.sequences[0]
}

export function sequenceDuration(s: Sequence): Ticks {
  let d = 0
  for (const t of s.tracks) for (const c of t.clips) d = Math.max(d, c.end)
  return d
}

export function findClip(p: Project, clipId: string): { track: Track; clip: Clip } | null {
  for (const s of p.sequences)
    for (const track of s.tracks) {
      const clip = track.clips.find(c => c.id === clipId)
      if (clip) return { track, clip }
    }
  return null
}

export function findTrack(p: Project, trackId: string): Track | null {
  for (const s of p.sequences) for (const t of s.tracks) if (t.id === trackId) return t
  return null
}

export function mediaOf(p: Project, clip: Clip): MediaAsset | null {
  return clip.source.type === 'media' ? (p.media.find(m => m.id === (clip.source as { media_id: string }).media_id) ?? null) : null
}

export function mediaInUse(p: Project, mediaId: string): boolean {
  return p.sequences.some(s => s.tracks.some(t => t.clips.some(c => c.source.type === 'media' && c.source.media_id === mediaId)))
}

/** Source time played at clip-local time `local` (constant speed only). */
export function sourceTime(clip: Clip, local: Ticks): Ticks {
  const rate = clip.speed?.rate ?? 1
  return (clip.source_in ?? 0) + Math.round(local * rate)
}

/** Length of source media the clip consumes. */
export function sourceLength(clip: Clip): Ticks {
  return Math.round((clip.end - clip.start) * (clip.speed?.rate ?? 1))
}

/** A clip that plays a whole media asset starting at `start`. */
export function clipFromMedia(m: MediaAsset, start: Ticks, linkGroup?: string): Clip {
  return {
    id: newId(),
    name: m.name,
    source: { type: 'media', media_id: m.id },
    start,
    end: start + Math.max(m.duration, TICKS_PER_SECOND / 10),
    source_in: 0,
    speed: { rate: 1, reverse: false, maintain_pitch: true },
    transform: defaultTransform(),
    opacity: float(1),
    blend_mode: 'normal',
    audio: defaultAudio(),
    ...(linkGroup ? { link_group: linkGroup } : {}),
  }
}

/** Timeline tracks in display order: video tracks top (highest first), then audio. */
export function displayTracks(s: Sequence): Track[] {
  const video = s.tracks.filter(t => t.kind === 'video').reverse()
  const audio = s.tracks.filter(t => t.kind === 'audio')
  return [...video, ...audio]
}

/** True if [start,end) fits on the track without overlapping clips other than `ignore`. */
export function fits(track: Track, start: Ticks, end: Ticks, ignore: Set<string> = new Set()): boolean {
  if (start < 0) return false
  return track.clips.every(c => ignore.has(c.id) || c.end <= start || c.start >= end)
}
