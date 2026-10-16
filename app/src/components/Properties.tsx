import { useEffect, useState, type ReactNode } from 'react'
import { backend } from '../backend'
import { defaultAudio, defaultTransform, findClip, float, fps, mainSequence, newId, paramValue, seconds, sequenceDuration, sourceLength, ticks, timecode } from '../core/model'
import type { AudioSettings, Clip, Command, EffectInstance, MediaAsset, Param, Project, Track, Transform } from '../core/types'
import { exec, setState, useEditor, type PropsTab } from '../store'
import { loadWave } from '../timeline/waves'
import { Diamond, Module, NumberField, Pills, Select, Slider, SliderRow, Toggle } from '../ui/controls'
import {
  IcBrush,
  IcCircle,
  IcCompare,
  IcCorner,
  IcDoubleRight,
  IcDownload,
  IcEyedropper,
  IcFilter,
  IcFlipH,
  IcFlipV,
  IcHeart,
  IcHelp,
  IcImage,
  IcLine,
  IcLines,
  IcLock,
  IcNone,
  IcNote,
  IcPen,
  IcRect,
  IcReset,
  IcRotateCcw,
  IcRotateCw,
  IcStar,
  IcTitles,
  IcChevronDown,
} from '../ui/icons'

export function Properties() {
  const project = useEditor(s => s.snap?.project)
  const path = useEditor(s => s.snap?.path ?? null)
  const selection = useEditor(s => s.selection)
  const tab = useEditor(s => s.propsTab)
  if (!project) return <section className="panel properties" />
  const found = selection.length ? findClip(project, selection[selection.length - 1]) : null
  if (!found) return <ProjectInfo project={project} path={path} />
  return <ClipProperties key={found.clip.id} project={project} clip={found.clip} track={found.track} tab={tab} />
}

function ProjectInfo({ project, path }: { project: Project; path: string | null }) {
  const s = project.settings
  const rate = fps(s.frame_rate)
  const firstVideo = mainSequence(project).tracks.flatMap(t => t.clips).find(c => c.source.type === 'media')
  const media = firstVideo && project.media.find(m => m.id === (firstVideo.source as { media_id: string }).media_id)
  const [name, setName] = useState<string | null>(null)
  return (
    <section className="panel properties">
      <div className="panel-head prop-tabs single">
        <button className="on">Project Info</button>
      </div>
      <div className="prop-scroll info">
        <dl className="info-list">
          <dt>Project Name:</dt>
          <dd>
            <input
              className="inline-edit"
              value={name ?? project.name}
              onChange={e => setName(e.target.value)}
              onBlur={() => {
                if (name != null && name.trim() && name !== project.name) exec({ op: 'rename_project', name: name.trim() })
                setName(null)
              }}
              onKeyDown={e => {
                e.stopPropagation()
                if (e.key === 'Enter') (e.target as HTMLInputElement).blur()
              }}
            />
          </dd>
          <dt>Project Files Location:</dt>
          <dd className="path">{path ?? 'Not saved yet'}</dd>
          <dt>Resolution:</dt>
          <dd>{s.width} x {s.height}</dd>
          <dt>Frame Rate:</dt>
          <dd>{Number.isInteger(rate) ? rate : rate.toFixed(2)}fps</dd>
          <dt>Color Space:</dt>
          <dd>SDR - Rec.709</dd>
          <dt>Sample Rate:</dt>
          <dd>{s.sample_rate}Hz</dd>
          <dt>Duration:</dt>
          <dd>{timecode(sequenceDuration(mainSequence(project)), s.frame_rate).slice(3)}</dd>
          <dt>Thumbnail:</dt>
          <dd>
            <div className="info-thumb">
              {media && media.kind !== 'audio' ? <img src={backend.media.thumb(media.path, 1, 160)} alt="" /> : <IcImage size={36} />}
            </div>
            <button className="btn-gray info-edit">Edit</button>
          </dd>
        </dl>
      </div>
    </section>
  )
}

type TabDef = { id: PropsTab; label: string }

function tabsFor(clip: Clip, track: Track, media: MediaAsset | null): TabDef[] {
  if (clip.source.type === 'title') return [{ id: 'video', label: 'Titles' }, { id: 'animation', label: 'Animation' }]
  if (track.kind === 'audio' || media?.kind === 'audio') return [{ id: 'audio', label: 'Audio' }, { id: 'speed', label: 'Speed' }]
  const tabs: TabDef[] = [{ id: 'video', label: 'Video' }]
  if (media?.has_audio) tabs.push({ id: 'audio', label: 'Audio' })
  if (media?.kind !== 'image') tabs.push({ id: 'speed', label: 'Speed' })
  tabs.push({ id: 'animation', label: 'Animation' }, { id: 'color', label: 'Color' })
  return tabs
}

function ClipProperties({ project, clip, track, tab }: { project: Project; clip: Clip; track: Track; tab: PropsTab }) {
  const media = clip.source.type === 'media' ? (project.media.find(m => m.id === (clip.source as { media_id: string }).media_id) ?? null) : null
  const tabs = tabsFor(clip, track, media)
  const active = tabs.some(t => t.id === tab) ? tab : tabs[0].id
  let body: ReactNode
  let reset: (() => void) | null = null
  let extraFooter: ReactNode = null
  if (active === 'video' && clip.source.type === 'title') body = <TitleTab clip={clip} />
  else if (active === 'video') {
    body = <VideoTab clip={clip} project={project} />
    reset = () => exec({ op: 'batch', commands: [{ op: 'set_transform', clip_id: clip.id, transform: defaultTransform() }, { op: 'set_opacity', clip_id: clip.id, opacity: float(1) }] })
  } else if (active === 'audio') {
    body = <AudioTab clip={clip} media={media} />
    reset = () => exec({ op: 'set_audio', clip_id: clip.id, audio: defaultAudio() })
  } else if (active === 'speed') {
    body = <SpeedTab clip={clip} track={track} project={project} />
    reset = () => exec({ op: 'set_speed', clip_id: clip.id, rate: 1, reverse: false })
  } else if (active === 'animation') {
    body = <AnimationTab clip={clip} media={media} />
    reset = () => {
      const anims = (clip.effects ?? []).filter(e => e.kind.startsWith('jb.anim.'))
      if (anims.length) exec({ op: 'batch', commands: anims.map(e => ({ op: 'remove_effect', clip_id: clip.id, effect_id: e.id })) })
    }
  } else {
    body = <ColorTab clip={clip} media={media} />
    const fx = colorEffect(clip)
    reset = () => fx && exec({ op: 'remove_effect', clip_id: clip.id, effect_id: fx.id })
    extraFooter = <button className="btn-accent">Save as Custom</button>
  }
  return (
    <section className="panel properties">
      <div className="panel-head prop-tabs">
        {tabs.map(t => (
          <button key={t.id} className={t.id === active ? 'on' : ''} onClick={() => setState({ propsTab: t.id })}>
            {t.label}
          </button>
        ))}
      </div>
      {body}
      {reset && (
        <div className="prop-footer">
          <button className="btn-gray" onClick={reset}>Reset</button>
          {extraFooter}
        </div>
      )}
    </section>
  )
}

function SubTabs({ tabs, value, onChange, more, compare }: { tabs: string[]; value: string; onChange: (v: string) => void; more?: boolean; compare?: boolean }) {
  return (
    <div className="sub-tabs">
      {tabs.map(t => (
        <button key={t} className={t === value ? 'on' : ''} onClick={() => onChange(t)}>
          {t}
        </button>
      ))}
      {more && (
        <span className="sub-more">
          <i />
          <IcDoubleRight size={16} />
        </span>
      )}
      {compare && (
        <span className="sub-compare">
          <IcCompare size={18} />
          <IcChevronDown size={14} />
        </span>
      )}
    </div>
  )
}

// ---------- Video ----------

const p = (v: Param | undefined, d = 0) => paramValue(v, d)

function VideoTab({ clip, project }: { clip: Clip; project: Project }) {
  const [sub, setSub] = useState('Basic')
  return (
    <>
      <SubTabs tabs={['Basic', 'Mask', 'Matting']} value={sub} onChange={setSub} />
      <div className="prop-scroll">{sub === 'Basic' ? <VideoBasic clip={clip} project={project} /> : sub === 'Mask' ? <MaskPane clip={clip} /> : <MattingPane clip={clip} />}</div>
    </>
  )
}

/** Live value while dragging a slider, committed as one command on release. */
function useDraft<T>(value: T): [T, (v: T | null) => void] {
  const [draft, setDraft] = useState<T | null>(null)
  useEffect(() => setDraft(null), [value])
  return [draft ?? value, setDraft]
}

function VideoBasic({ clip, project }: { clip: Clip; project: Project }) {
  const tr = clip.transform ?? defaultTransform()
  const [linked, setLinked] = useState(true)
  const [draft, setDraft] = useDraft(tr)
  const [opacity, setOpacity] = useDraft(p(clip.opacity, 1))
  const W = project.settings.width
  const H = project.settings.height
  const commit = (next: Transform) => exec({ op: 'set_transform', clip_id: clip.id, transform: next })
  const scale = (axis: 'x' | 'y', pct: number, live: boolean) => {
    const v = pct / 100
    const next = { ...draft, ...(axis === 'x' || linked ? { scale_x: float(v) } : {}), ...(axis === 'y' || linked ? { scale_y: float(v) } : {}) }
    if (live) setDraft(next)
    else commit(next)
  }
  const set = (patch: Partial<Transform>) => commit({ ...tr, ...patch })
  const fx = (kind: string) => (clip.effects ?? []).find(e => e.kind === kind)
  const toggleFx = (kind: string, on: boolean, params: Record<string, Param> = {}) => {
    const e = fx(kind)
    if (on && !e) exec({ op: 'add_effect', clip_id: clip.id, effect: { id: newId(), kind, enabled: true, params } })
    if (!on && e) exec({ op: 'remove_effect', clip_id: clip.id, effect_id: e.id })
  }
  const enhance = fx('jb.auto_enhance')
  return (
    <>
      <Module title="Transform" toggle onToggle={() => {}} keyframe>
        <div className="prow label-only">Scale</div>
        <div className="scale-group">
          <button className={`scale-lock ${linked ? 'on' : ''}`} onClick={() => setLinked(!linked)} title="Keep proportions">
            <IcLock size={13} />
          </button>
          <SliderRow sublabel label="Width" value={p(draft.scale_x, 1) * 100} min={0} max={500} unit="%" onInput={v => scale('x', v, true)} onCommit={v => scale('x', v, false)} />
          <SliderRow sublabel label="Height" value={p(draft.scale_y, 1) * 100} min={0} max={500} unit="%" onInput={v => scale('y', v, true)} onCommit={v => scale('y', v, false)} />
        </div>
        <div className="prow xy-row">
          <span className="plabel">Position</span>
          <NumberField className="xy" prefix="X" value={p(tr.position_x) * W} onCommit={v => set({ position_x: float(v / W) })} />
          <NumberField className="xy" prefix="Y" value={-p(tr.position_y) * H} onCommit={v => set({ position_y: float(-v / H) })} />
          <span className="pend"><Diamond /></span>
        </div>
        <div className="prow">
          <span className="plabel">Path Curve</span>
          <span className="spacer" />
          <Toggle on={false} disabled />
        </div>
        <div className="prow xy-row">
          <span className="plabel">Anchor Point</span>
          <NumberField className="xy" prefix="X" value={0} onCommit={() => {}} disabled />
          <NumberField className="xy" prefix="Y" value={0} onCommit={() => {}} disabled />
          <span className="pend" />
        </div>
        <div className="prow rotate-row">
          <span className="plabel">Rotate</span>
          <NumberField className="wide" value={p(tr.rotation)} suffix="°" onCommit={v => set({ rotation: float(v) })} />
          <Dial value={p(tr.rotation)} onCommit={v => set({ rotation: float(v) })} />
          <span className="pend"><Diamond /></span>
        </div>
        <div className="prow flip-row">
          <span className="plabel">Flip</span>
          <button className={`sq-btn ${tr.flip_h ? 'on' : ''}`} title="Flip horizontal" onClick={() => set({ flip_h: !tr.flip_h })}><IcFlipH size={18} /></button>
          <button className={`sq-btn ${tr.flip_v ? 'on' : ''}`} title="Flip vertical" onClick={() => set({ flip_v: !tr.flip_v })}><IcFlipV size={18} /></button>
          <button className="sq-btn" title="Rotate 90° clockwise" onClick={() => set({ rotation: float((p(tr.rotation) + 90) % 360) })}><IcRotateCw size={18} /></button>
          <button className="sq-btn" title="Rotate 90° counterclockwise" onClick={() => set({ rotation: float((p(tr.rotation) + 270) % 360) })}><IcRotateCcw size={18} /></button>
        </div>
        <div className="prow corner-row">
          <span className="plabel">Corner Radius</span>
          <NumberField className="wide" prefix={<IcCorner size={14} />} value={p(fx('jb.corner_radius')?.params?.radius)} onCommit={v => {
            const e = fx('jb.corner_radius')
            if (e) exec({ op: 'set_effect_params', clip_id: clip.id, effect_id: e.id, params: { radius: float(v) } })
            else exec({ op: 'add_effect', clip_id: clip.id, effect: { id: newId(), kind: 'jb.corner_radius', enabled: true, params: { radius: float(v) } } })
          }} />
          <IcCorner size={18} className="corner-icon" />
          <span className="pend"><Diamond /></span>
        </div>
      </Module>
      <Module title="Compositing" toggle onToggle={() => {}} keyframe defaultOpen={false}>
        <div className="prow select-row">
          <span className="plabel">Blend Mode</span>
          <Select value={clip.blend_mode ?? 'normal'} options={[{ value: 'normal', label: 'Normal' }]} onChange={() => {}} disabled />
        </div>
        <SliderRow label="Opacity" value={opacity * 100} min={0} max={100} unit="%" onInput={v => setOpacity(v / 100)} onCommit={v => exec({ op: 'set_opacity', clip_id: clip.id, opacity: float(v / 100) })} />
      </Module>
      <Module title="Background" toggle={!!fx('jb.background')} onToggle={on => toggleFx('jb.background', on)} help defaultOpen={false} />
      <Module title="Shadow" toggle={!!fx('jb.shadow')} onToggle={on => toggleFx('jb.shadow', on)} keyframe defaultOpen={false} />
      <Module title="Auto Enhance" toggle={!!enhance} onToggle={on => toggleFx('jb.auto_enhance', on, { amount: float(50) })} keyframe>
        <SliderRow
          label="Amount"
          disabled={!enhance}
          value={p(enhance?.params?.amount, 50)}
          min={0}
          max={100}
          onCommit={v => enhance && exec({ op: 'set_effect_params', clip_id: clip.id, effect_id: enhance.id, params: { amount: float(v) } })}
        />
      </Module>
      <Module title="Motion Blur" toggle={!!fx('jb.motion_blur')} onToggle={on => toggleFx('jb.motion_blur', on)} defaultOpen={false} />
      <Module title="Stabilization" toggle={!!fx('jb.stabilize')} onToggle={on => toggleFx('jb.stabilize', on)} defaultOpen={false} />
      <Module title="Video Denoise" toggle={!!fx('jb.denoise')} onToggle={on => toggleFx('jb.denoise', on)} defaultOpen={false} />
      <Module title="Lens Correction" toggle={!!fx('jb.lens_correction')} onToggle={on => toggleFx('jb.lens_correction', on)} defaultOpen={false} />
    </>
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
        const mv = (ev: PointerEvent) => {
          v = Math.round(((Math.atan2(ev.clientX - cx, cy - ev.clientY) * 180) / Math.PI + 360) % 360)
          ;(e.target as HTMLElement).closest('.dial')?.setAttribute('style', `--rot:${v}deg`)
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

const MASKS = [
  { kind: 'pen', icon: IcPen },
  { kind: 'text', icon: IcTitles },
  { kind: 'rectangle', icon: IcRect },
  { kind: 'ellipse', icon: IcCircle },
  { kind: 'double_line', icon: IcLines },
  { kind: 'single_line', icon: IcLine },
  { kind: 'heart', icon: IcHeart },
  { kind: 'star', icon: IcStar },
  { kind: 'brush', icon: IcBrush, accent: true },
]

function MaskPane({ clip }: { clip: Clip }) {
  const current = (clip.effects ?? []).find(e => e.kind.startsWith('jb.mask.'))
  return (
    <div className="mask-pane">
      <div className="mask-grid">
        {MASKS.map(m => (
          <button
            key={m.kind}
            className={`mask-btn ${current?.kind === `jb.mask.${m.kind}` ? 'on' : ''} ${m.accent ? 'accent' : ''}`}
            title={m.kind.replace('_', ' ')}
            onClick={() => {
              const cmds: Command[] = []
              if (current) cmds.push({ op: 'remove_effect', clip_id: clip.id, effect_id: current.id })
              if (current?.kind !== `jb.mask.${m.kind}`) cmds.push({ op: 'add_effect', clip_id: clip.id, effect: { id: newId(), kind: `jb.mask.${m.kind}`, enabled: true, params: {} } })
              exec({ op: 'batch', commands: cmds })
            }}
          >
            <m.icon size={26} strokeWidth={1.1} />
          </button>
        ))}
      </div>
      <div className="prow select-row">
        <span className="plabel">Custom Mask</span>
        <Select value="none" options={[{ value: 'none', label: 'Select a mask' }]} onChange={() => {}} />
        <button className="btn-gray add-btn">Add</button>
      </div>
      <p className="mask-hint">Use preset masks or import an image to create a mask.</p>
    </div>
  )
}

function MattingPane({ clip }: { clip: Clip }) {
  const e = (clip.effects ?? []).find(x => x.kind === 'jb.chroma_key')
  return (
    <Module
      title="Chroma Key"
      toggle={!!e}
      onToggle={on =>
        on
          ? exec({ op: 'add_effect', clip_id: clip.id, effect: { id: newId(), kind: 'jb.chroma_key', enabled: true, params: { tolerance: float(30) } } })
          : e && exec({ op: 'remove_effect', clip_id: clip.id, effect_id: e.id })
      }
    >
      <SliderRow
        label="Tolerance"
        disabled={!e}
        value={p(e?.params?.tolerance, 30)}
        min={0}
        max={100}
        onCommit={v => e && exec({ op: 'set_effect_params', clip_id: clip.id, effect_id: e.id, params: { tolerance: float(v) } })}
      />
    </Module>
  )
}

// ---------- Titles ----------

function TitleTab({ clip }: { clip: Clip }) {
  const src = clip.source.type === 'title' ? clip.source : null
  const [text, setText] = useState<string | null>(null)
  if (!src) return null
  const { type: _t, ...title } = src
  void _t
  const set = (patch: Partial<typeof title>) => exec({ op: 'set_title', clip_id: clip.id, title: { ...title, ...patch } })
  return (
    <div className="prop-scroll">
      <Module title="Text">
        <textarea
          className="title-text"
          value={text ?? title.text}
          onChange={e => setText(e.target.value)}
          onBlur={() => {
            if (text != null && text !== title.text) set({ text })
            setText(null)
          }}
          onKeyDown={e => e.stopPropagation()}
        />
        <SliderRow label="Size" value={title.font_size * 100} min={1} max={40} digits={1} keyframe={false} onCommit={v => set({ font_size: v / 100 })} />
        <div className="prow">
          <span className="plabel">Style</span>
          <button className={`sq-btn ${title.bold ? 'on' : ''}`} onClick={() => set({ bold: !title.bold })}><b>B</b></button>
          <button className={`sq-btn ${title.italic ? 'on' : ''}`} onClick={() => set({ italic: !title.italic })}><i>I</i></button>
          <input
            type="color"
            className="color-input"
            value={'#' + title.color.slice(0, 3).map(c => Math.round(c * 255).toString(16).padStart(2, '0')).join('')}
            onChange={e => {
              const h = e.target.value
              set({ color: [parseInt(h.slice(1, 3), 16) / 255, parseInt(h.slice(3, 5), 16) / 255, parseInt(h.slice(5, 7), 16) / 255, 1] })
            }}
          />
        </div>
        <div className="prow xy-row">
          <span className="plabel">Position</span>
          <NumberField className="xy" prefix="X" value={title.position[0]} onCommit={v => set({ position: [v, title.position[1]] })} />
          <NumberField className="xy" prefix="Y" value={title.position[1]} onCommit={v => set({ position: [title.position[0], v] })} />
          <span className="pend" />
        </div>
      </Module>
    </div>
  )
}

// ---------- Audio ----------

function AudioTab({ clip, media }: { clip: Clip; media: MediaAsset | null }) {
  const [sub, setSub] = useState('Basic')
  const audio: AudioSettings = clip.audio ?? defaultAudio()
  const [draft, setDraft] = useDraft(audio)
  const commit = (patch: Partial<AudioSettings>) => exec({ op: 'set_audio', clip_id: clip.id, audio: { ...audio, ...patch } })
  const live = (patch: Partial<AudioSettings>) => setDraft({ ...draft, ...patch })
  const len = seconds(clip.end - clip.start)
  return (
    <>
      <SubTabs tabs={['Basic', 'Denoise', 'Voice Filters', 'Voice C']} value={sub} onChange={setSub} more />
      <div className="prop-scroll">
        {sub === 'Basic' ? (
          <>
            <div className="wave-card">
              <div className="wave-card-head">
                <IcNote size={16} />
                <span>{media?.name.replace(/\.[^.]+$/, '') ?? clip.name}</span>
                <span className="spacer" />
                <Slider value={p(draft.gain_db)} min={-60} max={10} step={0.1} onInput={v => live({ gain_db: float(v) })} onCommit={v => commit({ gain_db: float(v) })} />
              </div>
              {media && <CardWave media={media} clip={clip} />}
            </div>
            <Module title="Adjustment" keyframe>
              <div className="prow select-row">
                <span className="plabel">Audio Channels <IcHelp size={13} /></span>
                <Select value="none" options={[{ value: 'none', label: 'None' }, { value: 'left', label: 'Left to Stereo' }, { value: 'right', label: 'Right to Stereo' }]} onChange={() => {}} />
              </div>
              <div className="prow slider-row balance">
                <span className="plabel">Sound Balance</span>
                <span className="lr">L</span>
                <Slider value={p(draft.pan)} min={-1} max={1} centered onInput={v => live({ pan: float(v) })} onCommit={v => commit({ pan: float(v) })} />
                <span className="lr">R</span>
                <NumberField className="pvalue" value={p(draft.pan) * 100} onCommit={v => commit({ pan: float(Math.max(-100, Math.min(100, v)) / 100) })} />
                <span className="pend"><Diamond /></span>
              </div>
              <SliderRow label="Volume" value={p(draft.gain_db)} min={-60} max={10} unit="dB" onInput={v => live({ gain_db: float(v) })} onCommit={v => commit({ gain_db: float(v) })} />
              <SliderRow label="Fade In" value={seconds(draft.fade_in)} min={0} max={Math.max(0.01, len / 2)} unit="s" keyframe={false} onInput={v => live({ fade_in: ticks(v) })} onCommit={v => commit({ fade_in: ticks(v) })} />
              <SliderRow label="Fade Out" value={seconds(draft.fade_out)} min={0} max={Math.max(0.01, len / 2)} unit="s" keyframe={false} onInput={v => live({ fade_out: ticks(v) })} onCommit={v => commit({ fade_out: ticks(v) })} />
              <SliderRow label="Pitch" value={draft.pitch ?? 0} min={-12} max={12} step={0.1} digits={1} centered onInput={v => live({ pitch: v })} onCommit={v => commit({ pitch: v })} />
              <div className="prow">
                <span className="plabel">Equalizer</span>
                <Select className="eq" value="default" options={[{ value: 'default', label: 'Default' }]} onChange={() => {}} />
                <button className="btn-gray setting">Setting</button>
              </div>
            </Module>
            <Module title="Auto Normalization" toggle={false} help defaultOpen={false} />
          </>
        ) : (
          <EffectToggles clip={clip} items={sub === 'Denoise' ? [['Normal Denoise', 'jb.audio.denoise'], ['Wind Removal', 'jb.audio.wind']] : [['Echo', 'jb.audio.echo'], ['Telephone', 'jb.audio.telephone']]} />
        )}
      </div>
    </>
  )
}

function CardWave({ media, clip }: { media: MediaAsset; clip: Clip }) {
  const [peaks, setPeaks] = useState<number[] | null>(null)
  useEffect(() => {
    let alive = true
    Promise.resolve().then(() =>
      loadWave(media.path).then(w => {
        if (!alive || !w) return
        const from = Math.floor(seconds(clip.source_in ?? 0) * w.pps)
        const to = Math.ceil((seconds(clip.source_in ?? 0) + seconds(sourceLength(clip))) * w.pps)
        const slice = w.peaks.slice(from, to)
        const n = 150
        const out: number[] = []
        for (let i = 0; i < n; i++) {
          let m = 0
          for (let j = Math.floor((i * slice.length) / n); j < Math.floor(((i + 1) * slice.length) / n); j++) m = Math.max(m, slice[j])
          out.push(Math.sqrt(m))
        }
        setPeaks(out)
      }),
    )
    return () => {
      alive = false
    }
  }, [media.path, clip.source_in, clip.end, clip.start])
  return (
    <svg className="wave-card-svg" viewBox="0 0 150 40" preserveAspectRatio="none">
      {peaks && <path d={`M0 40 ${peaks.map((v, i) => `L${i} ${40 - v * 38} L${i + 1} ${40 - v * 38}`).join(' ')} L150 40 Z`} />}
    </svg>
  )
}

function EffectToggles({ clip, items }: { clip: Clip; items: [string, string][] }) {
  return (
    <>
      {items.map(([title, kind]) => {
        const e = (clip.effects ?? []).find(x => x.kind === kind)
        return (
          <Module
            key={kind}
            title={title}
            toggle={!!e}
            defaultOpen={false}
            onToggle={on =>
              on ? exec({ op: 'add_effect', clip_id: clip.id, effect: { id: newId(), kind, enabled: true, params: {} } }) : e && exec({ op: 'remove_effect', clip_id: clip.id, effect_id: e.id })
            }
          />
        )
      })}
    </>
  )
}

// ---------- Speed ----------

// The speed slider is logarithmic around 1x, like Filmora's (0.01x to 100x).
const speedToK = (r: number) => (Math.log10(r) + 2) / 4
const kToSpeed = (k: number) => Math.round(Math.pow(10, k * 4 - 2) * 100) / 100

function SpeedTab({ clip, track, project }: { clip: Clip; track: Track; project: Project }) {
  const [sub, setSub] = useState('Uniform Speed')
  const [ripple, setRipple] = useState(true)
  const [pitch, setPitch] = useState(clip.speed?.maintain_pitch ?? true)
  const rate = clip.speed?.rate ?? 1
  const [k, setK] = useDraft(speedToK(rate))
  const fr = project.settings.frame_rate
  const [dur, setDur] = useState<string | null>(null)

  const apply = (r: number, reverse: boolean | null = null) => {
    const newLen = Math.round(sourceLength(clip) / r)
    const delta = clip.start + newLen - clip.end
    const cmds: Command[] = []
    // Ripple: shift later clips so the retimed clip never overlaps them.
    if (ripple && delta !== 0) {
      const later = track.clips.filter(c => c.start >= clip.end).sort((a, b) => (delta > 0 ? b.start - a.start : a.start - b.start))
      if (delta > 0) for (const c of later) cmds.push({ op: 'move_clip', clip_id: c.id, start: c.start + delta, track_id: null })
      cmds.push({ op: 'set_speed', clip_id: clip.id, rate: r, reverse })
      if (delta < 0) for (const c of later) cmds.push({ op: 'move_clip', clip_id: c.id, start: c.start + delta, track_id: null })
    } else cmds.push({ op: 'set_speed', clip_id: clip.id, rate: r, reverse })
    exec(cmds.length === 1 ? cmds[0] : { op: 'batch', commands: cmds })
  }

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
              <span className="punit" />
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
                  if (secs > 0) apply(Math.max(0.01, Math.min(100, seconds(sourceLength(clip)) / secs)))
                }}
              />
            </div>
            <div className="prow">
              <span className="plabel">Reverse Speed</span>
              <span className="spacer" />
              <Toggle on={!!clip.speed?.reverse} onChange={v => apply(rate, v)} />
            </div>
            <div className="prow">
              <span className="plabel">Ripple Edit</span>
              <span className="spacer" />
              <Toggle on={ripple} onChange={setRipple} />
            </div>
            <div className="prow">
              <span className="plabel">Maintain Pitch</span>
              <span className="spacer" />
              <Toggle on={pitch} onChange={setPitch} />
            </div>
            <div className="prow select-row">
              <span className="plabel">Frame Interpolation <IcHelp size={13} /></span>
              <Select value="sampling" options={[{ value: 'sampling', label: 'Frame Sampling' }, { value: 'blending', label: 'Frame Blending' }, { value: 'flow', label: 'Optical Flow' }]} onChange={() => {}} />
            </div>
          </div>
        ) : (
          <div className="preset-grid">
            {['None', 'Custom', 'Montage', 'Hero Moment', 'Bullet Time', 'Jumper', 'Flash In', 'Flash Out'].map((n, i) => (
              <div key={n} className={`preset ${i === 0 ? 'on' : ''}`}>
                <div className="preset-thumb">{i === 0 ? <IcNone size={34} strokeWidth={1} /> : <RampCurve seed={i} />}</div>
                <span>{n}</span>
              </div>
            ))}
          </div>
        )}
      </div>
    </>
  )
}

function RampCurve({ seed }: { seed: number }) {
  const pts = Array.from({ length: 6 }, (_, i) => `${i * 12 + 2},${30 - ((Math.sin(seed * 1.7 + i * 1.3) + 1) / 2) * 22}`).join(' ')
  return (
    <svg viewBox="0 0 64 34" width="56" height="30">
      <path d="M2 17H62" stroke="#3b4248" strokeDasharray="2 2" />
      <polyline points={pts} fill="none" stroke="var(--accent)" strokeWidth="1.5" />
    </svg>
  )
}

// ---------- Animation ----------

const ANIMS_IN = ['Fade In', 'Zoom In', 'Zoom Out', 'Slide Left', 'Slide Right', 'Slide Up', 'Slide Down', 'Spin', 'Bounce', 'Rise', 'Drop', 'Pop']
const ANIMS_OUT = ['Fade Out', 'Zoom In', 'Zoom Out', 'Slide Left', 'Slide Right', 'Slide Up', 'Slide Down', 'Spin', 'Shrink', 'Sink', 'Lift', 'Blink']
const animKind = (dir: 'in' | 'out', name: string) => `jb.anim.${dir}.${name.toLowerCase().replace(/\s+/g, '_')}`

function AnimationTab({ clip, media }: { clip: Clip; media: MediaAsset | null }) {
  const [sub, setSub] = useState('Motion Presets')
  const [dir, setDir] = useState<'in' | 'out'>('in')
  const current = (clip.effects ?? []).find(e => e.kind.startsWith(`jb.anim.${dir}.`))
  const names = dir === 'in' ? ANIMS_IN : ANIMS_OUT
  const len = clip.end - clip.start
  const pick = (name: string | null) => {
    const cmds: Command[] = []
    if (current) cmds.push({ op: 'remove_effect', clip_id: clip.id, effect_id: current.id })
    if (name) {
      const d = Math.min(10_000_000, Math.floor(len / 2))
      const effect: EffectInstance = { id: newId(), kind: animKind(dir, name), enabled: true, params: {}, range: dir === 'in' ? [0, d] : [len - d, len] }
      cmds.push({ op: 'add_effect', clip_id: clip.id, effect })
    }
    if (cmds.length) exec({ op: 'batch', commands: cmds })
  }
  return (
    <>
      <SubTabs tabs={['Motion Presets', 'Keyframe Presets']} value={sub} onChange={setSub} />
      <div className="prop-scroll">
        <div className="anim-head">
          <Pills value={dir} onChange={setDir} options={[{ value: 'in', label: 'In' }, { value: 'out', label: 'Out' }]} className="inout" />
          <span className="spacer" />
          <IcFilter size={20} className="muted-icon" />
        </div>
        <div className="preset-grid anim">
          <div className={`preset ${!current ? 'on' : ''}`} onClick={() => pick(null)}>
            <div className="preset-thumb"><IcNone size={34} strokeWidth={1} /></div>
            <span>No Motion</span>
          </div>
          {names.map(n => (
            <div key={n} className={`preset ${current?.kind === animKind(dir, n) ? 'on' : ''}`} onClick={() => pick(n)} title={n}>
              <div className="preset-thumb">
                {media && media.kind !== 'audio' ? <img src={backend.media.thumb(media.path, seconds(clip.source_in ?? 0) + 0.5, 90)} alt="" /> : <IcImage size={28} />}
                <span className="dl"><IcDownload size={12} /></span>
              </div>
              <span>{n}</span>
            </div>
          ))}
        </div>
      </div>
    </>
  )
}

// ---------- Color ----------

const COLOR_KIND = 'jb.color.adjust'
const colorEffect = (clip: Clip) => (clip.effects ?? []).find(e => e.kind === COLOR_KIND)

const PRESETS: { name: string; filter: string; params: Record<string, number> }[] = [
  { name: 'None', filter: '', params: {} },
  { name: 'TV', filter: 'saturate(0.7) contrast(1.15) sepia(0.15)', params: { saturation: -30, contrast: 15 } },
  { name: 'Boost Color', filter: 'saturate(1.6)', params: { saturation: 60, vibrance: 30 } },
  { name: 'Darken', filter: 'brightness(0.75) contrast(1.1)', params: { brightness: -25, contrast: 10 } },
  { name: 'Warm', filter: 'sepia(0.3) saturate(1.2)', params: { temperature: 30 } },
  { name: 'Cool', filter: 'hue-rotate(-12deg) saturate(1.1)', params: { temperature: -30 } },
]

function ColorTab({ clip, media }: { clip: Clip; media: MediaAsset | null }) {
  const [sub, setSub] = useState('Basic')
  const [mode, setMode] = useState<'default' | 'custom'>('default')
  const fx = colorEffect(clip)
  const value = (k: string) => p(fx?.params?.[k], 0)
  const [drafts, setDrafts] = useState<Record<string, number>>({})
  useEffect(() => setDrafts({}), [fx])
  const setParams = (params: Record<string, number>) => {
    const pp = Object.fromEntries(Object.entries(params).map(([k, v]) => [k, float(v)]))
    if (fx) exec({ op: 'set_effect_params', clip_id: clip.id, effect_id: fx.id, params: pp })
    else exec({ op: 'add_effect', clip_id: clip.id, effect: { id: newId(), kind: COLOR_KIND, enabled: true, params: pp } })
  }
  const row = (key: string, label: string, gradient?: string, min = -100, max = 100) => (
    <SliderRow
      key={key}
      label={label}
      value={drafts[key] ?? value(key)}
      min={min}
      max={max}
      gradient={gradient}
      onInput={v => setDrafts(d => ({ ...d, [key]: v }))}
      onCommit={v => setParams({ [key]: v })}
    />
  )
  const thumb = media && media.kind !== 'audio' ? backend.media.thumb(media.path, seconds(clip.source_in ?? 0) + 0.5, 90) : null
  return (
    <>
      <SubTabs tabs={['Basic', 'HSL', 'Curves', 'Color']} value={sub} onChange={setSub} more compare />
      <div className="prop-scroll">
        {sub !== 'Basic' ? (
          <p className="mask-hint">{sub} adjustments are coming soon.</p>
        ) : (
          <>
            <Module title="Preset">
              <Pills value={mode} onChange={setMode} className="wide-pills" options={[{ value: 'default', label: 'Default' }, { value: 'custom', label: 'Custom' }]} />
              <div className="color-presets">
                {PRESETS.slice(0, 4).map(pr => (
                  <div key={pr.name} className="preset" onClick={() => (pr.name === 'None' ? fx && exec({ op: 'remove_effect', clip_id: clip.id, effect_id: fx.id }) : setParams(pr.params))}>
                    <div className="preset-thumb">{pr.name === 'None' || !thumb ? <IcNone size={34} strokeWidth={1} /> : <img src={thumb} style={{ filter: pr.filter }} alt="" />}</div>
                    <span>{pr.name}</span>
                  </div>
                ))}
              </div>
              <button className="expand-bar"><IcChevronDown size={12} /></button>
            </Module>
            <Module title="LUT" toggle={false} defaultOpen={false} keyframe />
            <Module title="Color" toggle onToggle={() => {}} keyframe>
              <div className="prow">
                <span className="plabel wide">Auto White Balance</span>
                <span className="spacer" />
                <button className="btn-gray auto-btn">Auto</button>
                <button className="kf"><IcEyedropper size={18} /></button>
              </div>
              {row('temperature', 'Temperature', 'linear-gradient(90deg,#2fa4d9,#7c8a8f 50%,#e8892e)')}
              {row('tint', 'Tint', 'linear-gradient(90deg,#2fbf4a,#7c8a8f 50%,#b440e0)')}
              {row('vibrance', 'Vibrance', 'linear-gradient(90deg,#7c8a8f,#d6a04a 40%,#e040c8 75%,#a040ff)')}
              {row('saturation', 'Saturation', 'linear-gradient(90deg,#7c8a8f,#d6a04a 40%,#e040c8 75%,#a040ff)')}
            </Module>
            <Module title="Light" toggle onToggle={() => {}} keyframe extra={<button className="kf" onClick={() => setParams({ exposure: 0, brightness: 0, contrast: 0, highlight: 0, shadow: 0, white: 0, black: 0 })}><IcReset size={16} /></button>}>
              {row('exposure', 'Exposure')}
              {row('brightness', 'Brightness')}
              {row('contrast', 'Contrast')}
              {row('highlight', 'Highlight')}
              {row('shadow', 'Shadow')}
              {row('white', 'White')}
              {row('black', 'Black')}
            </Module>
          </>
        )}
      </div>
    </>
  )
}
