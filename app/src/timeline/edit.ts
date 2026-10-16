// Editing actions behind the timeline toolbar and menus. Every change is a
// jb-core Command sent through `exec`.

import { findClip, frameTicks, mainSequence, newId, paramValue } from '../core/model'
import { TICKS_PER_SECOND, type Clip, type Command, type Marker, type Project, type Ticks, type Track, type Value } from '../core/types'
import { exec, getState, project, select, selectTransition, setState, toast } from '../store'
import { setUi } from './tlstate'
import { allTransitions, cutNear, defaultTransition, newTransition, removeTransitionCmds, setTransitionCommands } from './transitions'

export const FREEZE_DURATION = 5 * TICKS_PER_SECOND
/** The Marker Editor swatches; the second one is Filmora's default. */
export const MARKER_COLORS: [number, number, number, number][] = [
  [28 / 255, 177 / 255, 214 / 255, 1],
  [71 / 255, 191 / 255, 165 / 255, 1],
  [95 / 255, 129 / 255, 255 / 255, 1],
  [129 / 255, 87 / 255, 235 / 255, 1],
  [235 / 255, 126 / 255, 84 / 255, 1],
  [221 / 255, 171 / 255, 75 / 255, 1],
  [219 / 255, 88 / 255, 153 / 255, 1],
]

export const notSupported = (what: string) => () => toast(`${what} needs Wondershare's cloud AI and is not supported in Jibaru Editor`)

export function selected(): { track: Track; clip: Clip } | null {
  const p = project()
  const s = getState()
  if (!p || !s.selection.length) return null
  return findClip(p, s.selection[s.selection.length - 1])
}

function selectedClips(p: Project): Clip[] {
  return getState().selection.map(id => findClip(p, id)?.clip).filter((c): c is Clip => !!c)
}

/** One command per link group (linked clips change together in the core). */
function perGroup(clips: Clip[], f: (c: Clip) => Command | null): Command[] {
  const seen = new Set<string>()
  const out: Command[] = []
  for (const c of clips) {
    const k = c.link_group ?? c.id
    if (seen.has(k)) continue
    seen.add(k)
    const cmd = f(c)
    if (cmd) out.push(cmd)
  }
  return out
}

const run = (cmds: Command[]) => (cmds.length ? exec(cmds.length === 1 ? cmds[0] : { op: 'batch', commands: cmds }) : Promise.resolve(false))

const isMediaLike = (c: Clip) => c.source.type === 'media' || c.source.type === 'sequence'

// ---------- Speed ----------

export function setRate(rate: number) {
  const p = project()
  if (!p) return
  run(perGroup(selectedClips(p).filter(isMediaLike), c => ({ op: 'set_speed', clip_id: c.id, rate, reverse: null })))
}

export function toggleReverse() {
  const p = project()
  if (!p) return
  const clips = selectedClips(p).filter(isMediaLike)
  const on = !clips.every(c => c.speed?.reverse)
  run(perGroup(clips, c => ({ op: 'set_reverse', clip_id: c.id, reverse: on })))
}

export function freezeFrame() {
  const p = project()
  const s = selected()
  if (!p || !s) return
  const at = getState().playhead
  if (at < s.clip.start || at >= s.clip.end) return toast('Move the playhead over the selected clip to add a freeze frame')
  exec({ op: 'freeze_frame', clip_id: s.clip.id, at, duration: FREEZE_DURATION })
}

// ---------- Keyframes ----------

const f = (v: number): Value => ({ type: 'float', value: v })

/** Adds keyframes for the transform and opacity at the playhead (Filmora's Keyframing). */
export function addKeyframes() {
  const s = selected()
  if (!s) return
  const c = s.clip
  const at = getState().playhead
  if (at < c.start || at > c.end) return toast('Move the playhead over the selected clip to add a keyframe')
  const local = at - c.start
  const tr = c.transform
  const cmds: Command[] = []
  if (s.track.kind === 'audio') cmds.push({ op: 'add_keyframe', clip_id: c.id, param: 'volume', time: local, value: f(paramValue(c.audio?.gain_db, 0)), interpolation: null })
  else {
    const vals: [string, number][] = [
      ['transform.position_x', paramValue(tr?.position_x, 0)],
      ['transform.position_y', paramValue(tr?.position_y, 0)],
      ['transform.scale_x', paramValue(tr?.scale_x, 1)],
      ['transform.scale_y', paramValue(tr?.scale_y, 1)],
      ['transform.rotation', paramValue(tr?.rotation, 0)],
      ['opacity', paramValue(c.opacity, 1)],
    ]
    for (const [param, v] of vals) cmds.push({ op: 'add_keyframe', clip_id: c.id, param, time: local, value: f(v), interpolation: null })
  }
  run(cmds).then(ok => ok && setState({ propsTab: s.track.kind === 'audio' ? 'audio' : 'video' }))
}

// ---------- Audio / links ----------

export function detachAudio() {
  const p = project()
  if (!p) return
  const clips = selectedClips(p).filter(c => findClip(p, c.id)?.track.kind === 'video' && c.source.type === 'media')
  if (!clips.length) return toast('Select a video clip with audio')
  run(clips.map(c => ({ op: 'detach_audio', clip_id: c.id, track_id: null })))
}

export function linkSelection() {
  const p = project()
  if (!p) return
  const ids = getState().selection
  if (ids.length < 2) return toast('Select a video clip and an audio clip to link them')
  exec({ op: 'link_clips', clip_ids: ids })
}

export function unlinkSelection() {
  const p = project()
  if (!p) return
  const ids = selectedClips(p).filter(c => c.link_group).map(c => c.id)
  if (!ids.length) return toast('The selected clips are not linked')
  exec({ op: 'unlink_clips', clip_ids: ids })
}

// ---------- Markers ----------

/** Adds a marker at the playhead: on the selected clip, or on the timeline. */
export async function addMarker() {
  const p = project()
  if (!p) return
  const at = getState().playhead
  const s = selected()
  const marker: Marker = { id: newId(), time: at, duration: 0, name: '', comment: '', color: MARKER_COLORS[1] }
  if (s && at >= s.clip.start && at <= s.clip.end) await exec({ op: 'add_marker', sequence_id: null, clip_id: s.clip.id, marker: { ...marker, time: at - s.clip.start } })
  else await exec({ op: 'add_marker', sequence_id: null, clip_id: null, marker })
}

export function allMarkers(p: Project): { marker: Marker; clip: Clip | null; at: Ticks }[] {
  const seq = mainSequence(p)
  const out: { marker: Marker; clip: Clip | null; at: Ticks }[] = (seq.markers ?? []).map(m => ({ marker: m, clip: null, at: m.time }))
  for (const t of seq.tracks) for (const c of t.clips) for (const m of c.markers ?? []) out.push({ marker: m, clip: c, at: c.start + m.time })
  return out
}

export function findMarker(p: Project, id: string) {
  return allMarkers(p).find(m => m.marker.id === id) ?? null
}

export function removeAllMarkers() {
  const p = project()
  if (!p) return
  run(allMarkers(p).map(m => ({ op: 'remove_marker', marker_id: m.marker.id })))
}

// ---------- Titles ----------

const quickTitle = {
  text: 'Title',
  font_family: 'Helvetica Neue',
  font_size: 0.09,
  color: [1, 1, 1, 1] as [number, number, number, number],
  position: [0.5, 0.5] as [number, number],
  bold: true,
}

/** Quick Text (T): a 5 s title at the playhead on the first free video track above the main one. */
export async function quickText() {
  const p = project()
  if (!p) return
  const seq = mainSequence(p)
  const at = getState().playhead
  const len = 5 * TICKS_PER_SECOND
  const clip: Clip = { id: newId(), name: 'Title', source: { type: 'title', ...quickTitle }, start: at, end: at + len }
  const videos = seq.tracks.filter(t => t.kind === 'video')
  const free = videos.slice(1).find(t => !t.locked && t.clips.every(c => c.end <= at || c.start >= at + len))
  let trackId = free?.id
  if (!trackId) {
    const before = new Set(seq.tracks.map(t => t.id))
    if (!(await exec({ op: 'add_track', sequence_id: null, kind: 'video', index: videos.length }))) return
    trackId = mainSequence(project()!).tracks.find(t => !before.has(t.id))?.id
  }
  if (trackId && (await exec({ op: 'add_clip', track_id: trackId, clip }))) select([clip.id])
}

// ---------- Durations ----------

/** Sets a clip's duration (trims or extends its end, within its media). */
export function setClipDuration(clipId: string, dur: Ticks) {
  const p = project()
  if (!p) return
  const found = findClip(p, clipId)
  if (!found) return
  const c = found.clip
  const media = c.source.type === 'media' ? p.media.find(m => m.id === (c.source as { media_id: string }).media_id) : null
  let end = c.start + Math.max(frameTicks(p.settings.frame_rate), dur)
  if (media && media.kind !== 'image') end = Math.min(end, c.start + Math.floor((media.duration - (c.source_in ?? 0)) / (c.speed?.rate ?? 1)))
  const next = found.track.clips.find(x => x.start >= c.end && x.id !== c.id)
  if (next) end = Math.min(end, next.start)
  if (end !== c.end) exec({ op: 'trim_clip', clip_id: c.id, start: null, end })
}

// ---------- Transitions ----------

/** Apply Quick Transition (⇧T): the default transition on the cut nearest the playhead. */
export async function applyQuickTransition() {
  const p = project()
  if (!p) return
  const seq = mainSequence(p)
  const at = getState().playhead
  const sel = selected()
  const tracks = sel ? [sel.track] : seq.tracks.filter(t => t.kind === 'video' && !t.locked)
  let best: { track: Track; a: Clip; b: Clip | null; d: number } | null = null
  for (const t of tracks) {
    const c = cutNear(p, t, at, Number.MAX_SAFE_INTEGER)
    if (c && c.b && (!best || Math.abs(c.a.end - at) < best.d)) best = { track: t, a: c.a, b: c.b, d: Math.abs(c.a.end - at) }
  }
  if (!best) return toast('There is no cut between two clips to add a transition to')
  const d = defaultTransition()
  const tr = newTransition(p, best.a, best.b, d.kind, d.duration)
  if (await exec({ op: 'batch', commands: setTransitionCommands(p, best.track, best.a, tr) })) selectTransition(best.a.id)
}

export function deleteAllTransitions() {
  const p = project()
  if (!p) return
  const cmds = allTransitions(p).map(({ clip }) => ({ op: 'set_transition', clip_id: clip.id, transition: null }) as Command)
  if (!cmds.length) return toast('There are no transitions in the timeline')
  run(cmds).then(ok => ok && selectTransition(null))
}

export function removeTransition(clipId: string) {
  const p = project()
  const found = p && findClip(p, clipId)
  if (!p || !found) return
  run(removeTransitionCmds(p, found.track, found.clip)).then(ok => ok && selectTransition(null))
}

export function openDuration() {
  const s = getState()
  if (s.transitionSel) return setUi({ dialog: { kind: 'duration', target: 'transition', id: s.transitionSel } })
  const sel = selected()
  if (sel) setUi({ dialog: { kind: 'duration', target: 'clip', id: sel.clip.id } })
}

