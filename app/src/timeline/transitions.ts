// Transition geometry and the commands that add, change and remove them.
// A transition lives on the outgoing clip (`clip.transition_out`); its window
// sits on the cut to the next clip according to `alignment`. On camera clips
// the linked audio clips get a matching crossfade, like Filmora's "Audio
// Transitions-Constant Gain".

import { frameTicks, mainSequence, newId } from '../core/model'
import { TICKS_PER_SECOND, type Clip, type Command, type Project, type Ticks, type Track, type Transition, type TransitionAlignment } from '../core/types'

export const AUDIO_CROSSFADE = 'jb.audio_fade'

/** [from, to) of a transition whose cut is at `cut`. */
export function transitionWindow(tr: Pick<Transition, 'duration' | 'alignment'>, cut: Ticks): [Ticks, Ticks] {
  if (tr.alignment === 'end_at_cut') return [cut - tr.duration, cut]
  if (tr.alignment === 'start_at_cut') return [cut, cut + tr.duration]
  return [cut - Math.floor(tr.duration / 2), cut + Math.ceil(tr.duration / 2)]
}

/** The clip right after `clip` on its track if they touch (within a frame). */
export function nextClip(track: Track, clip: Clip, frame: Ticks): Clip | null {
  const i = track.clips.findIndex(c => c.id === clip.id)
  const b = track.clips[i + 1]
  return b && b.start - clip.end <= frame ? b : null
}

/** Longest transition the clips around the cut allow for an alignment. */
export function maxDuration(a: Clip, b: Clip | null, alignment: TransitionAlignment): Ticks {
  const la = a.end - a.start
  const lb = b ? b.end - b.start : 0
  if (!b) return alignment === 'end_at_cut' ? la : 0
  if (alignment === 'end_at_cut') return la
  if (alignment === 'start_at_cut') return lb
  return 2 * Math.min(la, lb)
}

export function minDuration(p: Project): Ticks {
  return 3 * frameTicks(p.settings.frame_rate)
}

/** Rounds to whole frames, like Filmora's duration fields. */
export function frameRound(p: Project, t: Ticks): Ticks {
  const { num, den } = p.settings.frame_rate
  const frames = Math.max(1, Math.round((t * num) / (den * TICKS_PER_SECOND)))
  return Math.round((frames * den * TICKS_PER_SECOND) / num)
}

/** Alignment that fits: a transition out of the last clip can only end at the cut. */
export function fitAlignment(b: Clip | null, alignment: TransitionAlignment): TransitionAlignment {
  return b ? alignment : 'end_at_cut'
}

// ---------- Defaults ("Set as Default") ----------

export interface TransitionDefault {
  kind: string
  duration: Ticks
}

const DEFAULT_KEY = 'jb.defaultTransition'

export function defaultTransition(): TransitionDefault {
  try {
    const v = JSON.parse(localStorage.getItem(DEFAULT_KEY) ?? 'null')
    if (v && typeof v.kind === 'string' && typeof v.duration === 'number') return v
  } catch {
    /* storage unavailable */
  }
  return { kind: 'jb.dissolve', duration: 2 * TICKS_PER_SECOND }
}

export function setDefaultTransition(d: TransitionDefault) {
  try {
    localStorage.setItem(DEFAULT_KEY, JSON.stringify(d))
  } catch {
    /* storage unavailable */
  }
}

// ---------- Commands ----------

/** Audio clips linked to video clip `c` that end where it ends (its camera audio). */
function linkedAudio(p: Project, c: Clip): { track: Track; clip: Clip }[] {
  if (!c.link_group) return []
  const out: { track: Track; clip: Clip }[] = []
  for (const t of mainSequence(p).tracks)
    if (t.kind === 'audio') for (const x of t.clips) if (x.link_group === c.link_group && x.end === c.end) out.push({ track: t, clip: x })
  return out
}

/** The audio crossfades that go with the transition out of video clip `a`. */
export function audioPartners(p: Project, track: Track, a: Clip): Clip[] {
  if (track.kind !== 'video') return []
  return linkedAudio(p, a).map(l => l.clip)
}

/**
 * Sets the transition out of `a` (on `track`) and the matching crossfade on
 * its linked audio, as one list of commands. `audio: false` leaves audio alone.
 */
export function setTransitionCommands(p: Project, track: Track, a: Clip, tr: Transition, audio = true): Command[] {
  const cmds: Command[] = [{ op: 'set_transition', clip_id: a.id, transition: tr }]
  if (!audio) return cmds
  const frame = frameTicks(p.settings.frame_rate)
  for (const la of linkedAudio(p, a)) {
    if (track.kind !== 'video') break
    const b = nextClip(la.track, la.clip, frame)
    const existing = la.clip.transition_out
    // A crossfade needs a next audio clip, except at the very end (fade out).
    if (!b && tr.alignment !== 'end_at_cut') continue
    cmds.push({
      op: 'set_transition',
      clip_id: la.clip.id,
      transition: { id: existing?.id ?? newId(), kind: existing?.kind ?? AUDIO_CROSSFADE, duration: tr.duration, alignment: tr.alignment, params: existing?.params ?? {} },
    })
  }
  return cmds
}

/** Removes the transition out of `a` and its audio crossfades. */
export function removeTransitionCmds(p: Project, track: Track, a: Clip): Command[] {
  const cmds: Command[] = [{ op: 'set_transition', clip_id: a.id, transition: null }]
  for (const c of audioPartners(p, track, a)) if (c.transition_out) cmds.push({ op: 'set_transition', clip_id: c.id, transition: null })
  return cmds
}

/** Every clip that has a transition, with its track, on the main sequence. */
export function allTransitions(p: Project): { track: Track; clip: Clip }[] {
  const out: { track: Track; clip: Clip }[] = []
  for (const t of mainSequence(p).tracks) for (const c of t.clips) if (c.transition_out) out.push({ track: t, clip: c })
  return out
}

/** Cuts (pairs of touching clips) on a track. */
export function cuts(p: Project, track: Track): { a: Clip; b: Clip }[] {
  const frame = frameTicks(p.settings.frame_rate)
  const out: { a: Clip; b: Clip }[] = []
  for (let i = 0; i + 1 < track.clips.length; i++) {
    const a = track.clips[i]
    const b = track.clips[i + 1]
    if (b.start - a.end <= frame) out.push({ a, b })
  }
  return out
}

/**
 * The cut nearest `time` on `track` within `maxDist`, or the end of the last
 * clip (a fade out) when that is nearer.
 */
export function cutNear(p: Project, track: Track, time: Ticks, maxDist: Ticks): { a: Clip; b: Clip | null } | null {
  let best: { a: Clip; b: Clip | null; d: number } | null = null
  for (const { a, b } of cuts(p, track)) {
    const d = Math.abs(a.end - time)
    if (d <= maxDist && (!best || d < best.d)) best = { a, b, d }
  }
  const last = track.clips[track.clips.length - 1]
  if (last) {
    const d = Math.abs(last.end - time)
    if (d <= maxDist && (!best || d < best.d)) best = { a: last, b: null, d }
  }
  return best ? { a: best.a, b: best.b } : null
}

/** A new transition of `kind` for the cut after `a`, clamped to what the clips allow. */
export function newTransition(p: Project, a: Clip, b: Clip | null, kind: string, duration: Ticks, alignment: TransitionAlignment = 'center'): Transition {
  const al = fitAlignment(b, alignment)
  const d = frameRound(p, Math.max(minDuration(p), Math.min(duration, maxDuration(a, b, al))))
  return { id: newId(), kind, duration: d, alignment: al, params: {} }
}
