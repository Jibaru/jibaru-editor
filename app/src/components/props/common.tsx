// Pieces shared by the property-panel tabs.

import { useEffect, useState, type ReactNode } from 'react'
import type { Clip, EffectInstance, Value } from '../../core/types'
import { effectValue, fv, jumpKeyframe, keyframeHere, keyframeTimes, liveEffectParam, setEffectParams, setParam, toggleKeyframe } from '../../edit'
import { exec, openMenu, useEditor } from '../../store'
import type { MenuItem } from '../../ui/ContextMenu'
import type { Kf } from '../../ui/controls'
import { SliderRow } from '../../ui/controls'
import { IcChevronDown, IcCompare, IcDoubleRight } from '../../ui/icons'

/** Cloud AI features of Filmora that Jibaru does not offer. */
export const CLOUD_NOTE = 'This is a Wondershare cloud AI feature. It is not available in Jibaru Editor.'

export function SubTabs({
  tabs,
  value,
  onChange,
  more,
  compare,
}: {
  tabs: string[]
  value: string
  onChange: (v: string) => void
  /** Items of the » menu (tabs that do not fit). */
  more?: MenuItem[]
  compare?: { on: boolean; toggle: () => void }
}) {
  return (
    <div className="sub-tabs">
      {tabs.map(t => (
        <button key={t} className={t === value ? 'on' : ''} onClick={() => onChange(t)}>
          {t}
        </button>
      ))}
      {more && (
        <button
          className="sub-more"
          title="More"
          onClick={e => {
            const r = e.currentTarget.getBoundingClientRect()
            openMenu(r.left, r.bottom + 2, more)
          }}
        >
          <i />
          <IcDoubleRight size={16} />
        </button>
      )}
      {compare && (
        <button className={`sub-compare ${compare.on ? 'on' : ''}`} title={compare.on ? 'Show the adjusted image' : 'Compare with the original'} onClick={compare.toggle}>
          <IcCompare size={18} />
          <IcChevronDown size={14} />
        </button>
      )}
    </div>
  )
}

/** Live value while dragging a slider, committed as one command on release. */
export function useDraft<T>(value: T): [T, (v: T | null) => void] {
  const [draft, setDraft] = useState<T | null>(null)
  useEffect(() => setDraft(null), [value])
  return [draft ?? value, setDraft]
}

/** Re-renders with the playhead while paused, so animated values and diamonds follow it. */
export function usePlayheadTick(): number {
  return useEditor(s => (s.playing ? -1 : s.playhead))
}

/** Keyframe controls for float param paths on `target` (edited through `clip`'s time). */
export function kfFor(clip: Clip, paths: string[], values: number[], target: Clip = clip): Kf {
  const animated = paths.some(p => keyframeTimes(target, p).length > 0)
  return {
    active: paths.every(p => keyframeHere(target, p) != null),
    animated,
    onToggle: () => toggleKeyframe(clip, paths, values, target),
    onPrev: () => jumpKeyframe(target, paths[0], -1),
    onNext: () => jumpKeyframe(target, paths[0], 1),
  }
}

/** Slider row for one float parameter of an effect, keyframable. */
export function EffectSlider({
  clip,
  kind,
  effect,
  name,
  label,
  min,
  max,
  def,
  unit,
  digits = 0,
  step,
  disabled,
  gradient,
  centered,
}: {
  clip: Clip
  kind: string
  effect: EffectInstance | undefined
  name: string
  label: ReactNode
  min: number
  max: number
  def: number
  unit?: string
  digits?: number
  step?: number
  disabled?: boolean
  gradient?: string
  centered?: boolean
}) {
  usePlayheadTick()
  const value = effectValue(clip, effect, name, def)
  const [draft, setDraft] = useDraft(value)
  const path = effect ? `effect.${effect.id}.${name}` : null
  const animated = !!effect?.params?.[name]?.keyframes?.length
  // Animated params get a keyframe at the playhead; others a constant value.
  const commit = (v: number) => (path && animated ? setParam(clip, path, v) : setEffectParams(clip, kind, { [name]: v }))
  return (
    <SliderRow
      label={label}
      value={draft}
      min={min}
      max={max}
      step={step ?? (digits ? 1 / 10 ** digits : 1)}
      digits={digits}
      unit={unit}
      disabled={disabled}
      gradient={gradient}
      centered={centered}
      onInput={v => (setDraft(v), animated || liveEffectParam(clip, effect, kind, name, v))}
      onCommit={commit}
      keyframe={path ? kfFor(clip, [path], [value]) : false}
    />
  )
}

// ---------- Colors ----------

const toLin = (c: number) => (c <= 0.04045 ? c / 12.92 : ((c + 0.055) / 1.055) ** 2.4)
const toSrgb = (c: number) => (c <= 0.0031308 ? c * 12.92 : 1.055 * c ** (1 / 2.4) - 0.055)

/** `#rrggbb` for a color; `linear` colors (effect params) are encoded to sRGB first. */
export function hex(c: number[], linear = false): string {
  return '#' + c.slice(0, 3).map(v => Math.round(Math.max(0, Math.min(1, linear ? toSrgb(v) : v)) * 255).toString(16).padStart(2, '0')).join('')
}

export function fromHex(h: string, alpha = 1, linear = false): [number, number, number, number] {
  const ch = (i: number) => parseInt(h.slice(i, i + 2), 16) / 255
  const rgb = [ch(1), ch(3), ch(5)].map(v => (linear ? toLin(v) : v))
  return [rgb[0], rgb[1], rgb[2], alpha]
}

export function ColorInput({ value, onChange, title, disabled }: { value: string; onChange: (hex: string) => void; title?: string; disabled?: boolean }) {
  return <input type="color" className="color-input" value={value} title={title} disabled={disabled} onChange={e => onChange(e.target.value)} />
}

/** Color row for an effect's color parameter (linear RGBA). */
export function EffectColor({ clip, kind, effect, name, label, def }: { clip: Clip; kind: string; effect: EffectInstance | undefined; name: string; label: string; def: [number, number, number, number] }) {
  const v = effect?.params?.[name]?.value
  const c = v?.type === 'color' ? v.value : def
  return (
    <div className="prow">
      <span className="plabel">{label}</span>
      <span className="spacer" />
      <ColorInput value={hex(c, true)} title={label} onChange={h => setEffectParams(clip, kind, { [name]: { type: 'color', value: fromHex(h, c[3], true) } })} />
    </div>
  )
}

export const boolValue = (e: EffectInstance | undefined, name: string, def = false) => {
  const v = e?.params?.[name]?.value
  return v?.type === 'bool' ? v.value : v?.type === 'float' ? v.value > 0.5 : def
}

export const textValue = (e: EffectInstance | undefined, name: string, def = '') => {
  const v = e?.params?.[name]?.value
  return v?.type === 'text' ? v.value : def
}

export const bool = (b: boolean): Value => ({ type: 'bool', value: b })
export const text = (t: string): Value => ({ type: 'text', value: t })
export { fv, exec }
