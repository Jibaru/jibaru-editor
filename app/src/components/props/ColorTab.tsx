// Color tab: presets (built-in and saved custom), LUT, white balance, light,
// HSL, curves and color wheels. Each section is one jb-render color effect
// (`jb.color.adjust`, `jb.color.lut`, `jb.color.hsl`, `jb.color.curves`,
// `jb.color.wheels`) edited with set_effect_params.

import { useEffect, useRef, useState } from 'react'
import { backend } from '../../backend'
import { seconds } from '../../core/model'
import type { Clip, Command, EffectInstance, MediaAsset, Param } from '../../core/types'
import { addEffectCommand, effectOn, effectValue, findEffect, fv, moduleIsOff, moduleOff, moduleOn, setEffectParams, switchEffect } from '../../edit'
import { exec, getState, setState, toast, useEditor } from '../../store'
import { Module, Pills, Slider } from '../../ui/controls'
import { IcChevronDown, IcEyedropper, IcNone, IcReset } from '../../ui/icons'
import { EffectSlider, SubTabs, text, textValue, useDraft } from './common'

export const COLOR_KIND = 'jb.color.adjust'
export const colorEffect = (clip: Clip) => findEffect(clip, COLOR_KIND)
const COLOR_KINDS = ['jb.color.adjust', 'jb.color.lut', 'jb.color.hsl', 'jb.color.curves', 'jb.color.wheels']
export const isColorEffect = (e: EffectInstance) => COLOR_KINDS.includes(e.kind)

const PRESETS: { name: string; filter: string; params: Record<string, number> }[] = [
  { name: 'None', filter: '', params: {} },
  { name: 'TV', filter: 'saturate(0.7) contrast(1.15) sepia(0.15)', params: { saturation: -30, contrast: 15 } },
  { name: 'Boost Color', filter: 'saturate(1.6)', params: { saturation: 60, vibrance: 30 } },
  { name: 'Darken', filter: 'brightness(0.75) contrast(1.1)', params: { brightness: -25, contrast: 10 } },
  { name: 'Warm', filter: 'sepia(0.3) saturate(1.2)', params: { temperature: 30 } },
  { name: 'Cool', filter: 'hue-rotate(-12deg) saturate(1.1)', params: { temperature: -30 } },
  { name: 'Faded', filter: 'contrast(0.8) brightness(1.1) saturate(0.8)', params: { contrast: -25, blacks: 25, saturation: -15 } },
  { name: 'Vivid', filter: 'saturate(1.4) contrast(1.15)', params: { vibrance: 40, contrast: 15, saturation: 15 } },
  { name: 'Black & White', filter: 'grayscale(1)', params: { saturation: -100 } },
  { name: 'Bright', filter: 'brightness(1.2)', params: { exposure: 15, shadows: 20 } },
]

const COLOR_KEYS = ['temperature', 'tint', 'vibrance', 'saturation']
const LIGHT_KEYS = ['exposure', 'brightness', 'contrast', 'highlights', 'shadows', 'whites', 'blacks']
const ADJUST_KEYS = ['exposure', 'brightness', 'contrast', 'saturation', 'vibrance', 'temperature', 'tint', 'highlights', 'shadows', 'whites', 'blacks', 'vignette']

// ---------- Custom presets (per user, kept in local storage) ----------

type CustomPreset = { name: string; effects: { kind: string; params: Record<string, Param> }[] }
const CUSTOM_KEY = 'jb.color.custom'

function loadCustom(): CustomPreset[] {
  try {
    return JSON.parse(localStorage.getItem(CUSTOM_KEY) ?? '[]') as CustomPreset[]
  } catch {
    return []
  }
}

function saveCustom(list: CustomPreset[]) {
  try {
    localStorage.setItem(CUSTOM_KEY, JSON.stringify(list))
  } catch {
    toast('Could not save the preset (storage unavailable)', 'error')
  }
  window.dispatchEvent(new Event('jb-color-presets'))
}

function useCustomPresets(): CustomPreset[] {
  const [list, setList] = useState(loadCustom)
  useEffect(() => {
    const f = () => setList(loadCustom())
    window.addEventListener('jb-color-presets', f)
    return () => window.removeEventListener('jb-color-presets', f)
  }, [])
  return list
}

/** Saves the clip's color effects as a named custom preset ("Save as Custom"). */
export function saveColorPreset(clip: Clip, name: string) {
  const effects = (clip.effects ?? []).filter(isColorEffect).map(e => ({ kind: e.kind, params: structuredClone(e.params ?? {}) }))
  if (!effects.length) return toast('Adjust the color first, then save it as a preset')
  const list = loadCustom().filter(p => p.name !== name)
  saveCustom([...list, { name, effects }])
  toast(`Saved color preset "${name}"`)
}

/** Replaces the clip's color effects with a preset's (one undo step). */
function applyCustom(clip: Clip, preset: CustomPreset) {
  const cmds: Command[] = (clip.effects ?? []).filter(isColorEffect).map(e => ({ op: 'remove_effect', clip_id: clip.id, effect_id: e.id }))
  for (const e of preset.effects) cmds.push(addEffectCommand(clip, e.kind).cmd)
  // addEffectCommand builds empty params; put the saved ones in.
  for (const c of cmds) if (c.op === 'add_effect') c.effect.params = structuredClone(preset.effects.find(e => e.kind === c.effect.kind)?.params ?? {})
  exec({ op: 'batch', commands: cmds })
}

export function ColorTab({ clip, media }: { clip: Clip; media: MediaAsset | null }) {
  const [sub, setSub] = useState('Basic')
  const colorFx = (clip.effects ?? []).filter(isColorEffect)
  const comparing = colorFx.length > 0 && colorFx.every(e => e.enabled === false)
  return (
    <>
      <SubTabs
        tabs={['Basic', 'HSL', 'Curves', 'Color Wheels']}
        value={sub}
        onChange={setSub}
        more={[
          { label: 'Color Match', disabled: true },
          { label: 'AI Color Palette', disabled: true },
        ]}
        compare={{
          on: comparing,
          toggle: () => {
            if (!colorFx.length) return toast('No color adjustments to compare yet')
            exec({ op: 'batch', commands: colorFx.map(e => ({ op: 'set_effect_enabled', clip_id: clip.id, effect_id: e.id, enabled: comparing }) as Command) })
          },
        }}
      />
      <div className="prop-scroll">
        {sub === 'Basic' ? <ColorBasic clip={clip} media={media} /> : sub === 'HSL' ? <HslPane clip={clip} /> : sub === 'Curves' ? <CurvesPane clip={clip} /> : <WheelsPane clip={clip} />}
      </div>
    </>
  )
}

function ColorBasic({ clip, media }: { clip: Clip; media: MediaAsset | null }) {
  const [mode, setMode] = useState<'default' | 'custom'>('default')
  const [all, setAll] = useState(false)
  const custom = useCustomPresets()
  const picking = useEditor(s => !!s.eyedropper)
  const fx = colorEffect(clip)
  const value = (k: string) => effectValue(clip, fx, k, 0)
  const setParams = (params: Record<string, number>) => setEffectParams(clip, COLOR_KIND, params)
  const row = (key: string, label: string, gradient?: string) => (
    <EffectSlider key={key} clip={clip} kind={COLOR_KIND} effect={fx} name={key} label={label} min={-100} max={100} def={0} gradient={gradient} centered={!gradient} />
  )
  const thumb = media && media.kind !== 'audio' ? backend.media.thumb(media.path, seconds(clip.source_in ?? 0) + 0.5, 90) : null
  const applyPreset = (params: Record<string, number>) => {
    if (!Object.keys(params).length) return fx && exec({ op: 'remove_effect', clip_id: clip.id, effect_id: fx.id })
    // A preset replaces the basic adjustments.
    setParams(Object.fromEntries(ADJUST_KEYS.map(k => [k, params[k] ?? 0])))
  }
  // Filmora's Color and Light switches turn their section off and back on.
  const section = (key: string, keys: string[], on: boolean) => {
    if (on) return moduleOn(key, clip)
    if (!fx) return
    const params = (values: (k: string) => number) => Object.fromEntries(keys.map(k => [k, { value: fv(values(k)) }]))
    moduleOff(key, clip, [{ op: 'set_effect_params', clip_id: clip.id, effect_id: fx.id, params: params(() => 0) }], [{ op: 'set_effect_params', clip_id: clip.id, effect_id: fx.id, params: params(k => value(k)) }])
  }
  const whiteBalance = (rgb: [number, number, number]) => {
    const wb = neutralize(rgb)
    if (wb) setParams(wb)
  }
  return (
    <>
      <Module title="Preset">
        <Pills value={mode} onChange={setMode} className="wide-pills" options={[{ value: 'default', label: 'Default' }, { value: 'custom', label: 'Custom' }]} />
        <div className="color-presets">
          {mode === 'default'
            ? (all ? PRESETS : PRESETS.slice(0, 4)).map(pr => (
                <button key={pr.name} className="preset" data-preset={pr.name} onClick={() => applyPreset(pr.params)}>
                  <div className="preset-thumb">{pr.name === 'None' || !thumb ? <IcNone size={34} strokeWidth={1} /> : <img src={thumb} style={{ filter: pr.filter }} alt="" />}</div>
                  <span>{pr.name}</span>
                </button>
              ))
            : custom.map(pr => (
                <button
                  key={pr.name}
                  className="preset"
                  data-preset={pr.name}
                  title="Right-click to delete"
                  onClick={() => applyCustom(clip, pr)}
                  onContextMenu={e => {
                    e.preventDefault()
                    saveCustom(loadCustom().filter(p => p.name !== pr.name))
                  }}
                >
                  <div className="preset-thumb">{thumb ? <img src={thumb} alt="" /> : <IcNone size={34} strokeWidth={1} />}</div>
                  <span>{pr.name}</span>
                </button>
              ))}
          {mode === 'custom' && !custom.length && <p className="mask-hint">Use "Save as Custom" to keep the current color settings here.</p>}
        </div>
        {mode === 'default' && (
          <button className={`expand-bar ${all ? 'open' : ''}`} title={all ? 'Show fewer' : 'Show all presets'} onClick={() => setAll(!all)}>
            <IcChevronDown size={12} />
          </button>
        )}
      </Module>
      <LutModule clip={clip} />
      <Module title="Color" toggle={!moduleIsOff('color', clip)} onToggle={v => section('color', COLOR_KEYS, v)}>
        <div className="prow">
          <span className="plabel wide">Auto White Balance</span>
          <span className="spacer" />
          <button
            className="btn-gray auto-btn"
            onClick={async () => {
              if (!media || media.kind === 'audio') return toast('Auto white balance needs a video or image clip')
              const rgb = await averageColor(backend.media.thumb(media.path, seconds(clip.source_in ?? 0) + 0.5, 90))
              if (!rgb) return toast('Could not read the frame')
              whiteBalance(rgb)
            }}
          >
            Auto
          </button>
          <button
            className={`kf ${picking ? 'on' : ''}`}
            title="Pick a white or gray point in the player"
            onClick={() => {
              if (getState().eyedropper) return setState({ eyedropper: null })
              setState({ eyedropper: rgb => whiteBalance(rgb) })
              toast('Click a white or gray area in the player')
            }}
          >
            <IcEyedropper size={18} />
          </button>
        </div>
        {row('temperature', 'Temperature', 'linear-gradient(90deg,#2fa4d9,#7c8a8f 50%,#e8892e)')}
        {row('tint', 'Tint', 'linear-gradient(90deg,#2fbf4a,#7c8a8f 50%,#b440e0)')}
        {row('vibrance', 'Vibrance', 'linear-gradient(90deg,#7c8a8f,#d6a04a 40%,#e040c8 75%,#a040ff)')}
        {row('saturation', 'Saturation', 'linear-gradient(90deg,#7c8a8f,#d6a04a 40%,#e040c8 75%,#a040ff)')}
      </Module>
      <Module
        title="Light"
        toggle={!moduleIsOff('light', clip)}
        onToggle={v => section('light', LIGHT_KEYS, v)}
        extra={
          <button className="kf" title="Reset light" onClick={e => (e.stopPropagation(), setParams({ exposure: 0, brightness: 0, contrast: 0, highlights: 0, shadows: 0, whites: 0, blacks: 0 }))}>
            <IcReset size={16} />
          </button>
        }
      >
        {row('exposure', 'Exposure')}
        {row('brightness', 'Brightness')}
        {row('contrast', 'Contrast')}
        {row('highlights', 'Highlight')}
        {row('shadows', 'Shadow')}
        {row('whites', 'White')}
        {row('blacks', 'Black')}
      </Module>
      <Module title="Vignette" defaultOpen={value('vignette') !== 0}>
        {row('vignette', 'Amount')}
      </Module>
    </>
  )
}

function LutModule({ clip }: { clip: Clip }) {
  const kind = 'jb.color.lut'
  const e = findEffect(clip, kind)
  const on = effectOn(e)
  const path = textValue(e, 'path')
  const pick = async () => {
    const r = await backend.pickFiles({ title: 'Load LUT', filters: [{ name: 'LUT', extensions: ['cube', 'CUBE'] }], multiple: false })
    if (!r?.[0]) return null
    return r[0]
  }
  return (
    <Module
      title="LUT"
      toggle={on}
      onToggle={async v => {
        if (!v || e) return switchEffect(clip, kind, v)
        const p = await pick()
        if (p) setEffectParams(clip, kind, { path: text(p), intensity: 100 })
      }}
      defaultOpen={on}
    >
      <div className="prow">
        <span className="plabel wide" title={path}>{path ? path.split(/[/\\]/).pop() : 'No LUT loaded'}</span>
        <span className="spacer" />
        <button
          className="btn-gray auto-btn"
          onClick={async () => {
            const p = await pick()
            if (p) setEffectParams(clip, kind, { path: text(p), ...(e ? {} : { intensity: 100 }) })
          }}
        >
          {path ? 'Change' : 'Load'}
        </button>
      </div>
      <EffectSlider clip={clip} kind={kind} effect={e} name="intensity" label="Strength" min={0} max={100} def={100} unit="%" disabled={!on} />
    </Module>
  )
}

/** Average color of an image URL (sRGB 0..1). */
async function averageColor(url: string): Promise<[number, number, number] | null> {
  try {
    const blob = await (await fetch(url)).blob()
    const bmp = await createImageBitmap(blob)
    const c = document.createElement('canvas')
    c.width = 64
    c.height = Math.max(1, Math.round((64 * bmp.height) / bmp.width))
    const ctx = c.getContext('2d')!
    ctx.drawImage(bmp, 0, 0, c.width, c.height)
    const d = ctx.getImageData(0, 0, c.width, c.height).data
    const sum = [0, 0, 0]
    for (let i = 0; i < d.length; i += 4) for (let k = 0; k < 3; k++) sum[k] += d[i + k]
    const n = d.length / 4
    return [sum[0] / n / 255, sum[1] / n / 255, sum[2] / n / 255]
  } catch {
    return null
  }
}

const lin = (c: number) => (c <= 0.04045 ? c / 12.92 : ((c + 0.055) / 1.055) ** 2.4)

/**
 * Temperature/tint that make `rgb` neutral under jb.color.adjust's white
 * balance gains (crates/jb-render/src/fx/color.rs `cc_white_balance`).
 */
export function neutralize(rgb: [number, number, number]): { temperature: number; tint: number } | null {
  const [r, g, b] = rgb.map(lin)
  if (r + b <= 1e-4 || g <= 1e-4) return null
  const t = Math.max(-1, Math.min(1, (b - r) / (0.3 * (r + b))))
  const r2 = r * (1 + 0.3 * t)
  const b2 = b * (1 - 0.3 * t)
  const m = (r2 + b2) / 2
  const tint = Math.max(-1, Math.min(1, (g - m) / (0.25 * g + 0.1 * m)))
  return { temperature: Math.round(t * 100), tint: Math.round(tint * 100) }
}

// ---------- HSL ----------

const HSL_COLORS: { id: string; css: string }[] = [
  { id: 'red', css: '#e5484d' },
  { id: 'orange', css: '#f38a2e' },
  { id: 'yellow', css: '#f2d43a' },
  { id: 'green', css: '#46c25a' },
  { id: 'aqua', css: '#2fc6c6' },
  { id: 'blue', css: '#3a7ef2' },
  { id: 'purple', css: '#8e5ae8' },
  { id: 'magenta', css: '#e04fc1' },
]

function HslPane({ clip }: { clip: Clip }) {
  const kind = 'jb.color.hsl'
  const [color, setColor] = useState('red')
  const e = findEffect(clip, kind)
  return (
    <Module
      title="HSL"
      toggle={effectOn(e) || !e}
      onToggle={v => (e ? switchEffect(clip, kind, v) : undefined)}
      extra={
        <button className="kf" title="Reset HSL" onClick={ev => (ev.stopPropagation(), e && exec({ op: 'remove_effect', clip_id: clip.id, effect_id: e.id }))}>
          <IcReset size={16} />
        </button>
      }
    >
      <div className="hsl-colors">
        {HSL_COLORS.map(c => (
          <button key={c.id} className={`hsl-dot ${c.id === color ? 'on' : ''}`} title={c.id} data-hsl={c.id} style={{ background: c.css }} onClick={() => setColor(c.id)} />
        ))}
      </div>
      <EffectSlider clip={clip} kind={kind} effect={e} name={`${color}_hue`} label="Hue" min={-100} max={100} def={0} centered />
      <EffectSlider clip={clip} kind={kind} effect={e} name={`${color}_sat`} label="Saturation" min={-100} max={100} def={0} centered />
      <EffectSlider clip={clip} kind={kind} effect={e} name={`${color}_lum`} label="Luminance" min={-100} max={100} def={0} centered />
    </Module>
  )
}

// ---------- Curves ----------

const CHANNELS: { id: 'master' | 'red' | 'green' | 'blue'; label: string; css: string }[] = [
  { id: 'master', label: 'RGB', css: '#e6e9ec' },
  { id: 'red', label: 'Red', css: '#e5484d' },
  { id: 'green', label: 'Green', css: '#46c25a' },
  { id: 'blue', label: 'Blue', css: '#3a7ef2' },
]

type Pt = [number, number]
const parseCurve = (s: string): Pt[] => {
  const pts = s
    .trim()
    .split(/\s+/)
    .map(p => p.split('/').map(Number) as Pt)
    .filter(p => p.length === 2 && p.every(Number.isFinite))
  return pts.length >= 2 ? pts.sort((a, b) => a[0] - b[0]) : [[0, 0], [1, 1]]
}
const curveText = (pts: Pt[]) => pts.map(([x, y]) => `${+x.toFixed(4)}/${+y.toFixed(4)}`).join(' ')

function CurvesPane({ clip }: { clip: Clip }) {
  const kind = 'jb.color.curves'
  const [ch, setCh] = useState<'master' | 'red' | 'green' | 'blue'>('master')
  const e = findEffect(clip, kind)
  const pts = parseCurve(textValue(e, ch, '0/0 1/1'))
  const commit = (next: Pt[]) => setEffectParams(clip, kind, { [ch]: text(curveText(next)) })
  return (
    <Module
      title="Curves"
      toggle={effectOn(e) || !e}
      onToggle={v => (e ? switchEffect(clip, kind, v) : undefined)}
      extra={
        <button className="kf" title="Reset curve" onClick={ev => (ev.stopPropagation(), e && setEffectParams(clip, kind, { [ch]: text('0/0 1/1') }))}>
          <IcReset size={16} />
        </button>
      }
    >
      <Pills value={ch} onChange={setCh} className="wide-pills" options={CHANNELS.map(c => ({ value: c.id, label: c.label }))} />
      <CurveEditor points={pts} color={CHANNELS.find(c => c.id === ch)!.css} onCommit={commit} />
      <EffectSlider clip={clip} kind={kind} effect={e} name="intensity" label="Strength" min={0} max={100} def={100} unit="%" />
    </Module>
  )
}

function CurveEditor({ points, color, onCommit }: { points: Pt[]; color: string; onCommit: (p: Pt[]) => void }) {
  const [pts, setPts] = useState(points)
  useEffect(() => setPts(points), [curveText(points)])
  const ref = useRef<SVGSVGElement>(null)
  const S = 200
  const at = (cx: number, cy: number): Pt => {
    const r = ref.current!.getBoundingClientRect()
    return [Math.max(0, Math.min(1, (cx - r.left) / r.width)), Math.max(0, Math.min(1, 1 - (cy - r.top) / r.height))]
  }
  return (
    <svg
      ref={ref}
      className="curve-svg"
      viewBox={`0 0 ${S} ${S}`}
      onDoubleClick={e => {
        const p = at(e.clientX, e.clientY)
        const next = [...pts, p].sort((a, b) => a[0] - b[0])
        setPts(next)
        onCommit(next)
      }}
    >
      {[0.25, 0.5, 0.75].map(k => (
        <g key={k} stroke="#2f363c">
          <line x1={k * S} x2={k * S} y1={0} y2={S} />
          <line y1={k * S} y2={k * S} x1={0} x2={S} />
        </g>
      ))}
      <line x1={0} y1={S} x2={S} y2={0} stroke="#3b4248" strokeDasharray="3 3" />
      <polyline points={pts.map(([x, y]) => `${x * S},${(1 - y) * S}`).join(' ')} fill="none" stroke={color} strokeWidth="1.5" />
      {pts.map(([x, y], i) => (
        <circle
          key={i}
          cx={x * S}
          cy={(1 - y) * S}
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
            const mv = (ev: PointerEvent) => {
              const [nx, ny] = at(ev.clientX, ev.clientY)
              const lo = i > 0 ? pts[i - 1][0] + 0.01 : 0
              const hi = i < pts.length - 1 ? pts[i + 1][0] - 0.01 : 1
              const fx = i === 0 || i === pts.length - 1 ? pts[i][0] : Math.max(lo, Math.min(hi, nx))
              cur = pts.map((q, j) => (j === i ? [fx, ny] : q))
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
  )
}

// ---------- Color wheels ----------

const WHEELS = [
  { id: 'shadows', label: 'Shadows' },
  { id: 'midtones', label: 'Midtones' },
  { id: 'highlights', label: 'Highlights' },
  { id: 'global', label: 'Global' },
]

function WheelsPane({ clip }: { clip: Clip }) {
  const kind = 'jb.color.wheels'
  const e = findEffect(clip, kind)
  return (
    <Module
      title="Color Wheels"
      toggle={effectOn(e) || !e}
      onToggle={v => (e ? switchEffect(clip, kind, v) : undefined)}
      extra={
        <button className="kf" title="Reset wheels" onClick={ev => (ev.stopPropagation(), e && exec({ op: 'remove_effect', clip_id: clip.id, effect_id: e.id }))}>
          <IcReset size={16} />
        </button>
      }
    >
      <div className="wheels">
        {WHEELS.map(w => (
          <Wheel key={w.id} clip={clip} e={e} id={w.id} label={w.label} />
        ))}
      </div>
    </Module>
  )
}

function Wheel({ clip, e, id, label }: { clip: Clip; e: EffectInstance | undefined; id: string; label: string }) {
  const kind = 'jb.color.wheels'
  const x = effectValue(clip, e, `${id}_x`, 0)
  const y = effectValue(clip, e, `${id}_y`, 0)
  const [pos, setPos] = useDraft<[number, number]>([x, y])
  const [lum, setLum] = useDraft(effectValue(clip, e, `${id}_lum`, 0))
  const ref = useRef<HTMLDivElement>(null)
  const at = (cx: number, cy: number): [number, number] => {
    const r = ref.current!.getBoundingClientRect()
    let dx = ((cx - r.left) / r.width) * 2 - 1
    let dy = ((cy - r.top) / r.height) * 2 - 1
    const d = Math.hypot(dx, dy)
    if (d > 1) {
      dx /= d
      dy /= d
    }
    return [Math.round(dx * 100), Math.round(-dy * 100)]
  }
  return (
    <div className="wheel-box" data-wheel={id}>
      <div
        ref={ref}
        className="wheel"
        onDoubleClick={() => setEffectParams(clip, kind, { [`${id}_x`]: 0, [`${id}_y`]: 0 })}
        onPointerDown={ev => {
          if (ev.button !== 0) return
          let cur = at(ev.clientX, ev.clientY)
          setPos(cur)
          const mv = (m: PointerEvent) => {
            cur = at(m.clientX, m.clientY)
            setPos(cur)
          }
          const up = () => {
            window.removeEventListener('pointermove', mv)
            window.removeEventListener('pointerup', up)
            setEffectParams(clip, kind, { [`${id}_x`]: cur[0], [`${id}_y`]: cur[1] })
          }
          window.addEventListener('pointermove', mv)
          window.addEventListener('pointerup', up)
        }}
      >
        <i style={{ left: `${50 + pos[0] / 2}%`, top: `${50 - pos[1] / 2}%` }} />
      </div>
      <span>{label}</span>
      <div className="wheel-lum" title={`${label} luminance: ${Math.round(lum)}`}>
        <Slider value={lum} min={-100} max={100} step={1} centered onInput={setLum} onCommit={v => setEffectParams(clip, kind, { [`${id}_lum`]: v })} />
      </div>
    </div>
  )
}

export { fv }
