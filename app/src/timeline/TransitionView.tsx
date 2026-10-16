// A transition on the timeline, drawn like Filmora: a rounded box over the
// thumbnail strip spanning the transition window, the thumbnails inside it
// dimmed, and the transition's sample image centered on the cut. Hover shows
// a light border and a tooltip; selected shows the accent border. Dragging
// either edge changes the duration.

import { useRef, useState, type MouseEvent as RME, type PointerEvent as RPE } from 'react'
import { findClip, frameTicks, timecode } from '../core/model'
import type { Clip, Command, Project, Ticks, Track } from '../core/types'
import { catalogLabel } from '../library'
import { exec, getState, openMenu, selectTransition, setState, useEditor } from '../store'
import { sep, type MenuItem } from '../ui/ContextMenu'
import { applyQuickTransition, deleteAllTransitions, removeTransition } from './edit'
import { tx, xt } from './geometry'
import { setUi } from './tlstate'
import { TipBox } from './Tooltip'
import { audioPartners, frameRound, maxDuration, minDuration, setTransitionCommands, transitionWindow } from './transitions'

export const TRANSITION_ICON_W = 28

interface Props {
  project: Project
  track: Track
  clip: Clip
  next: Clip | null
  pps: number
  /** Box top and height inside the lane. */
  top: number
  height: number
}

/** Clip copy/paste for transitions (Cut / Copy in the transition menu). */
let transitionClipboard: { kind: string; duration: Ticks } | null = null
export const copiedTransition = () => transitionClipboard

export function TransitionView({ project, track, clip, next, pps, top, height }: Props) {
  const tr = clip.transition_out!
  const selected = useEditor(s => s.transitionSel === clip.id)
  const [drag, setDrag] = useState<Ticks | null>(null)
  const [tip, setTip] = useState<{ x: number; y: number } | null>(null)
  const timer = useRef<number | undefined>(undefined)
  const duration = drag ?? tr.duration
  const [from, to] = transitionWindow({ duration, alignment: tr.alignment }, clip.end)
  const left = tx(from, pps)
  const width = Math.max(4, tx(to, pps) - left)
  const cutX = tx(clip.end, pps) - left
  const audio = track.kind === 'audio'
  const partners = audioPartners(project, track, clip)
  const hasAudioFade = partners.some(c => c.transition_out)

  const hideTip = () => {
    window.clearTimeout(timer.current)
    setTip(null)
  }

  const down = (e: RPE) => {
    if (e.button !== 0) return
    e.stopPropagation()
    hideTip()
    selectTransition(clip.id)
    if (track.locked) return
    const r = e.currentTarget.getBoundingClientRect()
    const x = e.clientX - r.left
    const edge = x < 7 ? 'start' : x > r.width - 7 ? 'end' : null
    if (!edge) return
    // Which edges move: centered windows grow on both sides; aligned ones only away from the cut.
    const al = tr.alignment ?? 'center'
    if ((edge === 'start' && al === 'start_at_cut') || (edge === 'end' && al === 'end_at_cut')) return
    const k = al === 'center' ? 2 : 1
    const sign = edge === 'start' ? -1 : 1
    const x0 = e.clientX
    const lo = minDuration(project)
    const hi = Math.max(lo, maxDuration(clip, next, al))
    let cur = tr.duration
    const mv = (ev: PointerEvent) => {
      const dt = xt(ev.clientX - x0, getState().zoom) * sign * k
      cur = frameRound(project, Math.min(hi, Math.max(lo, tr.duration + dt)))
      setDrag(cur)
    }
    const up = () => {
      window.removeEventListener('pointermove', mv)
      window.removeEventListener('pointerup', up)
      setDrag(null)
      if (cur !== tr.duration) setDuration(project, track, clip, cur)
    }
    window.addEventListener('pointermove', mv)
    window.addEventListener('pointerup', up)
  }

  const menu = (e: RME) => {
    e.preventDefault()
    e.stopPropagation()
    hideTip()
    selectTransition(clip.id)
    openMenu(e.clientX, e.clientY, transitionMenu(clip, hasAudioFade, partners.length > 0))
  }

  const name = audio ? 'Audio Transitions-Constant Gain' : `Transitions-${transitionName(tr)}`
  const lines = [...(hasAudioFade ? ['Audio Transitions-Constant Gain'] : []), `Duration: ${timecode(duration, project.settings.frame_rate)}`]

  return (
    <div
      className={`transition-box ${audio ? 'audio' : ''} ${selected ? 'sel' : ''} ${drag != null ? 'dragging' : ''}`}
      data-transition={clip.id}
      style={{ left, width, top, height }}
      onPointerDown={down}
      onDoubleClick={e => {
        e.stopPropagation()
        selectTransition(clip.id)
      }}
      onContextMenu={menu}
      onMouseMove={e => {
        const x = e.clientX
        const y = e.clientY
        window.clearTimeout(timer.current)
        if (tip) setTip({ x: x + 14, y: y + 18 })
        else timer.current = window.setTimeout(() => setTip({ x: x + 14, y: y + 18 }), 500)
      }}
      onMouseLeave={hideTip}
    >
      {!audio && (
        <span className="transition-thumb" style={{ left: cutX - TRANSITION_ICON_W / 2, width: TRANSITION_ICON_W }}>
          <span className={`transition-sample tr-${tr.kind}`}>
            <i className="a" />
            <i className="b" />
          </span>
        </span>
      )}
      {audio && <svg className="crossfade" viewBox="0 0 100 100" preserveAspectRatio="none"><path d="M0 8 L100 92 M0 92 L100 8" /></svg>}
      {tip && <TipBox tip={{ label: name, lines }} x={tip.x} y={tip.y} anchor="cursor" />}
    </div>
  )
}

function run(cmds: Command[]) {
  return exec(cmds.length === 1 ? cmds[0] : { op: 'batch', commands: cmds })
}

export function setDuration(project: Project, track: Track, clip: Clip, duration: Ticks) {
  const tr = clip.transition_out
  if (!tr) return
  const frame = frameTicks(project.settings.frame_rate)
  const i = track.clips.findIndex(c => c.id === clip.id)
  const next = track.clips[i + 1] && track.clips[i + 1].start - clip.end <= frame ? track.clips[i + 1] : null
  const d = frameRound(project, Math.min(Math.max(minDuration(project), duration), Math.max(minDuration(project), maxDuration(clip, next, tr.alignment ?? 'center'))))
  return run(setTransitionCommands(project, track, clip, { ...tr, duration: d }))
}

function transitionMenu(clip: Clip, hasAudioFade: boolean, canAudio: boolean): MenuItem[] {
  const tr = clip.transition_out!
  const snapping = getState().snapping
  return [
    {
      label: 'Cut',
      shortcut: '⌘X',
      action: () => {
        transitionClipboard = { kind: tr.kind, duration: tr.duration }
        removeTransition(clip.id)
      },
    },
    { label: 'Copy', shortcut: '⌘C', action: () => void (transitionClipboard = { kind: tr.kind, duration: tr.duration }) },
    { label: 'Delete', shortcut: '⌫', action: () => removeTransition(clip.id) },
    { label: 'Delete All Transitions', action: deleteAllTransitions },
    sep,
    { label: 'Apply Quick Transition', shortcut: '⇧T', action: applyQuickTransition },
    sep,
    { label: 'Duration', action: () => setUi({ dialog: { kind: 'duration', target: 'transition', id: clip.id } }) },
    sep,
    {
      label: hasAudioFade ? 'Disable Audio Transition' : 'Enable Audio Transition',
      disabled: !canAudio,
      action: () => {
        const p = getState().snap!.project
        const f = findClip(p, clip.id)
        if (!f) return
        if (hasAudioFade) run(audioPartners(p, f.track, f.clip).filter(c => c.transition_out).map(c => ({ op: 'set_transition', clip_id: c.id, transition: null })))
        else run(setTransitionCommands(p, f.track, f.clip, f.clip.transition_out!).slice(1))
      },
    },
    sep,
    { label: 'Locate in the Resource Panel', action: () => setState({ libraryTab: 'transitions' }) },
    { label: 'Find Similar', action: () => setState({ libraryTab: 'transitions' }) },
    sep,
    { label: 'Enable Timeline Snapping', shortcut: 'N', checked: snapping, action: () => setState({ snapping: !snapping }) },
  ]
}

/** Transitions to draw on a track, with the clip after each cut. */
export function trackTransitions(project: Project, track: Track): { clip: Clip; next: Clip | null }[] {
  const frame = frameTicks(project.settings.frame_rate)
  const out: { clip: Clip; next: Clip | null }[] = []
  track.clips.forEach((c, i) => {
    if (!c.transition_out) return
    const b = track.clips[i + 1]
    out.push({ clip: c, next: b && b.start - c.end <= frame ? b : null })
  })
  return out
}


/** Display name: Filmora's own name for imported transitions, else the catalog's. */
export function transitionName(tr: { kind: string; extra?: unknown }): string {
  const f = (tr.extra as { filmora?: { display?: string | null } } | undefined)?.filmora
  return f?.display || catalogLabel(tr.kind)
}
