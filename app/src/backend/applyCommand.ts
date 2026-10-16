// A TypeScript port of `jb_core::command::apply`, used only by the mock
// backend (browser dev mode and UI tests). The Tauri app always executes
// commands in Rust; behavior here must follow crates/jb-core/src/command.rs.

import { newId, sourceTime } from '../core/model'
import { EQ_PRESETS, RAMP_PRESETS, rampStretch } from '../core/presets'
import type { AudioSettings, Clip, Command, Interpolation, Marker, Param, Project, Track, TrackKind, Transform, Value } from '../core/types'

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
    default:
      applyPanel(p, cmd)
  }
}

// ---------- Property-panel commands (ops.rs) ----------

const defaultAudio = (): AudioSettings => ({ gain_db: fl(0), pan: fl(0), fade_in: 0, fade_out: 0, pitch: 0, muted: false })
const defaultTransform = (): Transform => ({ position_x: fl(0), position_y: fl(0), scale_x: fl(1), scale_y: fl(1), rotation: fl(0), flip_h: false, flip_v: false, crop: [0, 0, 0, 0], fit: 'fit' })
function fl(v: number): Param {
  return { value: { type: 'float', value: v } }
}

/** Where a camera clip's sound lives: its linked audio clips (or itself). */
function audioClips(p: Project, clipId: string): string[] {
  const l = locate(p, clipId)
  if (!l) throw err(`not found: clip ${clipId}`)
  const track = p.sequences[l[0]].tracks[l[1]]
  const clip = track.clips[l[2]]
  if (track.kind === 'audio' || !clip.link_group) return [clipId]
  const linked: string[] = []
  for (const s of p.sequences) for (const t of s.tracks) if (t.kind === 'audio') for (const c of t.clips) if (c.link_group === clip.link_group) linked.push(c.id)
  return linked.length ? linked : [clipId]
}

function withAudio(p: Project, clipId: string, f: (a: AudioSettings) => void) {
  for (const id of audioClips(p, clipId)) withClip(p, id, c => f((c.audio ??= defaultAudio())))
}

/** The params a path addresses (`edit::ParamPath`), creating effect params on demand. */
function paramsAt(c: Clip, path: string, create?: Value): { get: () => Param | undefined; set: (v: Param) => void }[] {
  const tr = () => (c.transform ??= defaultTransform())
  const field = (get: () => Param | undefined, set: (v: Param) => void) => ({ get, set })
  switch (path) {
    case 'transform.position_x':
    case 'transform.position_y':
    case 'transform.scale_x':
    case 'transform.scale_y':
    case 'transform.rotation': {
      const k = path.slice(10) as 'position_x' | 'position_y' | 'scale_x' | 'scale_y' | 'rotation'
      return [field(() => tr()[k], v => void (tr()[k] = v))]
    }
    case 'transform.scale':
      return [field(() => tr().scale_x, v => void (tr().scale_x = v)), field(() => tr().scale_y, v => void (tr().scale_y = v))]
    case 'opacity':
      return [field(() => c.opacity ?? fl(1), v => void (c.opacity = v))]
    case 'volume':
      return [field(() => (c.audio ??= defaultAudio()).gain_db, v => void ((c.audio ??= defaultAudio()).gain_db = v))]
    case 'pan':
      return [field(() => (c.audio ??= defaultAudio()).pan, v => void ((c.audio ??= defaultAudio()).pan = v))]
  }
  const m = /^effect\.([^.]+)\.(.+)$/.exec(path)
  if (!m) throw err(`invalid: unknown parameter path \`${path}\``)
  const e = [...(c.effects ?? []), ...(c.animations ?? [])].find(x => x.id === m[1])
  if (!e) throw err(`not found: effect ${m[1]}`)
  const name = m[2]
  if (!e.params?.[name] && !create) throw err(`not found: parameter ${name}`)
  return [field(() => e.params?.[name] ?? { value: structuredClone(create!) }, v => void ((e.params ??= {})[name] = v))]
}

function upsertKeyframe(prm: Param, time: number, value: Value, interpolation: Interpolation) {
  const kfs = (prm.keyframes ??= [])
  if (!kfs.length) prm.value = structuredClone(value)
  const i = kfs.findIndex(k => k.time >= time)
  const kf = { time, value: structuredClone(value), interpolation }
  if (i >= 0 && kfs[i].time === time) kfs[i] = kf
  else if (i >= 0) kfs.splice(i, 0, kf)
  else kfs.push(kf)
}

function audioTargets(p: Project, clipId: string, path: string): string[] {
  return path === 'volume' || path === 'pan' ? audioClips(p, clipId) : [clipId]
}

function applyPanel(p: Project, cmd: Command) {
  switch (cmd.op) {
    case 'rename_media': {
      const m = p.media.find(x => x.id === cmd.media_id)
      if (!m) throw err(`not found: media ${cmd.media_id}`)
      if (!cmd.name.trim()) throw err('invalid: media name is empty')
      m.name = cmd.name.trim()
      break
    }
    case 'set_crop':
      if (cmd.crop.some(v => v < 0 || v >= 1) || cmd.crop[0] + cmd.crop[2] >= 1 || cmd.crop[1] + cmd.crop[3] >= 1) throw err('invalid: crop insets out of range')
      withClip(p, cmd.clip_id, c => void ((c.transform ??= defaultTransform()).crop = [...cmd.crop]))
      break
    case 'set_flip':
      withClip(p, cmd.clip_id, c => {
        const t = (c.transform ??= defaultTransform())
        if (cmd.horizontal != null) t.flip_h = cmd.horizontal
        if (cmd.vertical != null) t.flip_v = cmd.vertical
      })
      break
    case 'set_rotation':
      withClip(p, cmd.clip_id, c => void ((c.transform ??= defaultTransform()).rotation = fl(cmd.degrees)))
      break
    case 'set_fit_mode':
      withClip(p, cmd.clip_id, c => void ((c.transform ??= defaultTransform()).fit = cmd.fit))
      break
    case 'set_blend_mode':
      withClip(p, cmd.clip_id, c => void (c.blend_mode = cmd.blend_mode))
      break
    case 'set_param':
      for (const id of audioTargets(p, cmd.clip_id, cmd.param))
        withClip(p, id, c => {
          for (const f of paramsAt(c, cmd.param, cmd.value)) f.set({ value: structuredClone(cmd.value) })
        })
      break
    case 'add_keyframe':
      for (const id of audioTargets(p, cmd.clip_id, cmd.param))
        withClip(p, id, c => {
          if (cmd.time < 0 || cmd.time > c.end - c.start) throw err('invalid: keyframe time must be within the clip')
          for (const f of paramsAt(c, cmd.param, cmd.value)) {
            const prm = structuredClone(f.get() ?? { value: cmd.value })
            upsertKeyframe(prm, cmd.time, cmd.value, cmd.interpolation ?? 'linear')
            f.set(prm)
          }
        })
      break
    case 'remove_keyframe':
      for (const id of audioTargets(p, cmd.clip_id, cmd.param))
        withClip(p, id, c => {
          let any = false
          for (const f of paramsAt(c, cmd.param)) {
            const prm = f.get()
            if (!prm?.keyframes?.length) continue
            const i = prm.keyframes.findIndex(k => k.time === cmd.time)
            if (i < 0) continue
            const [gone] = prm.keyframes.splice(i, 1)
            if (!prm.keyframes.length) prm.value = gone.value
            any = true
          }
          if (!any) throw err(`not found: keyframe at ${cmd.time} on ${cmd.param}`)
        })
      break
    case 'move_keyframe':
      withClip(p, cmd.clip_id, c => {
        for (const f of paramsAt(c, cmd.param)) {
          const prm = f.get()
          const k = prm?.keyframes?.find(x => x.time === cmd.from)
          if (!prm || !k) continue
          prm.keyframes = prm.keyframes!.filter(x => x !== k && x.time !== cmd.to)
          upsertKeyframe(prm, cmd.to, k.value, k.interpolation ?? 'linear')
        }
      })
      break
    case 'clear_keyframes':
      for (const id of audioTargets(p, cmd.clip_id, cmd.param))
        withClip(p, id, c => {
          for (const f of paramsAt(c, cmd.param)) {
            const prm = f.get()
            if (prm?.keyframes?.length) f.set({ value: prm.keyframes[0].value })
          }
        })
      break
    case 'set_volume':
      withAudio(p, cmd.clip_id, a => void (a.gain_db = fl(cmd.gain_db)))
      break
    case 'set_pan':
      if (cmd.pan < -1 || cmd.pan > 1) throw err('invalid: pan must be within -1..1')
      withAudio(p, cmd.clip_id, a => void (a.pan = fl(cmd.pan)))
      break
    case 'set_pitch':
      withAudio(p, cmd.clip_id, a => void (a.pitch = Math.max(-12, Math.min(12, cmd.semitones))))
      break
    case 'set_mute':
      withAudio(p, cmd.clip_id, a => void (a.muted = cmd.muted))
      break
    case 'set_audio_channels':
      withAudio(p, cmd.clip_id, a => (cmd.channels === 'stereo' ? delete a.channels : (a.channels = cmd.channels)))
      break
    case 'set_audio_fades':
      withAudio(p, cmd.clip_id, a => {
        if (cmd.fade_in != null) a.fade_in = cmd.fade_in
        if (cmd.fade_out != null) a.fade_out = cmd.fade_out
      })
      break
    case 'set_normalization':
      withAudio(p, cmd.clip_id, a => {
        if (cmd.target_lufs == null) delete a.normalize
        else a.normalize = { target_lufs: cmd.target_lufs, ...(cmd.measured_lufs != null ? { measured_lufs: cmd.measured_lufs } : {}) }
      })
      break
    case 'set_equalizer':
      withAudio(p, cmd.clip_id, a => {
        if (cmd.bands) a.equalizer = { preset: 'custom', bands: [...cmd.bands] }
        else if (cmd.preset) {
          const pr = EQ_PRESETS.find(x => x.id === cmd.preset)
          if (!pr) throw err(`invalid: unknown equalizer preset ${cmd.preset}`)
          if (pr.id === 'default') delete a.equalizer
          else a.equalizer = { preset: pr.id, bands: [...pr.bands] }
        } else delete a.equalizer
      })
      break
    case 'set_maintain_pitch':
      for (const id of linkedIds(p, cmd.clip_id)) withClip(p, id, c => void ((c.speed ??= { rate: 1 }).maintain_pitch = cmd.maintain_pitch))
      break
    case 'set_reverse':
      for (const id of linkedIds(p, cmd.clip_id)) withClip(p, id, c => void ((c.speed ??= { rate: 1 }).reverse = cmd.reverse))
      break
    case 'set_speed_ramp': {
      const preset = cmd.preset.toLowerCase().replace(/[\s-]/g, '_')
      const points = cmd.points ?? (preset === 'none' ? null : RAMP_PRESETS.find(r => r.id === preset)?.points)
      if (preset !== 'none' && !points) throw err(`invalid: unknown speed ramp preset ${cmd.preset}`)
      for (const id of linkedIds(p, cmd.clip_id))
        withClip(p, id, c => {
          const speed = (c.speed ??= { rate: 1 })
          const span = Math.round((c.end - c.start) * (speed.curve ? 1 / rampStretch(speed.curve.points) : speed.rate))
          delete speed.ramp
          if (!points) {
            delete speed.curve
            speed.rate = 1
            c.end = c.start + span
          } else {
            const stretch = rampStretch(points)
            speed.curve = { preset, points: structuredClone(points) }
            speed.rate = 1 / stretch
            c.end = c.start + Math.max(1, Math.round(span * stretch))
          }
        })
      for (const s of p.sequences) for (const t of s.tracks) sortAndCheck(t)
      break
    }
    case 'set_clip_enabled':
      for (const id of linkedIds(p, cmd.clip_id)) withClip(p, id, c => void (cmd.enabled ? delete c.disabled : (c.disabled = true)))
      break
    case 'set_effect_enabled':
      withClip(p, cmd.clip_id, c => {
        const e = [...(c.effects ?? []), ...(c.animations ?? [])].find(x => x.id === cmd.effect_id)
        if (!e) throw err(`not found: effect ${cmd.effect_id}`)
        e.enabled = cmd.enabled
      })
      break
    case 'add_animation':
      withClip(p, cmd.clip_id, c => {
        const len = c.end - c.start
        const d = Math.min(cmd.duration ?? 10_000_000, len)
        const range: [number, number] = cmd.placement === 'out' ? [len - d, len] : cmd.placement === 'in' ? [0, d] : [0, len]
        c.animations = (c.animations ?? []).filter(a => a.placement !== cmd.placement)
        c.animations.push({ id: cmd.id ?? newId(), kind: cmd.kind, enabled: true, params: structuredClone(cmd.params ?? {}), range, placement: cmd.placement })
      })
      break
    case 'remove_animation':
      withClip(p, cmd.clip_id, c => {
        const before = c.animations?.length ?? 0
        c.animations = (c.animations ?? []).filter(a => (cmd.animation_id ? a.id !== cmd.animation_id : a.placement !== cmd.placement))
        if (c.animations.length === before) throw err('not found: animation')
      })
      break
    case 'replace_animation':
      withClip(p, cmd.clip_id, c => {
        const a = c.animations?.find(x => x.id === cmd.animation_id)
        if (!a) throw err(`not found: animation ${cmd.animation_id}`)
        if (cmd.kind) a.kind = cmd.kind
        if (cmd.params) a.params = structuredClone(cmd.params)
        if (cmd.duration != null) {
          const len = c.end - c.start
          const d = Math.min(cmd.duration, len)
          a.range = a.placement === 'out' ? [len - d, len] : [0, d]
        }
      })
      break
    case 'set_title_style':
      withClip(p, cmd.clip_id, c => {
        if (c.source.type !== 'title') throw err('invalid: clip is not a title')
        const { op: _op, clip_id: _id, animation, ...patch } = cmd
        void _op
        void _id
        const src = c.source as unknown as Record<string, unknown>
        for (const [k, v] of Object.entries(patch)) {
          if (v === undefined) continue
          if (v === null) delete src[k]
          else src[k] = structuredClone(v)
        }
        if (animation) {
          const placement = animation.placement ?? 'in'
          const len = c.end - c.start
          const d = Math.min(animation.duration ?? 10_000_000, len)
          c.animations = (c.animations ?? []).filter(a => a.placement !== placement)
          c.animations.push({ id: newId(), kind: animation.kind, enabled: true, params: {}, range: placement === 'out' ? [len - d, len] : [0, d], placement })
        }
      })
      break
    case 'add_marker': {
      const marker = { duration: 0, name: '', color: [0, 206 / 255, 208 / 255, 1] as [number, number, number, number], ...structuredClone(cmd.marker) }
      if (cmd.clip_id) withClip(p, cmd.clip_id, c => void (c.markers = [...(c.markers ?? []), marker].sort((a, b) => a.time - b.time)))
      else {
        const seq = p.sequences.find(s => s.id === (cmd.sequence_id ?? p.main_sequence))
        if (!seq) throw err('not found: sequence')
        seq.markers = [...(seq.markers ?? []), marker].sort((a, b) => a.time - b.time)
      }
      break
    }
    case 'update_marker':
    case 'remove_marker': {
      const id = cmd.op === 'update_marker' ? cmd.marker.id : cmd.marker_id
      let found = false
      const edit = (list: Marker[] | undefined) =>
        (list ?? []).flatMap(m => {
          if (m.id !== id) return [m]
          found = true
          return cmd.op === 'update_marker' ? [structuredClone(cmd.marker)] : []
        })
      for (const s of p.sequences) {
        s.markers = edit(s.markers)
        for (const t of s.tracks) for (const c of t.clips) c.markers = edit(c.markers)
      }
      if (!found) throw err(`not found: marker ${id}`)
      break
    }
    case 'set_project_settings': {
      const s = p.settings
      for (const v of [cmd.width, cmd.height]) if (v != null && (v <= 0 || v % 2)) throw err('invalid: project size must be positive and even')
      if (cmd.width != null) s.width = cmd.width
      if (cmd.height != null) s.height = cmd.height
      if (cmd.frame_rate) s.frame_rate = { ...cmd.frame_rate }
      if (cmd.background) s.background = [...cmd.background]
      break
    }
    case 'freeze_frame': {
      // The mock keeps it simple: the clip grows by the frozen duration and
      // later clips on its track move right (the core also splits the clip).
      const l = locate(p, cmd.clip_id)
      if (!l) throw err(`not found: clip ${cmd.clip_id}`)
      const track = p.sequences[l[0]].tracks[l[1]]
      withClip(p, cmd.clip_id, c => {
        if (cmd.at <= c.start || cmd.at >= c.end) throw err('invalid: freeze point outside the clip')
        for (const o of track.clips) if (o !== c && o.start >= c.end) {
          o.start += cmd.duration
          o.end += cmd.duration
        }
        c.end += cmd.duration
      })
      for (const s of p.sequences) for (const t of s.tracks) sortAndCheck(t)
      break
    }
    default:
      throw err(`invalid: the browser mock does not implement ${cmd.op}`)
  }
}
