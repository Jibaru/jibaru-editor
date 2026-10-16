// Properties panel: Project Info when nothing is selected, otherwise the
// tabs of the selected clip (Video/Titles, Audio, Speed, Animation, Color).
// The tabs live in ./props; every control sends jb-core Commands.

import { useState, type ReactNode } from 'react'
import { backend } from '../backend'
import { defaultAudio, defaultTransform, findClip, float, fps, mainSequence, sequenceDuration, timecode } from '../core/model'
import { ratioOf } from '../core/presets'
import type { Clip, Command, MediaAsset, Project, Track } from '../core/types'
import { audioTarget } from '../edit'
import { exec, getState, setState, useEditor, type PropsTab } from '../store'
import { TransitionPanel } from '../timeline/TransitionPanel'
import { IcImage } from '../ui/icons'
import { AnimationTab } from './props/AnimationTab'
import { AudioTab } from './props/AudioTab'
import { ColorTab, isColorEffect, saveColorPreset } from './props/ColorTab'
import { SpeedTab } from './props/SpeedTab'
import { TitleTab } from './props/TitleTab'
import { VideoTab } from './props/VideoTab'

export function Properties() {
  const project = useEditor(s => s.snap?.project)
  const path = useEditor(s => s.snap?.path ?? null)
  const selection = useEditor(s => s.selection)
  const tab = useEditor(s => s.propsTab)
  const transitionSel = useEditor(s => s.transitionSel)
  if (!project) return <section className="panel properties" />
  if (transitionSel) return <TransitionPanel key={transitionSel} project={project} clipId={transitionSel} />
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
          <dd className="path">
            {path ? (
              <button className="link" title="Show in Finder" disabled={!backend.engine} onClick={() => backend.engine?.reveal(path)}>
                {path}
              </button>
            ) : (
              'Not saved yet'
            )}
          </dd>
          <dt>Resolution:</dt>
          <dd>
            {s.width} x {s.height}
            {ratioOf(s.width, s.height) ? ` (${ratioOf(s.width, s.height)})` : ''}
          </dd>
          <dt>Frame Rate:</dt>
          <dd>{Number.isInteger(rate) ? rate : rate.toFixed(2)}fps</dd>
          <dt>Color Space:</dt>
          <dd>SDR - Rec.709</dd>
          <dt>Sample Rate:</dt>
          <dd>{s.sample_rate}Hz</dd>
          <dt>Duration:</dt>
          <dd>{timecode(sequenceDuration(mainSequence(project)), s.frame_rate)}</dd>
          <dt>Thumbnail:</dt>
          <dd>
            <div className="info-thumb">{media && media.kind !== 'audio' ? <img src={backend.media.thumb(media.path, 1, 160)} alt="" /> : <IcImage size={36} />}</div>
            <button className="btn-gray info-edit" disabled title="The project thumbnail is the first video frame; a custom cover is not supported yet">
              Edit
            </button>
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
  if (media?.has_audio || (clip.link_group && audioTarget(clip).id !== clip.id)) tabs.push({ id: 'audio', label: 'Audio' })
  if (media?.kind !== 'image') tabs.push({ id: 'speed', label: 'Speed' })
  tabs.push({ id: 'animation', label: 'Animation' }, { id: 'color', label: 'Color' })
  return tabs
}

/** Panel-only effects that the Video tab's Reset removes (not library effects). */
const VIDEO_PANEL_KINDS = ['jb.corner_radius', 'jb.background', 'jb.shadow', 'jb.auto_enhance', 'jb.motion_blur', 'jb.stabilize', 'jb.denoise', 'jb.lens_correction', 'jb.chroma_key']

function ClipProperties({ project, clip, track, tab }: { project: Project; clip: Clip; track: Track; tab: PropsTab }) {
  const media = clip.source.type === 'media' ? (project.media.find(m => m.id === (clip.source as { media_id: string }).media_id) ?? null) : null
  const tabs = tabsFor(clip, track, media)
  const active = tabs.some(t => t.id === tab) ? tab : tabs[0].id
  const [saving, setSaving] = useState<string | null>(null)
  let body: ReactNode
  let reset: (() => void) | null = null
  let extraFooter: ReactNode = null
  const removeAll = (pred: (kind: string) => boolean): Command[] =>
    (clip.effects ?? []).filter(e => pred(e.kind)).map(e => ({ op: 'remove_effect', clip_id: clip.id, effect_id: e.id }))
  if (active === 'video' && clip.source.type === 'title') {
    body = <TitleTab clip={clip} />
    reset = () => exec({ op: 'set_title_style', clip_id: clip.id, bold: false, italic: false, alignment: 'center', stroke: null, shadow: null, background: null, letter_spacing: 0, line_spacing: 0, position: [0.5, 0.5] })
  } else if (active === 'video') {
    body = <VideoTab clip={clip} project={project} />
    reset = () =>
      exec({
        op: 'batch',
        commands: [
          { op: 'set_transform', clip_id: clip.id, transform: defaultTransform() },
          { op: 'set_opacity', clip_id: clip.id, opacity: float(1) },
          { op: 'set_blend_mode', clip_id: clip.id, blend_mode: 'normal' },
          ...removeAll(k => VIDEO_PANEL_KINDS.includes(k) || k.startsWith('jb.mask.')),
        ],
      })
  } else if (active === 'audio') {
    body = <AudioTab clip={clip} media={media} />
    reset = () => exec({ op: 'set_audio', clip_id: audioTarget(clip).id, audio: defaultAudio() })
  } else if (active === 'speed') {
    body = <SpeedTab clip={clip} track={track} project={project} />
    reset = () => exec(clip.speed?.curve ? { op: 'set_speed_ramp', clip_id: clip.id, preset: 'none' } : { op: 'set_speed', clip_id: clip.id, rate: 1, reverse: false })
  } else if (active === 'animation') {
    body = <AnimationTab clip={clip} media={media} />
    reset = () => {
      const fx = getState().fx
      const cmds: Command[] = [
        ...(clip.animations ?? []).map(a => ({ op: 'remove_animation', clip_id: clip.id, animation_id: a.id }) as Command),
        ...removeAll(k => fx.some(f => f.id === k && f.kind === 'animation')),
      ]
      if (cmds.length) exec({ op: 'batch', commands: cmds })
    }
  } else {
    body = <ColorTab clip={clip} media={media} />
    reset = () => {
      const cmds = (clip.effects ?? []).filter(isColorEffect).map(e => ({ op: 'remove_effect', clip_id: clip.id, effect_id: e.id }) as Command)
      if (cmds.length) exec({ op: 'batch', commands: cmds })
    }
    extraFooter =
      saving == null ? (
        <button className="btn-accent" onClick={() => setSaving('')}>
          Save as Custom
        </button>
      ) : (
        <span className="save-preset">
          <input
            autoFocus
            className="field"
            placeholder="Preset name"
            value={saving}
            onChange={e => setSaving(e.target.value)}
            onKeyDown={e => {
              e.stopPropagation()
              if (e.key === 'Enter' && saving.trim()) {
                saveColorPreset(clip, saving.trim())
                setSaving(null)
              }
              if (e.key === 'Escape') setSaving(null)
            }}
          />
          <button
            className="btn-accent"
            disabled={!saving.trim()}
            onClick={() => {
              saveColorPreset(clip, saving.trim())
              setSaving(null)
            }}
          >
            Save
          </button>
        </span>
      )
  }
  return (
    <section className="panel properties" data-tab={active}>
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
          <button className="btn-gray" onClick={reset}>
            Reset
          </button>
          {extraFooter}
        </div>
      )}
    </section>
  )
}
