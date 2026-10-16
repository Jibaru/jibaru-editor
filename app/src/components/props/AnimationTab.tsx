// Animation tab: In/Out motion presets (`add_animation`, the clip's animation
// slots) and keyframe presets (keyframes written over the whole clip).

import { useState } from 'react'
import { backend } from '../../backend'
import { newId, seconds, ticks } from '../../core/model'
import type { AnimationPlacement, Clip, Command, MediaAsset } from '../../core/types'
import { fv } from '../../edit'
import { animationsFor } from '../../library'
import { exec, openMenu, useEditor } from '../../store'
import { Pills, SliderRow } from '../../ui/controls'
import { IcFilter, IcImage, IcNone } from '../../ui/icons'
import { SubTabs, useDraft } from './common'

const GROUPS = ['All', 'Fade', 'Zoom', 'Slide', 'Spin', 'Bounce', 'Other']
const groupOf = (id: string) => {
  const g = GROUPS.slice(1, -1).find(g => id.includes(g.toLowerCase()))
  return g ?? 'Other'
}

export function AnimationTab({ clip, media }: { clip: Clip; media: MediaAsset | null }) {
  const [sub, setSub] = useState('Motion Presets')
  return (
    <>
      <SubTabs tabs={['Motion Presets', 'Keyframe Presets']} value={sub} onChange={setSub} />
      <div className="prop-scroll">{sub === 'Motion Presets' ? <MotionPresets clip={clip} media={media} /> : <KeyframePresets clip={clip} media={media} />}</div>
    </>
  )
}

function MotionPresets({ clip, media }: { clip: Clip; media: MediaAsset | null }) {
  const [dir, setDir] = useState<'in' | 'out'>('in')
  const [group, setGroup] = useState('All')
  const fx = useEditor(s => s.fx)
  const anims = animationsFor(fx, dir).filter(a => group === 'All' || groupOf(a.id) === group)
  const current = (clip.animations ?? []).find(a => a.placement === dir) ?? (clip.effects ?? []).find(e => anims.some(a => a.id === e.kind))
  const len = clip.end - clip.start
  const curDur = current?.range ? current.range[1] - current.range[0] : Math.min(10_000_000, Math.floor(len / 2))
  const [dur, setDur] = useDraft(seconds(curDur))
  const pick = (kind: string | null) => {
    const cmds: Command[] = []
    // Legacy: animations stored as ranged effects by older builds.
    const legacy = (clip.effects ?? []).filter(e => fx.some(f => f.kind === 'animation' && f.id === e.kind) && (dir === 'out') === (e.range?.[1] === len))
    for (const e of legacy) cmds.push({ op: 'remove_effect', clip_id: clip.id, effect_id: e.id })
    if (kind) cmds.push({ op: 'add_animation', clip_id: clip.id, kind, placement: dir as AnimationPlacement, duration: Math.min(curDur, len), id: newId() })
    else if ((clip.animations ?? []).some(a => a.placement === dir)) cmds.push({ op: 'remove_animation', clip_id: clip.id, placement: dir })
    if (cmds.length) exec(cmds.length === 1 ? cmds[0] : { op: 'batch', commands: cmds })
  }
  const thumb = media && media.kind !== 'audio' ? backend.media.thumb(media.path, seconds(clip.source_in ?? 0) + 0.5, 90) : null
  return (
    <>
      <div className="anim-head">
        <Pills value={dir} onChange={setDir} options={[{ value: 'in', label: 'In' }, { value: 'out', label: 'Out' }]} className="inout" />
        <span className="spacer" />
        <button
          className={`ticon ${group !== 'All' ? 'on' : ''}`}
          title="Filter"
          onClick={e => {
            const r = e.currentTarget.getBoundingClientRect()
            openMenu(r.left - 120, r.bottom + 2, GROUPS.map(g => ({ label: g, checked: g === group, action: () => setGroup(g) })))
          }}
        >
          <IcFilter size={20} className="muted-icon" />
        </button>
      </div>
      <div className="preset-grid anim">
        <button className={`preset ${!current ? 'on' : ''}`} onClick={() => pick(null)}>
          <div className="preset-thumb">
            <IcNone size={34} strokeWidth={1} />
          </div>
          <span>No Motion</span>
        </button>
        {anims.map(a => (
          <button key={a.id} className={`preset ${current?.kind === a.id ? 'on' : ''}`} onClick={() => pick(a.id)} title={a.description} data-anim={a.id}>
            <div className="preset-thumb">{thumb ? <img src={thumb} alt="" className={`anim-${a.id.split('.').pop()}`} /> : <IcImage size={28} />}</div>
            <span>{a.name}</span>
          </button>
        ))}
      </div>
      {current && (clip.animations ?? []).some(a => a.id === current.id) && (
        <SliderRow
          label="Duration"
          value={dur}
          min={0.1}
          max={Math.max(0.2, seconds(len))}
          step={0.1}
          digits={1}
          unit="s"
          onInput={setDur}
          onCommit={v => exec({ op: 'replace_animation', clip_id: clip.id, animation_id: current.id, duration: ticks(v) })}
        />
      )}
    </>
  )
}

type KfPreset = { id: string; label: string; keys: (len: number) => { param: string; values: [number, number] }[] }

/** Keyframe animations written over the whole clip (start and end keys). */
const KF_PRESETS: KfPreset[] = [
  { id: 'zoom_in', label: 'Zoom In', keys: () => [{ param: 'transform.scale', values: [1, 1.3] }] },
  { id: 'zoom_out', label: 'Zoom Out', keys: () => [{ param: 'transform.scale', values: [1.3, 1] }] },
  { id: 'pan_left', label: 'Pan Left', keys: () => [{ param: 'transform.scale', values: [1.2, 1.2] }, { param: 'transform.position_x', values: [0.08, -0.08] }] },
  { id: 'pan_right', label: 'Pan Right', keys: () => [{ param: 'transform.scale', values: [1.2, 1.2] }, { param: 'transform.position_x', values: [-0.08, 0.08] }] },
  { id: 'pan_up', label: 'Pan Up', keys: () => [{ param: 'transform.scale', values: [1.2, 1.2] }, { param: 'transform.position_y', values: [0.08, -0.08] }] },
  { id: 'pan_down', label: 'Pan Down', keys: () => [{ param: 'transform.scale', values: [1.2, 1.2] }, { param: 'transform.position_y', values: [-0.08, 0.08] }] },
  { id: 'rotate', label: 'Rotate', keys: () => [{ param: 'transform.rotation', values: [0, 360] }] },
  { id: 'fade', label: 'Fade In', keys: () => [{ param: 'opacity', values: [0, 1] }] },
]

const KF_PARAMS = ['transform.scale', 'transform.position_x', 'transform.position_y', 'transform.rotation', 'opacity']

function KeyframePresets({ clip, media }: { clip: Clip; media: MediaAsset | null }) {
  const len = clip.end - clip.start
  const apply = (preset: KfPreset | null) => {
    const cmds: Command[] = []
    // Start from a clean slate for every param a preset can touch.
    for (const param of KF_PARAMS) {
      const p = param === 'opacity' ? clip.opacity : param === 'transform.scale' ? clip.transform?.scale_x : clip.transform?.[param.slice(10) as 'position_x']
      if (p?.keyframes?.length) cmds.push({ op: 'clear_keyframes', clip_id: clip.id, param })
    }
    if (preset)
      for (const k of preset.keys(len)) {
        cmds.push({ op: 'add_keyframe', clip_id: clip.id, param: k.param, time: 0, value: fv(k.values[0]), interpolation: 'ease_in_out' })
        cmds.push({ op: 'add_keyframe', clip_id: clip.id, param: k.param, time: len, value: fv(k.values[1]), interpolation: 'linear' })
      }
    if (cmds.length) exec({ op: 'batch', commands: cmds })
  }
  const thumb = media && media.kind !== 'audio' ? backend.media.thumb(media.path, seconds(clip.source_in ?? 0) + 0.5, 90) : null
  return (
    <div className="preset-grid anim">
      <button className="preset" onClick={() => apply(null)} title="Remove transform and opacity keyframes">
        <div className="preset-thumb">
          <IcNone size={34} strokeWidth={1} />
        </div>
        <span>None</span>
      </button>
      {KF_PRESETS.map(p => (
        <button key={p.id} className="preset" onClick={() => apply(p)} data-kfpreset={p.id}>
          <div className="preset-thumb">{thumb ? <img src={thumb} alt="" /> : <IcImage size={28} />}</div>
          <span>{p.label}</span>
        </button>
      ))}
    </div>
  )
}
