// A light TypeScript version of `jb_core::eval` for the DOM preview: which
// clips are visible and audible at an instant.

import { paramValue, sourceTime } from '../core/model'
import type { Clip, MediaAsset, Project, Ticks, Track } from '../core/types'

export interface ActiveClip {
  clip: Clip
  track: Track
  media: MediaAsset | null
  /** Source time in ticks. */
  source: Ticks
}

function sourceAt(clip: Clip, t: Ticks): Ticks {
  const local = t - clip.start
  if (clip.speed?.reverse) {
    const total = Math.round((clip.end - clip.start) * (clip.speed.rate ?? 1))
    return (clip.source_in ?? 0) + Math.max(0, total - Math.round(local * (clip.speed.rate ?? 1)))
  }
  return sourceTime(clip, local)
}

/** Visible layers, bottom first. */
export function visualLayers(p: Project, t: Ticks): ActiveClip[] {
  const seq = p.sequences.find(s => s.id === p.main_sequence) ?? p.sequences[0]
  const out: ActiveClip[] = []
  for (const track of seq.tracks) {
    if (track.kind !== 'video' || track.hidden) continue
    const clip = track.clips.find(c => !c.disabled && t >= c.start && t < c.end)
    if (!clip) continue
    const media = clip.source.type === 'media' ? (p.media.find(m => m.id === (clip.source as { media_id: string }).media_id) ?? null) : null
    out.push({ clip, track, media, source: sourceAt(clip, t) })
  }
  return out
}

/** Audible clips with their linear gain. */
export function audioVoices(p: Project, t: Ticks): (ActiveClip & { gain: number })[] {
  const seq = p.sequences.find(s => s.id === p.main_sequence) ?? p.sequences[0]
  const out: (ActiveClip & { gain: number })[] = []
  for (const track of seq.tracks) {
    if (track.muted) continue
    for (const clip of track.clips) {
      if (clip.disabled || t < clip.start || t >= clip.end || clip.source.type !== 'media') continue
      const media = p.media.find(m => m.id === (clip.source as { media_id: string }).media_id)
      if (!media || !(media.has_audio || media.kind === 'audio')) continue
      if (track.kind === 'video' && clip.link_group) continue
      if (clip.audio?.muted) continue
      const local = t - clip.start
      let g = Math.pow(10, paramValue(clip.audio?.gain_db, 0) / 20)
      const fi = clip.audio?.fade_in ?? 0
      const fo = clip.audio?.fade_out ?? 0
      if (fi > 0 && local < fi) g *= local / fi
      const rem = clip.end - clip.start - local
      if (fo > 0 && rem < fo) g *= Math.max(0, rem) / fo
      out.push({ clip, track, media, source: sourceAt(clip, t), gain: g })
    }
  }
  return out
}
