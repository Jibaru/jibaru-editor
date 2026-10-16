// Timeline dialogs, laid out like Filmora's: Duration Setting, Marker Editor,
// Crop and Audio Mixer.

import { useEffect, useMemo, useState, type ReactNode } from 'react'
import { findClip, fps, mainSequence, paramValue, timecode } from '../core/model'
import type { Command, Marker, Project, Rational, Ticks, Track } from '../core/types'
import { exec, useEditor } from '../store'
import { IcAudio, IcVideoBadge } from '../ui/icons'
import { findMarker, MARKER_COLORS, setClipDuration } from './edit'
import { markerCss, swatchCss } from './Markers'
import { getUi, setUi, useUi } from './tlstate'
import { setDuration } from './TransitionView'

export function TimelineDialogs() {
  const dialog = useUi(s => s.dialog)
  const project = useEditor(s => s.snap?.project)
  if (!dialog || !project) return null
  const close = () => setUi({ dialog: null })
  if (dialog.kind === 'duration') return <DurationDialog project={project} target={dialog.target} id={dialog.id} onClose={close} />
  if (dialog.kind === 'marker') return <MarkerEditor project={project} id={dialog.id} onClose={close} />
  if (dialog.kind === 'crop') return <CropDialog project={project} id={dialog.id} onClose={close} />
  if (dialog.kind === 'mixer') return <AudioMixer project={project} onClose={close} />
  if (dialog.kind === 'voiceover') return <Voiceover onClose={close} />
  return null
}

function Shell({ title, onClose, children, className = '' }: { title: string; onClose: () => void; children: ReactNode; className?: string }) {
  useEffect(() => {
    const esc = (e: KeyboardEvent) => {
      if (e.key === 'Escape' && getUi().dialog) {
        e.stopPropagation()
        onClose()
      }
    }
    window.addEventListener('keydown', esc, true)
    return () => window.removeEventListener('keydown', esc, true)
  }, [onClose])
  return (
    <div className="jb-dialog-backdrop" onPointerDown={e => e.target === e.currentTarget && onClose()}>
      <div className={`jb-dialog ${className}`} role="dialog" aria-label={title}>
        <header>
          <span className="lights">
            <button className="light close" aria-label="Close" onClick={onClose} />
            <i className="light" />
            <i className="light" />
          </span>
          <h3>{title}</h3>
        </header>
        {children}
      </div>
    </div>
  )
}

/** Parses HH:MM:SS:FF (or fewer fields, right-aligned) into ticks. */
export function parseTimecode(text: string, r: Rational): Ticks | null {
  const parts = text.trim().split(/[:;.]/).map(Number)
  if (!parts.length || parts.some(n => !Number.isFinite(n) || n < 0)) return null
  while (parts.length < 4) parts.unshift(0)
  const [h, m, s, f] = parts.slice(-4)
  const rate = Math.round(fps(r)) || 30
  const frames = ((h * 60 + m) * 60 + s) * rate + f
  return Math.round((frames * r.den * 10_000_000) / r.num)
}

function DurationDialog({ project, target, id, onClose }: { project: Project; target: 'clip' | 'transition'; id: string; onClose: () => void }) {
  const found = findClip(project, id)
  const r = project.settings.frame_rate
  const current = found ? (target === 'transition' ? (found.clip.transition_out?.duration ?? 0) : found.clip.end - found.clip.start) : 0
  const [text, setText] = useState(timecode(current, r))
  const save = () => {
    const t = parseTimecode(text, r)
    if (found && t != null && t > 0) {
      if (target === 'transition') setDuration(project, found.track, found.clip, t)
      else setClipDuration(id, t)
    }
    onClose()
  }
  return (
    <Shell title="Duration Setting" onClose={onClose} className="duration-dialog">
      <div className="dlg-body">
        <label className="dlg-row">
          <span>Input Duration:</span>
          <input
            className="tc-input"
            autoFocus
            value={text}
            onChange={e => setText(e.target.value)}
            onKeyDown={e => {
              e.stopPropagation()
              if (e.key === 'Enter') save()
            }}
          />
        </label>
        <div className="dlg-hint">HR:MIN:SEC:FR</div>
      </div>
      <footer>
        <button className="dlg-btn primary" onClick={save}>Save</button>
        <button className="dlg-btn" onClick={onClose}>Cancel</button>
      </footer>
    </Shell>
  )
}

function MarkerEditor({ project, id, onClose }: { project: Project; id: string; onClose: () => void }) {
  const found = findMarker(project, id)
  const [name, setName] = useState(found?.marker.name ?? '')
  const [comment, setComment] = useState(found?.marker.comment ?? '')
  const [color, setColor] = useState(found?.marker.color ?? MARKER_COLORS[1])
  if (!found) return null
  const ok = () => {
    const m: Marker = { ...found.marker, name, comment, color }
    exec({ op: 'update_marker', marker: m })
    onClose()
  }
  return (
    <Shell title="Marker Editor" onClose={onClose} className="marker-dialog">
      <div className="dlg-body grid">
        <span className="lbl">Name:</span>
        <input className="dlg-input" autoFocus value={name} onChange={e => setName(e.target.value)} onKeyDown={e => e.stopPropagation()} />
        <span className="lbl">Time:</span>
        <span className="val">{timecode(found.at, project.settings.frame_rate)}</span>
        <span className="lbl top">Comment:</span>
        <textarea className="dlg-input area" placeholder="Add your comments here" value={comment} onChange={e => setComment(e.target.value)} onKeyDown={e => e.stopPropagation()} />
        <span className="lbl">Marker Color:</span>
        <span className="swatches">
          {MARKER_COLORS.map((c, i) => (
            <button key={i} className={`swatch ${markerCss(c) === markerCss(color) ? 'on' : ''}`} style={{ background: swatchCss(c) }} aria-label={`Color ${i + 1}`} onClick={() => setColor(c)} />
          ))}
        </span>
      </div>
      <footer className="split">
        <button
          className="dlg-btn danger"
          onClick={() => {
            exec({ op: 'remove_marker', marker_id: id })
            onClose()
          }}
        >
          Delete
        </button>
        <button className="dlg-btn primary" onClick={ok}>OK</button>
      </footer>
    </Shell>
  )
}

const RATIOS: { label: string; value: number | null }[] = [
  { label: 'Custom', value: null },
  { label: '16:9', value: 16 / 9 },
  { label: '4:3', value: 4 / 3 },
  { label: '1:1', value: 1 },
  { label: '9:16', value: 9 / 16 },
]

function CropDialog({ project, id, onClose }: { project: Project; id: string; onClose: () => void }) {
  const found = findClip(project, id)
  const media = found && found.clip.source.type === 'media' ? project.media.find(m => m.id === (found.clip.source as { media_id: string }).media_id) : null
  const [crop, setCrop] = useState<[number, number, number, number]>(found?.clip.transform?.crop ?? [0, 0, 0, 0])
  const aspect = media?.width && media.height ? media.width / media.height : 16 / 9
  if (!found) return null
  const set = (i: number, v: number) => setCrop(c => c.map((x, j) => (j === i ? Math.min(0.95, Math.max(0, v / 100)) : x)) as [number, number, number, number])
  const ratio = (r: number) => {
    // Largest centered rectangle of ratio r inside the source.
    if (r >= aspect) {
      const h = aspect / r
      setCrop([0, (1 - h) / 2, 0, (1 - h) / 2])
    } else {
      const w = r / aspect
      setCrop([(1 - w) / 2, 0, (1 - w) / 2, 0])
    }
  }
  const [l, t, rr, b] = crop
  return (
    <Shell title="Crop and Zoom" onClose={onClose} className="crop-dialog">
      <div className="dlg-body">
        <div className="crop-preview" style={{ aspectRatio: String(aspect) }}>
          <div className="crop-rect" style={{ left: `${l * 100}%`, top: `${t * 100}%`, right: `${rr * 100}%`, bottom: `${b * 100}%` }} />
        </div>
        <div className="crop-fields">
          {(['Left', 'Top', 'Right', 'Bottom'] as const).map((name, i) => (
            <label key={name} className="dlg-row">
              <span>{name}</span>
              <input className="dlg-input num" type="number" min={0} max={95} step={1} value={Math.round(crop[i] * 1000) / 10} onChange={e => set(i, Number(e.target.value))} onKeyDown={e => e.stopPropagation()} />
              <span className="unit">%</span>
            </label>
          ))}
        </div>
        <div className="crop-ratios">
          <span>Aspect Ratio:</span>
          {RATIOS.map(r => (
            <button key={r.label} className="dlg-chip" onClick={() => (r.value ? ratio(r.value) : setCrop([0, 0, 0, 0]))}>
              {r.label}
            </button>
          ))}
        </div>
      </div>
      <footer className="split">
        <button className="dlg-btn" onClick={() => setCrop([0, 0, 0, 0])}>Reset</button>
        <span>
          <button
            className="dlg-btn primary"
            onClick={() => {
              exec({ op: 'set_crop', clip_id: id, crop })
              onClose()
            }}
          >
            OK
          </button>
          <button className="dlg-btn" onClick={onClose}>Cancel</button>
        </span>
      </footer>
    </Shell>
  )
}

/**
 * Audio Mixer: one strip per track with pan and gain. Applying adds the
 * strip's gain to every clip on the track and sets their pan (jb-core has
 * no track-level volume, so the mix is written to the clips).
 */
function AudioMixer({ project, onClose }: { project: Project; onClose: () => void }) {
  const tracks = useMemo(() => {
    const seq = mainSequence(project)
    return [...seq.tracks.filter(t => t.kind === 'video').reverse(), ...seq.tracks.filter(t => t.kind === 'audio')].filter(t => t.clips.some(c => c.audio))
  }, [project])
  const [mix, setMix] = useState<Record<string, { gain: number; pan: number }>>(() => Object.fromEntries(tracks.map(t => [t.id, { gain: 0, pan: avgPan(t) }])))
  const [master, setMaster] = useState(0)
  const apply = () => {
    const cmds: Command[] = []
    for (const t of tracks) {
      const m = mix[t.id]
      if (!m) continue
      for (const c of t.clips) {
        if (!c.audio) continue
        const g = m.gain + master
        if (g) cmds.push({ op: 'set_volume', clip_id: c.id, gain_db: Math.max(-60, Math.min(20, paramValue(c.audio.gain_db, 0) + g)) })
        if (Math.abs(m.pan - paramValue(c.audio.pan, 0)) > 1e-3) cmds.push({ op: 'set_pan', clip_id: c.id, pan: m.pan })
      }
    }
    if (cmds.length) exec({ op: 'batch', commands: cmds })
    onClose()
  }
  const strip = (id: string, label: string, kind: 'video' | 'audio') => {
    const m = mix[id]
    const set = (patch: Partial<{ gain: number; pan: number }>) => setMix(x => ({ ...x, [id]: { ...x[id], ...patch } }))
    return (
      <div className="mix-strip" key={id}>
        <span className="mix-icon">{kind === 'video' ? <IcVideoBadge size={15} /> : <IcAudio size={15} />}</span>
        <span className="mix-name">{label}</span>
        <Knob value={m.pan} onChange={pan => set({ pan })} />
        <input className="dlg-input mix-num" value={m.pan.toFixed(2)} onChange={e => set({ pan: clampNum(e.target.value, -1, 1, m.pan) })} onKeyDown={e => e.stopPropagation()} />
        <Fader value={m.gain} onChange={gain => set({ gain })} />
        <span className="mix-db">
          <input className="dlg-input mix-num" value={m.gain.toFixed(2)} onChange={e => set({ gain: clampNum(e.target.value, -60, 20, m.gain) })} onKeyDown={e => e.stopPropagation()} /> dB
        </span>
      </div>
    )
  }
  return (
    <Shell title="Audio Mixer" onClose={onClose} className="mixer-dialog">
      <div className="mixer-body">
        <div className="mix-strips">{tracks.map(t => strip(t.id, t.name, t.kind))}</div>
        <div className="mix-master">
          <span className="mix-name">Master</span>
          <Fader value={master} onChange={setMaster} />
          <span className="mix-db">
            <input className="dlg-input mix-num" value={master.toFixed(2)} onChange={e => setMaster(clampNum(e.target.value, -60, 20, master))} onKeyDown={e => e.stopPropagation()} /> dB
          </span>
        </div>
      </div>
      <footer>
        <button className="dlg-btn primary" onClick={apply}>Apply</button>
        <button className="dlg-btn" onClick={onClose}>Cancel</button>
      </footer>
    </Shell>
  )
}

const clampNum = (s: string, lo: number, hi: number, fallback: number) => {
  const v = Number(s)
  return Number.isFinite(v) ? Math.min(hi, Math.max(lo, v)) : fallback
}

function avgPan(t: Track) {
  const ps = t.clips.filter(c => c.audio).map(c => paramValue(c.audio!.pan, 0))
  return ps.length ? ps.reduce((a, b) => a + b, 0) / ps.length : 0
}

function Knob({ value, onChange }: { value: number; onChange: (v: number) => void }) {
  return (
    <div
      className="mix-knob"
      style={{ ['--rot' as string]: `${value * 135}deg` }}
      onPointerDown={e => {
        const y0 = e.clientY
        const v0 = value
        const mv = (ev: PointerEvent) => onChange(Math.max(-1, Math.min(1, v0 + (y0 - ev.clientY) / 80)))
        const up = () => {
          window.removeEventListener('pointermove', mv)
          window.removeEventListener('pointerup', up)
        }
        window.addEventListener('pointermove', mv)
        window.addEventListener('pointerup', up)
      }}
      onDoubleClick={() => onChange(0)}
    >
      <i />
      <span className="lr"><b>L</b><b>R</b></span>
    </div>
  )
}

/** Vertical gain fader, -60..+20 dB. */
function Fader({ value, onChange }: { value: number; onChange: (v: number) => void }) {
  const k = (value + 60) / 80
  return (
    <div
      className="mix-fader"
      onPointerDown={e => {
        const el = e.currentTarget
        const set = (y: number) => {
          const r = el.getBoundingClientRect()
          const t = 1 - Math.min(1, Math.max(0, (y - r.top) / r.height))
          onChange(Math.round((t * 80 - 60) * 10) / 10)
        }
        set(e.clientY)
        const mv = (ev: PointerEvent) => set(ev.clientY)
        const up = () => {
          window.removeEventListener('pointermove', mv)
          window.removeEventListener('pointerup', up)
        }
        window.addEventListener('pointermove', mv)
        window.addEventListener('pointerup', up)
      }}
      onDoubleClick={() => onChange(0)}
    >
      <div className="mix-scale">{[20, 6, 0, -6, -12, -18, -30, -42, -60].map(d => <span key={d} style={{ bottom: `${((d + 60) / 80) * 100}%` }}>{d}</span>)}</div>
      <div className="mix-track" />
      <div className="mix-handle" style={{ bottom: `${k * 100}%` }} />
    </div>
  )
}

function Voiceover({ onClose }: { onClose: () => void }) {
  return (
    <Shell title="Record Voiceover" onClose={onClose} className="duration-dialog">
      <div className="dlg-body">
        <div className="dlg-hint wide">Recording a voiceover needs microphone access, which this build of Jibaru Editor does not have yet. Record with another app and import the file.</div>
      </div>
      <footer>
        <button className="dlg-btn primary" onClick={onClose}>OK</button>
      </footer>
    </Shell>
  )
}
