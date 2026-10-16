// A TypeScript port of `jb_core::command::apply`, used only by the mock
// backend (browser dev mode and UI tests). The Tauri app always executes
// commands in Rust; behavior here must follow crates/jb-core/src/command.rs.

import { newId, sourceTime } from '../core/model'
import type { Clip, Command, Project, Track, TrackKind } from '../core/types'

export class CommandError extends Error {}

const err = (m: string) => new CommandError(m)

function trackMut(p: Project, trackId: string): Track {
  for (const s of p.sequences) for (const t of s.tracks) if (t.id === trackId) return t
  throw err(`not found: track ${trackId}`)
}

function sortAndCheck(track: Track) {
  for (let i = 1; i < track.clips.length; i++) {
    const a = track.clips[i - 1]
    const b = track.clips[i]
    if (a.end > b.start) throw err(`invalid: clips ${a.id} and ${b.id} overlap`)
  }
}

function insertClip(track: Track, clip: Clip) {
  if (clip.end <= clip.start) throw err('invalid: clip has non-positive duration')
  let pos = 0
  while (pos < track.clips.length && track.clips[pos].start < clip.start) pos++
  track.clips.splice(pos, 0, clip)
  sortAndCheck(track)
}

function locate(p: Project, clipId: string): [number, number, number] | null {
  for (let si = 0; si < p.sequences.length; si++) {
    const s = p.sequences[si]
    for (let ti = 0; ti < s.tracks.length; ti++) {
      const ci = s.tracks[ti].clips.findIndex(c => c.id === clipId)
      if (ci >= 0) return [si, ti, ci]
    }
  }
  return null
}

function clipRef(p: Project, clipId: string): Clip {
  const l = locate(p, clipId)
  if (!l) throw err(`not found: clip ${clipId}`)
  return p.sequences[l[0]].tracks[l[1]].clips[l[2]]
}

function linkedIds(p: Project, clipId: string): string[] {
  const clip = clipRef(p, clipId)
  if (!clip.link_group) return [clip.id]
  const ids: string[] = []
  for (const s of p.sequences) for (const t of s.tracks) for (const c of t.clips) if (c.link_group === clip.link_group) ids.push(c.id)
  return ids
}

function ensureUnlocked(p: Project, clipId: string) {
  const l = locate(p, clipId)
  if (!l) throw err(`not found: clip ${clipId}`)
  const track = p.sequences[l[0]].tracks[l[1]]
  if (track.locked) throw err(`track is locked: ${track.id}`)
}

function takeClip(p: Project, clipId: string): [number, number, Clip] {
  const l = locate(p, clipId)
  if (!l) throw err(`not found: clip ${clipId}`)
  const [clip] = p.sequences[l[0]].tracks[l[1]].clips.splice(l[2], 1)
  return [l[0], l[1], clip]
}

function withClip(p: Project, clipId: string, f: (c: Clip) => void) {
  ensureUnlocked(p, clipId)
  f(clipRef(p, clipId))
}

const duration = (c: Clip) => c.end - c.start

export function apply(p: Project, cmd: Command): void {
  switch (cmd.op) {
    case 'add_media':
      if (p.media.some(m => m.id === cmd.asset.id)) throw err(`invalid: media ${cmd.asset.id} exists`)
      p.media.push(structuredClone(cmd.asset))
      break
    case 'remove_media': {
      const used = p.sequences.some(s => s.tracks.some(t => t.clips.some(c => c.source.type === 'media' && c.source.media_id === cmd.media_id)))
      if (used) throw err(`invalid: media ${cmd.media_id} is in use`)
      p.media = p.media.filter(m => m.id !== cmd.media_id)
      break
    }
    case 'add_track': {
      const sid = cmd.sequence_id ?? p.main_sequence
      const seq = p.sequences.find(s => s.id === sid)
      if (!seq) throw err(`not found: sequence ${sid}`)
      const n = seq.tracks.filter(t => t.kind === cmd.kind).length + 1
      const name = (cmd.kind as TrackKind) === 'video' ? `Video ${n}` : `Audio ${n}`
      const i = Math.min(cmd.index ?? seq.tracks.length, seq.tracks.length)
      seq.tracks.splice(i, 0, { id: newId(), kind: cmd.kind, name, locked: false, muted: false, hidden: false, clips: [] })
      break
    }
    case 'remove_track': {
      let found = false
      for (const s of p.sequences) {
        const before = s.tracks.length
        s.tracks = s.tracks.filter(t => t.id !== cmd.track_id)
        found ||= s.tracks.length !== before
      }
      if (!found) throw err(`not found: track ${cmd.track_id}`)
      break
    }
    case 'set_track_flags': {
      const t = trackMut(p, cmd.track_id)
      if (cmd.locked != null) t.locked = cmd.locked
      if (cmd.muted != null) t.muted = cmd.muted
      if (cmd.hidden != null) t.hidden = cmd.hidden
      break
    }
    case 'add_clip': {
      const t = trackMut(p, cmd.track_id)
      if (t.locked) throw err(`track is locked: ${cmd.track_id}`)
      insertClip(t, structuredClone(cmd.clip))
      break
    }
    case 'remove_clip':
      for (const id of linkedIds(p, cmd.clip_id)) {
        ensureUnlocked(p, id)
        const [s, t, clip] = takeClip(p, id)
        if (cmd.ripple) {
          const gap = duration(clip)
          for (const c of p.sequences[s].tracks[t].clips)
            if (c.start >= clip.end) {
              c.start -= gap
              c.end -= gap
            }
        }
      }
      break
    case 'move_clip': {
      const anchor = clipRef(p, cmd.clip_id).start
      const delta = cmd.start - anchor
      for (const id of linkedIds(p, cmd.clip_id)) {
        ensureUnlocked(p, id)
        const [s, t, clip] = takeClip(p, id)
        if (clip.start + delta < 0) throw err('invalid: clip would start before zero')
        clip.start += delta
        clip.end += delta
        const target = id === cmd.clip_id ? cmd.track_id : null
        insertClip(target ? trackMut(p, target) : p.sequences[s].tracks[t], clip)
      }
      break
    }
    case 'trim_clip':
      for (const id of linkedIds(p, cmd.clip_id)) {
        ensureUnlocked(p, id)
        const [s, t, clip] = takeClip(p, id)
        if (cmd.start != null) {
          const d = cmd.start - clip.start
          clip.source_in = (clip.source_in ?? 0) + Math.round(d * (clip.speed?.rate ?? 1))
          if (clip.source_in < 0) throw err('invalid: trim before source start')
          clip.start = cmd.start
        }
        if (cmd.end != null) clip.end = cmd.end
        insertClip(p.sequences[s].tracks[t], clip)
      }
      break
    case 'split_clip': {
      for (const id of linkedIds(p, cmd.clip_id)) {
        ensureUnlocked(p, id)
        const [s, t, clip] = takeClip(p, id)
        if (!(cmd.at >= clip.start && cmd.at < clip.end) || cmd.at === clip.start) throw err('invalid: split point outside clip')
        const left = structuredClone(clip)
        const right = clip
        left.end = cmd.at
        delete left.transition_out
        right.id = newId()
        right.source_in = sourceTime(left, cmd.at - left.start)
        right.start = cmd.at
        if (left.audio) left.audio.fade_out = 0
        if (right.audio) right.audio.fade_in = 0
        const track = p.sequences[s].tracks[t]
        insertClip(track, left)
        insertClip(track, right)
      }
      const group = clipRef(p, cmd.clip_id).link_group
      if (group) {
        const next = newId()
        for (const s of p.sequences) for (const t of s.tracks) for (const c of t.clips) if (c.link_group === group && c.start === cmd.at) c.link_group = next
      }
      break
    }
    case 'set_speed': {
      if (cmd.rate <= 0) throw err('invalid: speed must be positive')
      for (const id of linkedIds(p, cmd.clip_id))
        withClip(p, id, c => {
          const speed = c.speed ?? { rate: 1, reverse: false, maintain_pitch: true }
          const sourceLen = duration(c) * speed.rate
          speed.rate = cmd.rate
          delete speed.ramp
          if (cmd.reverse != null) speed.reverse = cmd.reverse
          c.speed = speed
          c.end = c.start + Math.round(sourceLen / cmd.rate)
        })
      for (const s of p.sequences) for (const t of s.tracks) sortAndCheck(t)
      break
    }
    case 'set_transform':
      withClip(p, cmd.clip_id, c => void (c.transform = structuredClone(cmd.transform)))
      break
    case 'set_opacity':
      withClip(p, cmd.clip_id, c => void (c.opacity = structuredClone(cmd.opacity)))
      break
    case 'set_audio':
      withClip(p, cmd.clip_id, c => void (c.audio = structuredClone(cmd.audio)))
      break
    case 'add_effect':
      withClip(p, cmd.clip_id, c => void (c.effects = [...(c.effects ?? []), structuredClone(cmd.effect)]))
      break
    case 'remove_effect':
      withClip(p, cmd.clip_id, c => {
        const before = c.effects?.length ?? 0
        c.effects = (c.effects ?? []).filter(e => e.id !== cmd.effect_id)
        if (c.effects.length === before) throw err(`not found: effect ${cmd.effect_id}`)
      })
      break
    case 'set_effect_params':
      withClip(p, cmd.clip_id, c => {
        const e = [...(c.effects ?? []), ...(c.animations ?? [])].find(e => e.id === cmd.effect_id)
        if (!e) throw err(`not found: effect ${cmd.effect_id}`)
        e.params = { ...(e.params ?? {}), ...structuredClone(cmd.params) }
      })
      break
    case 'set_transition':
      withClip(p, cmd.clip_id, c => {
        if (cmd.transition) c.transition_out = structuredClone(cmd.transition)
        else delete c.transition_out
      })
      break
    case 'set_title':
      withClip(p, cmd.clip_id, c => {
        if (c.source.type !== 'title') throw err('invalid: clip is not a title')
        c.source = { type: 'title', ...structuredClone(cmd.title) }
      })
      break
    case 'rename_project':
      p.name = cmd.name
      break
    case 'batch':
      for (const c of cmd.commands) apply(p, c)
      break
  }
}
