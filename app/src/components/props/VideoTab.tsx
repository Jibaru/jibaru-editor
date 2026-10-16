// Video tab: Basic (Transform, Compositing, Background, Shadow, Auto
// Enhance, Stabilization, ...), Mask and Matting (Chroma Key).

import { useState } from 'react'
import { backend } from '../../backend'
import { defaultTransform, float } from '../../core/model'
import { BLEND_MODES } from '../../core/presets'
import type { BlendMode, Clip, Command, EffectInstance, Project } from '../../core/types'
import {
  addEffectCommand,
  effectOn,
  effectValue,
  findEffect,
  moduleIsOff,
  moduleOff,
  moduleOn,
  removeEffects,
  setEffectParams,
  setParam,
  setParamCommands,
  switchEffect,
  valueAt,
} from '../../edit'
import { catalogLabel } from '../../library'
import { applyBackend, exec, toast, useEditor } from '../../store'
import { KfControl, Module, NumberField, Select, SliderRow, Toggle } from '../../ui/controls'
import {
  IcBrush,
  IcCircle,
  IcCorner,
  IcFlipH,
  IcFlipV,
  IcHeart,
  IcLine,
  IcLines,
  IcLock,
  IcPen,
  IcRect,
  IcRotateCcw,
  IcRotateCw,
  IcStar,
  IcTitles,
  IcTrash,
} from '../../ui/icons'
import { bool, boolValue, CLOUD_NOTE, ColorInput, EffectColor, EffectSlider, fromHex, hex, kfFor, SubTabs, textValue, useDraft, usePlayheadTick } from './common'

export function VideoTab({ clip, project }: { clip: Clip; project: Project }) {
  const [sub, setSub] = useState('Basic')
  return (
    <>
      <SubTabs tabs={['Basic', 'Mask', 'Matting']} value={sub} onChange={setSub} />
      <div className="prop-scroll">{sub === 'Basic' ? <VideoBasic clip={clip} project={project} /> : sub === 'Mask' ? <MaskPane clip={clip} /> : <MattingPane clip={clip} />}</div>
    </>
  )
}

const TRANSFORM_PATHS = ['transform.position_x', 'transform.position_y', 'transform.scale_x', 'transform.scale_y', 'transform.rotation']

/** Commands that put a clip's transform back to `t` (keyframes included). */
const transformRestore = (clip: Clip): Command[] => [{ op: 'set_transform', clip_id: clip.id, transform: structuredClone(clip.transform ?? defaultTransform()) }]

function VideoBasic({ clip, project }: { clip: Clip; project: Project }) {
  usePlayheadTick()
  const tr = clip.transform ?? defaultTransform()
  const [linked, setLinked] = useState(true)
  const sx = valueAt(clip, 'transform.scale_x', 1)
  const sy = valueAt(clip, 'transform.scale_y', 1)
  const px = valueAt(clip, 'transform.position_x', 0)
  const py = valueAt(clip, 'transform.position_y', 0)
  const rot = valueAt(clip, 'transform.rotation', 0)
  const op = valueAt(clip, 'opacity', 1)
  const [scale, setScale] = useDraft<[number, number]>([sx, sy])
  const [opacity, setOpacity] = useDraft(op)
  const W = project.settings.width
  const H = project.settings.height
  const transformOff = moduleIsOff('transform', clip)
  const compOff = moduleIsOff('compositing', clip)

  const scaleTo = (axis: 'x' | 'y', pct: number, live: boolean) => {
    const v = Math.max(0, pct / 100)
    const next: [number, number] = linked ? [v, v] : axis === 'x' ? [v, scale[1]] : [scale[0], v]
    if (live) return setScale(next)
    const cmds = linked
      ? [...setParamCommands(clip, 'transform.scale_x', v), ...setParamCommands(clip, 'transform.scale_y', v)]
      : setParamCommands(clip, axis === 'x' ? 'transform.scale_x' : 'transform.scale_y', v)
    if (cmds.length) exec({ op: 'batch', commands: cmds })
  }
  const setPos = (x: number, y: number) => {
    const cmds = [...setParamCommands(clip, 'transform.position_x', x), ...setParamCommands(clip, 'transform.position_y', y)]
    if (cmds.length) exec({ op: 'batch', commands: cmds })
  }
  const corner = findEffect(clip, 'jb.corner_radius')
  const enhance = findEffect(clip, 'jb.auto_enhance')
  return (
    <>
      <Module
        title="Transform"
        toggle={!transformOff}
        onToggle={on => (on ? moduleOn('transform', clip) : moduleOff('transform', clip, [{ op: 'set_transform', clip_id: clip.id, transform: defaultTransform() }], transformRestore(clip)))}
        keyframe={kfFor(clip, TRANSFORM_PATHS, [px, py, sx, sy, rot])}
      >
        <div className="prow label-only">Scale</div>
        <div className="scale-group">
          <button className={`scale-lock ${linked ? 'on' : ''}`} onClick={() => setLinked(!linked)} title="Keep proportions">
            <IcLock size={13} />
          </button>
          <SliderRow sublabel label="Width" value={scale[0] * 100} min={0} max={500} unit="%" onInput={v => scaleTo('x', v, true)} onCommit={v => scaleTo('x', v, false)} keyframe={kfFor(clip, linked ? ['transform.scale_x', 'transform.scale_y'] : ['transform.scale_x'], linked ? [sx, sy] : [sx])} />
          <SliderRow sublabel label="Height" value={scale[1] * 100} min={0} max={500} unit="%" onInput={v => scaleTo('y', v, true)} onCommit={v => scaleTo('y', v, false)} keyframe={kfFor(clip, linked ? ['transform.scale_x', 'transform.scale_y'] : ['transform.scale_y'], linked ? [sx, sy] : [sy])} />
        </div>
        <div className="prow xy-row">
          <span className="plabel">Position</span>
          <NumberField className="xy" prefix="X" value={px * W} onCommit={v => setPos(v / W, py)} />
          <NumberField className="xy" prefix="Y" value={-py * H} onCommit={v => setPos(px, -v / H)} />
          <span className="pend">
            <KfButtons clip={clip} paths={['transform.position_x', 'transform.position_y']} values={[px, py]} />
          </span>
        </div>
        <div className="prow" title="Curved motion paths between position keyframes are not supported yet">
          <span className="plabel">Path Curve</span>
          <span className="spacer" />
          <Toggle on={false} disabled title="Curved motion paths between position keyframes are not supported yet" />
        </div>
        <div className="prow xy-row" title="Rotation and scale use the clip center; a custom anchor point is not supported yet">
          <span className="plabel">Anchor Point</span>
          <NumberField className="xy" prefix="X" value={0} onCommit={() => {}} disabled title="Not supported yet" />
          <NumberField className="xy" prefix="Y" value={0} onCommit={() => {}} disabled title="Not supported yet" />
          <span className="pend" />
        </div>
        <div className="prow rotate-row">
          <span className="plabel">Rotate</span>
          <NumberField className="wide" value={rot} suffix="°" onCommit={v => setParam(clip, 'transform.rotation', v)} />
          <Dial value={rot} onCommit={v => setParam(clip, 'transform.rotation', v)} />
          <span className="pend">
            <KfButtons clip={clip} paths={['transform.rotation']} values={[rot]} />
          </span>
        </div>
        <div className="prow flip-row">
          <span className="plabel">Flip</span>
          <button className={`sq-btn ${tr.flip_h ? 'on' : ''}`} title="Flip horizontal" onClick={() => exec({ op: 'set_flip', clip_id: clip.id, horizontal: !tr.flip_h, vertical: null })}><IcFlipH size={18} /></button>
          <button className={`sq-btn ${tr.flip_v ? 'on' : ''}`} title="Flip vertical" onClick={() => exec({ op: 'set_flip', clip_id: clip.id, horizontal: null, vertical: !tr.flip_v })}><IcFlipV size={18} /></button>
          <button className="sq-btn" title="Rotate 90° clockwise" onClick={() => setParam(clip, 'transform.rotation', (((rot + 90) % 360) + 360) % 360)}><IcRotateCw size={18} /></button>
          <button className="sq-btn" title="Rotate 90° counterclockwise" onClick={() => setParam(clip, 'transform.rotation', (((rot - 90) % 360) + 360) % 360)}><IcRotateCcw size={18} /></button>
        </div>
        <div className="prow corner-row">
          <span className="plabel">Corner Radius</span>
          <NumberField className="wide" digits={0} prefix={<IcCorner size={14} />} value={effectValue(clip, corner, 'radius', 0)} onCommit={v => setEffectParams(clip, 'jb.corner_radius', { radius: Math.max(0, Math.min(100, v)) })} />
          <IcCorner size={18} className="corner-icon" />
          <span className="pend">{corner && <KfButtons clip={clip} paths={[`effect.${corner.id}.radius`]} values={[effectValue(clip, corner, 'radius', 0)]} />}</span>
        </div>
      </Module>
      <Module
        title="Compositing"
        toggle={!compOff}
        onToggle={on =>
          on
            ? moduleOn('compositing', clip)
            : moduleOff(
                'compositing',
                clip,
                [
                  { op: 'set_blend_mode', clip_id: clip.id, blend_mode: 'normal' },
                  { op: 'set_opacity', clip_id: clip.id, opacity: float(1) },
                ],
                [
                  { op: 'set_blend_mode', clip_id: clip.id, blend_mode: clip.blend_mode ?? 'normal' },
                  { op: 'set_opacity', clip_id: clip.id, opacity: structuredClone(clip.opacity ?? float(1)) },
                ],
              )
        }
        keyframe={kfFor(clip, ['opacity'], [op])}
        defaultOpen={false}
      >
        <div className="prow select-row">
          <span className="plabel">Blend Mode</span>
          <Select value={clip.blend_mode ?? 'normal'} options={BLEND_MODES} onChange={(v: BlendMode) => exec({ op: 'set_blend_mode', clip_id: clip.id, blend_mode: v })} />
        </div>
        <SliderRow label="Opacity" value={opacity * 100} min={0} max={100} unit="%" digits={0} onInput={v => setOpacity(v / 100)} onCommit={v => setParam(clip, 'opacity', v / 100)} keyframe={kfFor(clip, ['opacity'], [op])} />
      </Module>
      <BackgroundModule clip={clip} />
      <ShadowModule clip={clip} />
      <Module title="Auto Enhance" toggle={effectOn(enhance)} onToggle={on => switchEffect(clip, 'jb.auto_enhance', on, { amount: 50 })} defaultOpen={false}>
        <EffectSlider clip={clip} kind="jb.auto_enhance" effect={enhance} name="amount" label="Amount" min={0} max={100} def={50} disabled={!effectOn(enhance)} />
      </Module>
      <FxModule clip={clip} kind="jb.motion_blur" title="Motion Blur" sliders={[['intensity', 'Intensity', 0, 100, 50], ['angle', 'Angle', 0, 360, 0, '°']]} />
      <StabilizationModule clip={clip} />
      <FxModule clip={clip} kind="jb.denoise" title="Video Denoise" sliders={[['strength', 'Strength', 0, 100, 50]]} />
      <FxModule clip={clip} kind="jb.lens_correction" title="Lens Correction" sliders={[['level', 'Adjust Level', -100, 100, 0]]} centered />
      <Module title="AI Video Enhancer" toggle={false} toggleDisabled note={CLOUD_NOTE} defaultOpen={false} badge="AI" />
      <AppliedEffects clip={clip} />
    </>
  )
}

function KfButtons({ clip, paths, values }: { clip: Clip; paths: string[]; values: number[] }) {
  return <KfControl kf={kfFor(clip, paths, values)} />
}

/** A module backed by one effect: switch adds/enables it, sliders edit its params. */
function FxModule({
  clip,
  kind,
  title,
  sliders,
  centered,
}: {
  clip: Clip
  kind: string
  title: string
  sliders: [string, string, number, number, number, string?][]
  centered?: boolean
}) {
  const e = findEffect(clip, kind)
  const on = effectOn(e)
  return (
    <Module title={title} toggle={on} onToggle={v => switchEffect(clip, kind, v)} defaultOpen={false}>
      {sliders.map(([name, label, min, max, def, unit]) => (
        <EffectSlider key={name} clip={clip} kind={kind} effect={e} name={name} label={label} min={min} max={max} def={def} unit={unit} disabled={!on} centered={centered} />
      ))}
    </Module>
  )
}

function BackgroundModule({ clip }: { clip: Clip }) {
  const kind = 'jb.background'
  const e = findEffect(clip, kind)
  const on = effectOn(e)
  const mode = Math.round(effectValue(clip, e, 'mode', 0))
  return (
    <Module title="Background" toggle={on} onToggle={v => switchEffect(clip, kind, v)} help="Fills the frame around a clip that does not cover it" defaultOpen={false}>
      <div className="prow select-row">
        <span className="plabel">Type</span>
        <Select value={String(mode)} disabled={!on} options={[{ value: '0', label: 'Blur' }, { value: '1', label: 'Color' }]} onChange={v => setEffectParams(clip, kind, { mode: Number(v) })} />
      </div>
      {mode === 0 ? (
        <EffectSlider clip={clip} kind={kind} effect={e} name="blur" label="Blur Level" min={0} max={100} def={50} disabled={!on} />
      ) : (
        <EffectColor clip={clip} kind={kind} effect={e} name="color" label="Color" def={[0, 0, 0, 1]} />
      )}
    </Module>
  )
}

function ShadowModule({ clip }: { clip: Clip }) {
  const kind = 'jb.shadow'
  const e = findEffect(clip, kind)
  const on = effectOn(e)
  return (
    <Module title="Shadow" toggle={on} onToggle={v => switchEffect(clip, kind, v)} defaultOpen={false}>
      <EffectSlider clip={clip} kind={kind} effect={e} name="distance" label="Distance" min={0} max={100} def={10} disabled={!on} />
      <EffectSlider clip={clip} kind={kind} effect={e} name="angle" label="Angle" min={0} max={360} def={135} unit="°" disabled={!on} />
      <EffectSlider clip={clip} kind={kind} effect={e} name="blur" label="Blur" min={0} max={100} def={20} disabled={!on} />
      <EffectSlider clip={clip} kind={kind} effect={e} name="opacity" label="Opacity" min={0} max={100} def={50} unit="%" disabled={!on} />
      <EffectColor clip={clip} kind={kind} effect={e} name="color" label="Color" def={[0, 0, 0, 1]} />
    </Module>
  )
}

function StabilizationModule({ clip }: { clip: Clip }) {
  const kind = 'jb.stabilize'
  const e = findEffect(clip, kind)
  const on = effectOn(e)
  const [busy, setBusy] = useState(false)
  const analyzed = !!textValue(e, 'data')
  const analyze = async (effectId: string) => {
    setBusy(true)
    try {
      if (await applyBackend(backend.analyzeStabilization(clip.id, effectId))) toast('Stabilization analysis finished')
    } finally {
      setBusy(false)
    }
  }
  const turnOn = async (v: boolean) => {
    if (!v) return switchEffect(clip, kind, false)
    if (e) {
      await switchEffect(clip, kind, true)
      if (!analyzed) await analyze(e.id)
    } else {
      const { cmd, id } = addEffectCommand(clip, kind, { smoothness: 50 })
      if (await exec(cmd)) await analyze(id)
    }
  }
  return (
    <Module title="Stabilization" toggle={on} onToggle={turnOn} help="Analyzes camera motion and smooths it (local motion estimation)" defaultOpen={false}>
      <EffectSlider clip={clip} kind={kind} effect={e} name="smoothness" label="Smooth Level" min={0} max={100} def={50} disabled={!on} />
      <EffectSlider clip={clip} kind={kind} effect={e} name="zoom" label="Extra Zoom" min={0} max={50} def={0} unit="%" disabled={!on} />
      <div className="prow">
        <span className="plabel wide">{busy ? 'Analyzing motion…' : analyzed ? 'Motion analyzed' : 'Not analyzed'}</span>
        <span className="spacer" />
        <button
          className="btn-gray auto-btn"
          disabled={!on || busy || !e}
          onClick={() => e && analyze(e.id)}
        >
          Analyze
        </button>
      </div>
    </Module>
  )
}

const PANEL_KINDS = new Set([
  'jb.color.adjust',
  'jb.color.hsl',
  'jb.color.curves',
  'jb.color.lut',
  'jb.color.wheels',
  'jb.corner_radius',
  'jb.background',
  'jb.shadow',
  'jb.auto_enhance',
  'jb.motion_blur',
  'jb.stabilize',
  'jb.denoise',
  'jb.lens_correction',
  'jb.chroma_key',
])

/** Effects and filters dropped from the library, with controls from the registry schema. */
function AppliedEffects({ clip }: { clip: Clip }) {
  const catalog = useEditor(s => s.fx)
  const list = (clip.effects ?? []).filter(e => {
    if (PANEL_KINDS.has(e.kind) || e.kind.startsWith('jb.mask.')) return false
    return catalog.find(f => f.id === e.kind)?.kind !== 'animation'
  })
  return (
    <>
      {list.map(e => (
        <AppliedEffect key={e.id} clip={clip} e={e} />
      ))}
    </>
  )
}

function AppliedEffect({ clip, e }: { clip: Clip; e: EffectInstance }) {
  const info = useEditor(s => s.fx.find(f => f.id === e.kind))
  const on = e.enabled !== false
  const setP = (name: string, value: import('../../core/types').Value) => exec({ op: 'set_effect_params', clip_id: clip.id, effect_id: e.id, params: { [name]: { value } } })
  return (
    <Module
      title={catalogLabel(e.kind)}
      toggle={on}
      onToggle={v => exec({ op: 'set_effect_enabled', clip_id: clip.id, effect_id: e.id, enabled: v })}
      extra={
        <button className="kf" title="Delete effect" onClick={ev => (ev.stopPropagation(), exec({ op: 'remove_effect', clip_id: clip.id, effect_id: e.id }))}>
          <IcTrash size={16} />
        </button>
      }
    >
      {(info?.params ?? []).map(pp => {
        if (pp.ty === 'float' || pp.ty === 'int')
          return (
            <EffectSliderById key={pp.name} clip={clip} e={e} name={pp.name} label={pp.label} min={pp.min} max={pp.max} def={pp.default[0]} unit={pp.unit} digits={pp.ty === 'int' ? 0 : pp.max - pp.min <= 10 ? 2 : 0} disabled={!on} />
          )
        if (pp.ty === 'bool')
          return (
            <div className="prow" key={pp.name}>
              <span className="plabel">{pp.label}</span>
              <span className="spacer" />
              <Toggle on={boolValue(e, pp.name, pp.default[0] > 0.5)} disabled={!on} onChange={v => setP(pp.name, bool(v))} />
            </div>
          )
        if (pp.ty === 'color') {
          const v = e.params?.[pp.name]?.value
          const c = v?.type === 'color' ? v.value : pp.default
          return (
            <div className="prow" key={pp.name}>
              <span className="plabel">{pp.label}</span>
              <span className="spacer" />
              <ColorInput value={hex(c, true)} disabled={!on} onChange={h => setP(pp.name, { type: 'color', value: fromHex(h, c[3], true) })} />
            </div>
          )
        }
        return null
      })}
      {!info && <p className="mask-hint">Imported effect: kept as is, rendered as pass-through.</p>}
    </Module>
  )
}

/** Like EffectSlider but for an effect picked by id (several of one kind may exist). */
function EffectSliderById(props: { clip: Clip; e: EffectInstance; name: string; label: string; min: number; max: number; def: number; unit?: string; digits?: number; disabled?: boolean }) {
  usePlayheadTick()
  const { clip, e, name } = props
  const value = effectValue(clip, e, name, props.def)
  const [draft, setDraft] = useDraft(value)
  const path = `effect.${e.id}.${name}`
  const animated = !!e.params?.[name]?.keyframes?.length
  return (
    <SliderRow
      label={props.label}
      value={draft}
      min={props.min}
      max={props.max}
      step={props.digits ? 1 / 10 ** props.digits : 1}
      digits={props.digits ?? 0}
      unit={props.unit}
      disabled={props.disabled}
      onInput={setDraft}
      onCommit={v => (animated ? setParam(clip, path, v) : exec({ op: 'set_effect_params', clip_id: clip.id, effect_id: e.id, params: { [name]: float(v) } }))}
      keyframe={kfFor(clip, [path], [value])}
    />
  )
}

function Dial({ value, onCommit }: { value: number; onCommit: (v: number) => void }) {
  return (
    <button
      className="dial"
      title="Drag to rotate"
      onPointerDown={e => {
        const r = (e.currentTarget as HTMLElement).getBoundingClientRect()
        const cx = r.left + r.width / 2
        const cy = r.top + r.height / 2
        let v = value
        const el = e.currentTarget as HTMLElement
        const mv = (ev: PointerEvent) => {
          v = Math.round(((Math.atan2(ev.clientX - cx, cy - ev.clientY) * 180) / Math.PI + 360) % 360)
          el.style.setProperty('--rot', `${v}deg`)
        }
        const up = () => {
          window.removeEventListener('pointermove', mv)
          window.removeEventListener('pointerup', up)
          if (v !== value) onCommit(v)
        }
        window.addEventListener('pointermove', mv)
        window.addEventListener('pointerup', up)
      }}
      style={{ ['--rot' as string]: `${value}deg` }}
    >
      <i />
    </button>
  )
}

// ---------- Mask ----------

const MASKS: { kind: string; label: string; icon: (p: { size?: number; strokeWidth?: number }) => React.ReactNode; supported: boolean; accent?: boolean }[] = [
  { kind: 'pen', label: 'Pen', icon: IcPen, supported: false },
  { kind: 'text', label: 'Text', icon: IcTitles, supported: false },
  { kind: 'rectangle', label: 'Rectangle', icon: IcRect, supported: true },
  { kind: 'ellipse', label: 'Circle', icon: IcCircle, supported: true },
  { kind: 'double_line', label: 'Double Line', icon: IcLines, supported: true },
  { kind: 'single_line', label: 'Single Line', icon: IcLine, supported: true },
  { kind: 'heart', label: 'Heart', icon: IcHeart, supported: true },
  { kind: 'star', label: 'Star', icon: IcStar, supported: true },
  { kind: 'brush', label: 'Smart Brush (AI)', icon: IcBrush, supported: false, accent: true },
]

function MaskPane({ clip }: { clip: Clip }) {
  const current = (clip.effects ?? []).find(e => e.kind.startsWith('jb.mask.'))
  const kind = current?.kind ?? ''
  return (
    <div className="mask-pane">
      <div className="mask-grid">
        {MASKS.map(m => (
          <button
            key={m.kind}
            className={`mask-btn ${kind === `jb.mask.${m.kind}` ? 'on' : ''} ${m.accent ? 'accent' : ''}`}
            title={m.supported ? m.label : m.kind === 'brush' ? `${m.label}: ${CLOUD_NOTE}` : `${m.label} mask: not supported yet`}
            disabled={!m.supported}
            data-mask={m.kind}
            onClick={() => {
              const cmds: Command[] = []
              if (current) cmds.push({ op: 'remove_effect', clip_id: clip.id, effect_id: current.id })
              if (kind !== `jb.mask.${m.kind}`) cmds.push(addEffectCommand(clip, `jb.mask.${m.kind}`).cmd)
              exec({ op: 'batch', commands: cmds })
            }}
          >
            <m.icon size={34} strokeWidth={0.9} />
          </button>
        ))}
      </div>
      {current ? (
        <MaskControls clip={clip} e={current} />
      ) : (
        <>
          <div className="prow select-row" title="Image masks are not supported yet">
            <span className="plabel">Custom Mask</span>
            <Select value="none" disabled options={[{ value: 'none', label: 'Select a mask' }]} onChange={() => {}} />
            <button className="btn-gray add-btn" disabled title="Image masks are not supported yet">Add</button>
          </div>
          <p className="mask-hint">Pick a shape to mask the clip.</p>
        </>
      )}
    </div>
  )
}

function MaskControls({ clip, e }: { clip: Clip; e: EffectInstance }) {
  const kind = e.kind
  const shape = kind.slice('jb.mask.'.length)
  const s = (name: string, label: string, min: number, max: number, def: number, unit?: string) => (
    <EffectSlider key={name} clip={clip} kind={kind} effect={e} name={name} label={label} min={min} max={max} def={def} unit={unit} />
  )
  return (
    <>
      <Module
        title={MASKS.find(m => m.kind === shape)?.label ?? 'Mask'}
        extra={
          <button className="kf" title="Remove mask" onClick={ev => (ev.stopPropagation(), removeEffects(clip, x => x.id === e.id))}>
            <IcTrash size={16} />
          </button>
        }
      >
        {s('x', 'Position X', -100, 100, 0, '%')}
        {s('y', 'Position Y', -100, 100, 0, '%')}
        {s('rotation', 'Rotate', -360, 360, 0, '°')}
        {s('width', shape.includes('line') ? 'Length' : 'Width', 0, 200, 60, '%')}
        {shape !== 'single_line' && s('height', shape === 'double_line' ? 'Spacing' : 'Height', 0, 200, 60, '%')}
        {(shape === 'rectangle' || shape === 'star') && s('roundness', 'Roundness', 0, 100, 0)}
        {s('feather', 'Blur Strength', 0, 100, 0)}
        {s('opacity', 'Opacity', 0, 100, 100, '%')}
        <div className="prow">
          <span className="plabel">Invert Mask</span>
          <span className="spacer" />
          <Toggle on={boolValue(e, 'invert')} onChange={v => setEffectParams(clip, kind, { invert: bool(v) })} />
        </div>
      </Module>
    </>
  )
}

// ---------- Matting ----------

function MattingPane({ clip }: { clip: Clip }) {
  const kind = 'jb.chroma_key'
  const e = findEffect(clip, kind)
  const on = effectOn(e)
  const v = e?.params?.color?.value
  const color = v?.type === 'color' ? v.value : ([0, 1, 0, 1] as [number, number, number, number])
  return (
    <>
      <Module title="Chroma Key" toggle={on} onToggle={x => switchEffect(clip, kind, x)}>
        <div className="prow">
          <span className="plabel">Color</span>
          <span className="spacer" />
          <button className="chip" disabled={!on} onClick={() => setEffectParams(clip, kind, { color: { type: 'color', value: [0, 1, 0, 1] } })} title="Green screen">
            <i style={{ background: '#00ff00' }} />
          </button>
          <button className="chip" disabled={!on} onClick={() => setEffectParams(clip, kind, { color: { type: 'color', value: [0, 0, 1, 1] } })} title="Blue screen">
            <i style={{ background: '#0000ff' }} />
          </button>
          <ColorInput value={hex(color, true)} disabled={!on} title="Key color" onChange={h => setEffectParams(clip, kind, { color: { type: 'color', value: fromHex(h, 1, true) } })} />
        </div>
        <EffectSlider clip={clip} kind={kind} effect={e} name="tolerance" label="Tolerance" min={0} max={100} def={20} disabled={!on} />
        <EffectSlider clip={clip} kind={kind} effect={e} name="offset" label="Offset" min={0} max={100} def={0} disabled={!on} />
        <EffectSlider clip={clip} kind={kind} effect={e} name="edge_thickness" label="Edge Thickness" min={-10} max={10} def={0} digits={1} disabled={!on} centered />
        <EffectSlider clip={clip} kind={kind} effect={e} name="edge_feather" label="Edge Feather" min={0} max={10} def={0} digits={1} disabled={!on} />
        <EffectSlider clip={clip} kind={kind} effect={e} name="spill" label="Spill Removal" min={0} max={100} def={50} disabled={!on} />
        <div className="prow">
          <span className="plabel">Alpha Channel</span>
          <span className="spacer" />
          <Toggle on={boolValue(e, 'alpha_channel')} disabled={!on} onChange={x => setEffectParams(clip, kind, { alpha_channel: bool(x) })} />
        </div>
      </Module>
      <Module title="AI Portrait" toggle={false} toggleDisabled note={CLOUD_NOTE} defaultOpen={false} badge="AI" />
      <Module title="Smart Cutout" toggle={false} toggleDisabled note={CLOUD_NOTE} defaultOpen={false} badge="AI" />
    </>
  )
}
