// Speed tab: uniform speed (rate, duration, reverse, ripple, maintain pitch)
// and speed ramping (Filmora's presets plus a custom curve editor).

import { useEffect, useRef, useState } from 'react'
import { fps, seconds, sourceLength, timecode } from '../../core/model'
import { CUSTOM_RAMP, RAMP_PRESETS, rampSpeed } from '../../core/presets'
import type { Clip, Command, Project, SpeedPoint, Track } from '../../core/types'
import { exec } from '../../store'
import { NumberField, Select, Slider, Toggle } from '../../ui/controls'
import { IcHelp, IcNone } from '../../ui/icons'
import { SubTabs, useDraft } from './common'

// The speed slider is logarithmic around 1x, like Filmora's (0.01x to 100x).
const speedToK = (r: number) => (Math.log10(r) + 2) / 4
const kToSpeed = (k: number) => Math.round(Math.pow(10, k * 4 - 2) * 100) / 100

const RIPPLE_KEY = 'jb.speed.ripple'
const storedRipple = () => {
  try {
    return localStorage.getItem(RIPPLE_KEY) !== '0'
  } catch {
    return true
  }
}

export function SpeedTab({ clip, track, project }: { clip: Clip; track: Track; project: Project }) {
  const ramped = !!clip.speed?.curve
  const [sub, setSub] = useState(ramped ? 'Speed Ramping' : 'Uniform Speed')
  const [ripple, setRippleState] = useState(storedRipple)
  const setRipple = (v: boolean) => {
    setRippleState(v)
    try {
      localStorage.setItem(RIPPLE_KEY, v ? '1' : '0')
    } catch {
      /* storage unavailable */
    }
  }
  const rate = clip.speed?.curve ? 1 : (clip.speed?.rate ?? 1)
  const [k, setK] = useDraft(speedToK(rate))
  const fr = project.settings.frame_rate
  const [dur, setDur] = useState<string | null>(null)
  const frozen = !!clip.speed?.freeze

  /** Wraps a retiming command so later clips move with the new length (Ripple Edit). */
  const retime = (cmd: Command, newLen: number) => {
    const delta = clip.start + newLen - clip.end
    const cmds: Command[] = []
    const later = track.clips.filter(c => c.start >= clip.end && c.id !== clip.id).sort((a, b) => (delta > 0 ? b.start - a.start : a.start - b.start))
    if (ripple && delta > 0) for (const c of later) cmds.push({ op: 'move_clip', clip_id: c.id, start: c.start + delta, track_id: null })
    cmds.push(cmd)
    if (ripple && delta < 0) for (const c of later) cmds.push({ op: 'move_clip', clip_id: c.id, start: c.start + delta, track_id: null })
    exec(cmds.length === 1 ? cmds[0] : { op: 'batch', commands: cmds })
  }
  const span = () => sourceSpan(clip)
  const apply = (r: number) => retime({ op: 'set_speed', clip_id: clip.id, rate: r, reverse: null }, Math.round(span() / r))
  const ramp = (preset: string, points?: SpeedPoint[]) => {
    const pts = points ?? RAMP_PRESETS.find(p => p.id === preset)?.points
    const len = pts ? Math.round(span() * stretch(pts)) : span()
    retime({ op: 'set_speed_ramp', clip_id: clip.id, preset, points: points ?? null }, len)
  }

  if (frozen) return <div className="prop-scroll"><p className="mask-hint">Freeze frames have no speed settings.</p></div>
  return (
    <>
      <SubTabs tabs={['Uniform Speed', 'Speed Ramping']} value={sub} onChange={setSub} />
      <div className="prop-scroll">
        {sub === 'Uniform Speed' ? (
          <div className="plain">
            <div className="prow slider-row">
              <span className="plabel">Speed</span>
              <Slider value={k} min={0} max={1} step={0.001} onInput={setK} onCommit={v => apply(kToSpeed(v))} />
              <NumberField className="pvalue" value={kToSpeed(k)} onCommit={v => apply(Math.max(0.01, Math.min(100, v)))} />
              <span className="punit">x</span>
              <span className="pend" />
            </div>
            <div className="prow">
              <span className="plabel">Duration</span>
              <input
                className="field wide-field"
                value={dur ?? timecode(clip.end - clip.start, fr)}
                onChange={e => setDur(e.target.value)}
                onKeyDown={e => {
                  e.stopPropagation()
                  if (e.key === 'Enter') (e.target as HTMLInputElement).blur()
                }}
                onBlur={() => {
                  if (dur == null) return
                  const parts = dur.split(':').map(Number)
                  setDur(null)
                  if (parts.length !== 4 || parts.some(n => !Number.isFinite(n))) return
                  const secs = parts[0] * 3600 + parts[1] * 60 + parts[2] + parts[3] / fps(fr)
                  if (secs > 0) apply(Math.max(0.01, Math.min(100, seconds(span()) / secs)))
                }}
              />
            </div>
            <div className="prow">
              <span className="plabel">Reverse Speed</span>
              <span className="spacer" />
              <Toggle on={!!clip.speed?.reverse} onChange={v => exec({ op: 'set_reverse', clip_id: clip.id, reverse: v })} />
            </div>
            <div className="prow">
              <span className="plabel">Ripple Edit</span>
              <span className="spacer" />
              <Toggle on={ripple} onChange={setRipple} title="Move later clips when the duration changes" />
            </div>
            <div className="prow">
              <span className="plabel">Maintain Pitch</span>
              <span className="spacer" />
              <Toggle on={clip.speed?.maintain_pitch ?? true} onChange={v => exec({ op: 'set_maintain_pitch', clip_id: clip.id, maintain_pitch: v })} />
            </div>
            <div className="prow select-row">
              <span className="plabel">
                <span className="ellipsis">Frame Interpolation</span> <IcHelp size={13} />
              </span>
              <Select
                value="sampling"
                options={[
                  { value: 'sampling', label: 'Frame Sampling' },
                  { value: 'blending', label: 'Frame Blending', disabled: true, title: 'Not available yet' },
                  { value: 'flow', label: 'Optical Flow', disabled: true, title: 'AI frame interpolation is not available' },
                ]}
                onChange={() => {}}
              />
            </div>
          </div>
        ) : (
          <RampPane clip={clip} ramp={ramp} />
        )}
      </div>
    </>
  )
}

/** Source ticks a clip consumes (from its ramp when it has one). */
function sourceSpan(clip: Clip): number {
  const pts = clip.speed?.curve?.points
  if (!pts) return sourceLength(clip)
  return Math.round((clip.end - clip.start) / stretch(pts))
}

function stretch(points: SpeedPoint[]): number {
  const n = 400
  let sum = 0
  for (let i = 0; i < n; i++) sum += 1 / Math.max(0.01, rampSpeed(points, (i + 0.5) / n))
  return sum / n
}

function RampPane({ clip, ramp }: { clip: Clip; ramp: (preset: string, points?: SpeedPoint[]) => void }) {
  const curve = clip.speed?.curve
  const active = !curve ? 'none' : RAMP_PRESETS.some(p => p.id === curve.preset) ? curve.preset : 'custom'
  const [editing, setEditing] = useState(active === 'custom')
  const items = [{ id: 'none', label: 'None' }, { id: 'custom', label: 'Custom' }, ...RAMP_PRESETS]
  return (
    <>
      <div className="preset-grid">
        {items.map(it => (
          <button
            key={it.id}
            className={`preset ${active === it.id ? 'on' : ''}`}
            data-ramp={it.id}
            onClick={() => {
              if (it.id === 'custom') {
                setEditing(true)
                if (active !== 'custom') ramp('custom', curve?.points ?? CUSTOM_RAMP)
              } else {
                setEditing(false)
                ramp(it.id)
              }
            }}
          >
            <div className="preset-thumb">{it.id === 'none' ? <IcNone size={34} strokeWidth={1} /> : <RampCurve points={'points' in it ? it.points : (curve?.points ?? CUSTOM_RAMP)} />}</div>
            <span>{it.label}</span>
          </button>
        ))}
      </div>
      {curve && (editing || active === 'custom') && <RampEditor points={curve.points} onCommit={pts => ramp('custom', pts)} />}
      {curve && <p className="mask-hint">Duration {seconds(clip.end - clip.start).toFixed(2)} s. Drag points up or down; double-click to add a point, right-click to remove one.</p>}
    </>
  )
}

const sy = (s: number) => 30 - (Math.log10(Math.max(0.1, Math.min(10, s))) + 1) * 14

function RampCurve({ points }: { points: SpeedPoint[] }) {
  const pts = points.map(p => `${2 + p.position * 60},${sy(p.speed) * 0.85 + 2}`).join(' ')
  return (
    <svg viewBox="0 0 64 34" width="56" height="30">
      <path d="M2 17H62" stroke="#3b4248" strokeDasharray="2 2" />
      <polyline points={pts} fill="none" stroke="var(--accent)" strokeWidth="1.5" />
    </svg>
  )
}

/** Custom ramp editor: speed (0.1x..10x, log scale) over the clip's source range. */
function RampEditor({ points, onCommit }: { points: SpeedPoint[]; onCommit: (p: SpeedPoint[]) => void }) {
  const [pts, setPts] = useState(points)
  const ref = useRef<SVGSVGElement>(null)
  useEffect(() => setPts(points), [JSON.stringify(points)])
  const W = 300
  const H = 120
  const x = (pos: number) => 10 + pos * (W - 20)
  const y = (s: number) => 10 + (1 - (Math.log10(Math.max(0.1, Math.min(10, s))) + 1) / 2) * (H - 20)
  const at = (cx: number, cy: number) => {
    const r = ref.current!.getBoundingClientRect()
    const pos = Math.max(0, Math.min(1, ((cx - r.left) * (W / r.width) - 10) / (W - 20)))
    const v = 1 - ((cy - r.top) * (H / r.height) - 10) / (H - 20)
    const speed = Math.round(Math.pow(10, Math.max(0, Math.min(1, v)) * 2 - 1) * 100) / 100
    return { pos, speed }
  }
  return (
    <div className="ramp-editor">
      <svg
        ref={ref}
        viewBox={`0 0 ${W} ${H}`}
        className="ramp-svg"
        onDoubleClick={e => {
          const { pos, speed } = at(e.clientX, e.clientY)
          const next = [...pts, { position: pos, speed }].sort((a, b) => a.position - b.position)
          setPts(next)
          onCommit(next)
        }}
      >
        {[10, 1, 0.1].map(s => (
          <g key={s}>
            <line x1={10} x2={W - 10} y1={y(s)} y2={y(s)} stroke="#2f363c" strokeDasharray={s === 1 ? '3 3' : undefined} />
            <text x={W - 8} y={y(s) + 3} fontSize="8" fill="#7c8389" textAnchor="end">{s}x</text>
          </g>
        ))}
        <polyline points={pts.map(p => `${x(p.position)},${y(p.speed)}`).join(' ')} fill="none" stroke="var(--accent)" strokeWidth="1.5" />
        {pts.map((p, i) => (
          <circle
            key={i}
            cx={x(p.position)}
            cy={y(p.speed)}
            r={5}
            className="ramp-point"
            data-point={i}
            onContextMenu={e => {
              e.preventDefault()
              if (pts.length <= 2) return
              const next = pts.filter((_, j) => j !== i)
              setPts(next)
              onCommit(next)
            }}
            onPointerDown={e => {
              if (e.button !== 0) return
              e.stopPropagation()
              let cur = pts
              const edge = i === 0 || i === pts.length - 1
              const mv = (ev: PointerEvent) => {
                const { pos, speed } = at(ev.clientX, ev.clientY)
                const lo = i > 0 ? pts[i - 1].position + 0.01 : 0
                const hi = i < pts.length - 1 ? pts[i + 1].position - 0.01 : 1
                cur = pts.map((q, j) => (j === i ? { position: edge ? q.position : Math.max(lo, Math.min(hi, pos)), speed } : q))
                setPts(cur)
              }
              const up = () => {
                window.removeEventListener('pointermove', mv)
                window.removeEventListener('pointerup', up)
                if (cur !== pts) onCommit(cur)
              }
              window.addEventListener('pointermove', mv)
              window.addEventListener('pointerup', up)
            }}
          />
        ))}
      </svg>
    </div>
  )
}
