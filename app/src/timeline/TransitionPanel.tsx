// Properties panel for a selected transition (Filmora's "Transitions" tab):
// duration, trimmed-frames switch, alignment (Prefix / Overlap / Postfix),
// Set as Default and Apply to all.

import { useEffect, useState } from 'react'
import { findClip, frameTicks, timecode } from '../core/model'
import type { Command, Project, TransitionAlignment } from '../core/types'
import { exec, openMenu, toast } from '../store'
import { Toggle } from '../ui/controls'
import { IcCaretDown, IcChevronDown } from '../ui/icons'
import { parseTimecode } from './Dialogs'
import { setDuration } from './TransitionView'
import { allTransitions, cuts, maxDuration, newTransition, setDefaultTransition, setTransitionCommands } from './transitions'

const ALIGN: { id: TransitionAlignment; label: string }[] = [
  { id: 'end_at_cut', label: 'Prefix' },
  { id: 'center', label: 'Overlap' },
  { id: 'start_at_cut', label: 'Postfix' },
]

export function TransitionPanel({ project, clipId }: { project: Project; clipId: string }) {
  const found = findClip(project, clipId)
  const tr = found?.clip.transition_out
  const r = project.settings.frame_rate
  const [text, setText] = useState(tr ? timecode(tr.duration, r) : '')
  const [open, setOpen] = useState(true)
  useEffect(() => setText(tr ? timecode(tr.duration, r) : ''), [tr?.duration, r]) // eslint-disable-line react-hooks/exhaustive-deps
  if (!found || !tr) return null
  const { track, clip } = found
  const frame = frameTicks(r)
  const i = track.clips.findIndex(c => c.id === clip.id)
  const next = track.clips[i + 1] && track.clips[i + 1].start - clip.end <= frame ? track.clips[i + 1] : null

  const commitDuration = () => {
    const t = parseTimecode(text, r)
    if (t == null || t <= 0) return setText(timecode(tr.duration, r))
    setDuration(project, track, clip, t)
  }

  const align = (al: TransitionAlignment) => {
    if (al === tr.alignment) return
    if (!next && al !== 'end_at_cut') return toast('A transition at the end of the last clip can only be a prefix')
    const max = maxDuration(clip, next, al)
    if (max <= 0) return
    exec({ op: 'batch', commands: setTransitionCommands(project, track, clip, { ...tr, alignment: al, duration: Math.min(tr.duration, max) }) })
  }

  const applyToAll = (durationOnly: boolean) => {
    const cmds: Command[] = []
    if (durationOnly) {
      for (const { track: t, clip: c } of allTransitions(project)) {
        if (t.kind !== track.kind || !c.transition_out) continue
        const n = t.clips[t.clips.findIndex(x => x.id === c.id) + 1] ?? null
        const d = Math.min(tr.duration, maxDuration(c, n && n.start - c.end <= frame ? n : null, c.transition_out.alignment ?? 'center'))
        if (d > 0) cmds.push(...setTransitionCommands(project, t, c, { ...c.transition_out, duration: d }))
      }
    } else {
      // This transition on every cut of every track of the same kind.
      for (const t of project.sequences.find(s => s.id === project.main_sequence)!.tracks) {
        if (t.kind !== track.kind || t.locked) continue
        for (const { a, b } of cuts(project, t)) {
          const existing = a.transition_out
          const n = newTransition(project, a, b, tr.kind, tr.duration, tr.alignment ?? 'center')
          cmds.push(...setTransitionCommands(project, t, a, { ...n, id: existing?.id ?? n.id, params: tr.params ?? {} }))
        }
      }
    }
    if (cmds.length) exec({ op: 'batch', commands: cmds })
  }

  return (
    <section className="panel properties transition-props">
      <div className="panel-head prop-tabs single">
        <button className="on">Transitions</button>
      </div>
      <div className="prop-scroll tr-props">
        <div className="tr-row">
          <span className="tr-label">Duration</span>
          <input
            className="tr-duration"
            value={text}
            onChange={e => setText(e.target.value)}
            onBlur={commitDuration}
            onKeyDown={e => {
              e.stopPropagation()
              if (e.key === 'Enter') (e.target as HTMLInputElement).blur()
              if (e.key === 'Escape') setText(timecode(tr.duration, r))
            }}
          />
        </div>
        <div className="tr-row">
          <span className="tr-label" title="Jibaru holds the edge frames of each clip during a transition instead of using trimmed source frames">Include Trimmed Frames</span>
          <Toggle on={false} disabled title="Transitions hold the clips' edge frames" />
        </div>
        <button className="tr-more" onClick={() => setOpen(o => !o)}>
          More Settings <IcCaretDown size={10} className={open ? '' : 'rot'} />
        </button>
        {open && (
          <div className="tr-align">
            {ALIGN.map(a => {
              const possible = a.id === 'end_at_cut' || !!next
              return (
                <div key={a.id} className="tr-align-item">
                  <button className={`tr-card ${tr.alignment === a.id || (!tr.alignment && a.id === 'center') ? 'on' : ''}`} disabled={!possible} data-align={a.id} onClick={() => align(a.id)} aria-label={a.label}>
                    <span className={`tr-card-art ${a.id}`}>
                      <i className="clip-a" />
                      <i className="clip-b" />
                      <i className="win" />
                    </span>
                  </button>
                  <span className="tr-card-label">{a.label}</span>
                </div>
              )
            })}
          </div>
        )}
      </div>
      <div className="prop-footer tr-footer">
        <button
          className="btn-gray"
          onClick={() => {
            setDefaultTransition({ kind: tr.kind, duration: tr.duration })
            toast('Set as the default transition')
          }}
        >
          Set as Default
        </button>
        <span className="split-btn">
          <button className="btn-accent" onClick={() => applyToAll(false)}>Apply to all</button>
          <button
            className="btn-accent caret"
            aria-label="Apply options"
            onClick={e => {
              const rect = (e.currentTarget.parentElement as HTMLElement).getBoundingClientRect()
              openMenu(rect.left - 70, rect.top - 52, [{ label: 'Apply Duration to All', action: () => applyToAll(true) }])
            }}
          >
            <IcChevronDown size={12} />
          </button>
        </span>
      </div>
    </section>
  )
}
