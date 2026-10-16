// Property-panel editing helpers. Every change is one jb-core Command (or
// a batch, one undo step) sent through `exec`, like the timeline's edits.

import { findClip, frameTicks, mainSequence, newId, paramAt, sampleParam } from './core/model'
import type { Clip, Command, EffectInstance, Param, Value } from './core/types'
import { backend } from './backend'
import { applyBackend, exec, getState, project, seek, setState, toast } from './store'

export const fv = (v: number): Value => ({ type: 'float', value: v })

/** Clip-local time of the playhead, or null when the playhead is outside the clip. */
export function localTime(clip: Clip): number | null {
  const t = getState().playhead
  return t >= clip.start && t <= clip.end ? t - clip.start : null
}

/** Clip-local playhead time clamped into the clip (for showing animated values). */
export function localClamped(clip: Clip): number {
  return Math.max(0, Math.min(clip.end - clip.start, getState().playhead - clip.start))
}

/** Value of a float param path at the playhead. */
export function valueAt(clip: Clip, path: string, fallback: number): number {
  return sampleParam(paramAt(clip, path), localClamped(clip), fallback)
}

const halfFrame = () => {
  const p = project()
  return p ? frameTicks(p.settings.frame_rate) / 2 : 166_667
}

export function keyframeTimes(clip: Clip, path: string): number[] {
  return (paramAt(clip, path)?.keyframes ?? []).map(k => k.time)
}

/** The keyframe under the playhead, if any. */
export function keyframeHere(clip: Clip, path: string): number | null {
  const t = localTime(clip)
  if (t == null) return null
  return keyframeTimes(clip, path).find(k => Math.abs(k - t) <= halfFrame()) ?? null
}

/**
 * Commands that set a float param: a keyframe at the playhead when the param
 * is animated (Filmora's behavior), a constant value otherwise.
 */
export function setParamCommands(clip: Clip, path: string, value: number, target: Clip = clip): Command[] {
  const animated = (paramAt(target, path)?.keyframes?.length ?? 0) > 0
  const t = localTime(clip)
  if (animated && t != null) {
    const at = keyframeHere(target, path) ?? t
    return [{ op: 'add_keyframe', clip_id: target.id, param: path, time: at, value: fv(value) }]
  }
  if (animated) toast('Move the playhead over the clip to change an animated value')
  return animated ? [] : [{ op: 'set_param', clip_id: target.id, param: path, value: fv(value) }]
}

export function setParam(clip: Clip, path: string, value: number, target: Clip = clip) {
  const cmds = setParamCommands(clip, path, value, target)
  if (cmds.length) return exec(cmds.length === 1 ? cmds[0] : { op: 'batch', commands: cmds })
  return Promise.resolve(false)
}

/** Diamond: adds a keyframe with the current value at the playhead, or removes the one there. */
export function toggleKeyframe(clip: Clip, paths: string[], values: number[], target: Clip = clip) {
  const t = localTime(clip)
  if (t == null) return toast('Move the playhead over the clip to add a keyframe')
  const here = paths.map(p => keyframeHere(target, p))
  const cmds: Command[] = here.every(h => h != null)
    ? paths.map((p, i) => ({ op: 'remove_keyframe', clip_id: target.id, param: p, time: here[i]! }))
    : paths.map((p, i) => ({ op: 'add_keyframe', clip_id: target.id, param: p, time: here[i] ?? t, value: fv(values[i]) }))
  exec(cmds.length === 1 ? cmds[0] : { op: 'batch', commands: cmds })
}

/** Moves the playhead to the previous/next keyframe of a param. */
export function jumpKeyframe(clip: Clip, path: string, dir: -1 | 1) {
  const now = getState().playhead - clip.start
  const ts = keyframeTimes(clip, path)
  const next = dir > 0 ? ts.find(k => k > now + halfFrame()) : [...ts].reverse().find(k => k < now - halfFrame())
  if (next != null) seek(clip.start + next)
}

/**
 * The clip whose audio settings a selection edits: a camera clip's sound
 * lives on its linked audio clip (that is what plays, and what Filmora
 * stores), so the Audio tab of a linked video clip edits that one.
 */
export function audioTarget(clip: Clip): Clip {
  if (!clip.link_group) return clip
  const p = project()
  if (!p) return clip
  const own = findClip(p, clip.id)
  if (own?.track.kind === 'audio') return clip
  for (const t of mainSequence(p).tracks) if (t.kind === 'audio') for (const c of t.clips) if (c.link_group === clip.link_group) return c
  return clip
}

// ---------- Effects ----------

export const findEffect = (clip: Clip, kind: string) => (clip.effects ?? []).find(e => e.kind === kind)

const toParams = (params: Record<string, number | Value>): Record<string, Param> =>
  Object.fromEntries(Object.entries(params).map(([k, v]) => [k, { value: typeof v === 'number' ? fv(v) : v }]))

export function addEffectCommand(clip: Clip, kind: string, params: Record<string, number | Value> = {}): { cmd: Command; id: string } {
  const effect: EffectInstance = { id: newId(), kind, enabled: true, params: toParams(params) }
  return { cmd: { op: 'add_effect', clip_id: clip.id, effect }, id: effect.id }
}

// ---------- Live preview while dragging ----------

let liveBusy = false
let livePending: Command | null | undefined

/**
 * Shows `cmd`'s result in the player right away without an undo step, so
 * the frame follows a slider while it is dragged (Filmora); the release
 * commits with `exec`. Calls are coalesced: only the latest waits.
 */
export function livePreview(cmd: Command | null) {
  const eng = backend.engine
  if (!eng) return
  if (liveBusy) {
    livePending = cmd
    return
  }
  liveBusy = true
  eng.previewCommand(cmd).finally(() => {
    liveBusy = false
    const next = livePending
    livePending = undefined
    if (next !== undefined) livePreview(next)
  })
}

/** Live preview of an effect param value (see `livePreview`). */
export function liveEffectParam(clip: Clip, effect: EffectInstance | undefined, kind: string, name: string, v: number) {
  livePreview(effect ? { op: 'set_effect_params', clip_id: clip.id, effect_id: effect.id, params: toParams({ [name]: v }) } : addEffectCommand(clip, kind, { [name]: v }).cmd)
}

/** Sets effect params, adding the effect first when the clip has none of that kind. */
export function setEffectParams(clip: Clip, kind: string, params: Record<string, number | Value>) {
  const e = findEffect(clip, kind)
  if (e) return exec({ op: 'set_effect_params', clip_id: clip.id, effect_id: e.id, params: toParams(params) })
  return exec(addEffectCommand(clip, kind, params).cmd)
}

/** Module switch for an effect-backed module: on adds it (or re-enables it), off disables it. */
export function switchEffect(clip: Clip, kind: string, on: boolean, params: Record<string, number | Value> = {}) {
  const e = findEffect(clip, kind)
  if (on && !e) return exec(addEffectCommand(clip, kind, params).cmd)
  if (e) return exec({ op: 'set_effect_enabled', clip_id: clip.id, effect_id: e.id, enabled: on })
  return Promise.resolve(true)
}

export function removeEffects(clip: Clip, pred: (e: EffectInstance) => boolean) {
  const list = (clip.effects ?? []).filter(pred)
  if (!list.length) return Promise.resolve(true)
  return exec({ op: 'batch', commands: list.map(e => ({ op: 'remove_effect', clip_id: clip.id, effect_id: e.id }) as Command) })
}

export const effectOn = (e: EffectInstance | undefined) => !!e && e.enabled !== false

/** Float value of an effect parameter at the playhead (keyframes included). */
export function effectValue(clip: Clip, e: EffectInstance | undefined, name: string, fallback: number): number {
  if (!e) return fallback
  return sampleParam(e.params?.[name], localClamped(clip), fallback)
}

// ---------- Module switches without a model flag ----------

/**
 * Filmora's Transform/Compositing/Color module switches disable a module and
 * bring its values back when switched on again. jb-core has no flag for
 * that, so switching off resets the module (one undo step) and keeps the
 * previous values here to restore them.
 */
const stash = new Map<string, Command[]>()

export function moduleOff(key: string, clip: Clip, reset: Command[], restore: Command[]) {
  stash.set(`${clip.id}:${key}`, restore)
  return exec(reset.length === 1 ? reset[0] : { op: 'batch', commands: reset })
}

export function moduleOn(key: string, clip: Clip) {
  const k = `${clip.id}:${key}`
  const restore = stash.get(k)
  stash.delete(k)
  if (restore?.length) return exec(restore.length === 1 ? restore[0] : { op: 'batch', commands: restore })
  return Promise.resolve(true)
}

export const moduleIsOff = (key: string, clip: Clip) => stash.has(`${clip.id}:${key}`)

// ---------- Tools menu ----------

/** Selected clips, or the clips under the playhead when nothing is selected. */
export function targetClips(kind?: 'video' | 'audio'): Clip[] {
  const p = project()
  if (!p) return []
  const s = getState()
  const sel = s.selection.map(id => findClip(p, id)).filter((x): x is NonNullable<typeof x> => !!x)
  const pool = sel.length ? sel : mainSequence(p).tracks.flatMap(track => track.clips.filter(c => s.playhead >= c.start && s.playhead < c.end).map(clip => ({ track, clip })))
  return pool.filter(x => !kind || x.track.kind === kind).map(x => x.clip)
}

export async function freezeFrame() {
  const t = getState().playhead
  const clip = targetClips('video').find(c => t > c.start && t < c.end && c.source.type === 'media')
  if (!clip) return toast('Move the playhead over a video clip to freeze a frame')
  await exec({ op: 'freeze_frame', clip_id: clip.id, at: t, duration: 50_000_000 })
}

export async function detachAudio() {
  const clips = targetClips('video').filter(c => c.source.type === 'media')
  if (!clips.length) return toast('Select a video clip to detach its audio')
  await exec({ op: 'batch', commands: clips.map(c => ({ op: 'detach_audio', clip_id: c.id, track_id: null }) as Command) })
}

export async function toggleMute() {
  const clips = targetClips()
  if (!clips.length) return toast('Select a clip to mute it')
  const muted = !clips.every(c => audioTarget(c).audio?.muted)
  await exec({ op: 'batch', commands: clips.map(c => ({ op: 'set_mute', clip_id: c.id, muted }) as Command) })
}

/** Adds (or re-enables) stabilization on a clip and runs the motion analysis. */
export async function stabilizeClip(clip: Clip): Promise<boolean> {
  const e = findEffect(clip, 'jb.stabilize')
  if (e && e.enabled === false && !(await exec({ op: 'set_effect_enabled', clip_id: clip.id, effect_id: e.id, enabled: true }))) return false
  let id = e?.id
  if (!e) {
    const add = addEffectCommand(clip, 'jb.stabilize', { smoothness: 50 })
    if (!(await exec(add.cmd))) return false
    id = add.id
  }
  toast('Analyzing camera motion…')
  const ok = await applyBackend(backend.analyzeStabilization(clip.id, id!))
  if (ok) toast('Stabilization applied')
  return ok
}

export async function stabilizeSelection() {
  const clip = targetClips('video').find(c => c.source.type === 'media')
  if (!clip) return toast('Select a video clip to stabilize it')
  setState({ propsTab: 'video', selection: [clip.id] })
  await stabilizeClip(clip)
}

export async function chromaKeySelection() {
  const clip = targetClips('video')[0]
  if (!clip) return toast('Select a video clip to apply Chroma Key')
  setState({ propsTab: 'video', selection: [clip.id] })
  await switchEffect(clip, 'jb.chroma_key', true)
}
