import { useEffect, useLayoutEffect, useRef, useState, type ReactNode } from 'react'
import { mod } from '../actions'
import { findClip } from '../core/model'
import { deleteSelection, getState, MAX_ZOOM, MIN_ZOOM, openMenu, redo, setState, splitAtPlayhead, toast, undo, useEditor, zoomBy, zoomToFit } from '../store'
import { sep, type MenuItem } from '../ui/ContextMenu'
import { IcCaretDown, IcCrop, IcDoubleRight, IcDuration, IcFit, IcGrid, IcLayout, IcMarker, IcMic, IcMixer, IcPen, IcPointer, IcRedo, IcScissors, IcShapes, IcSpeed, IcTrash, IcUndo, IcZoomIn, IcZoomOut } from '../ui/icons'
import {
  addKeyframes,
  addMarker,
  applyQuickTransition,
  freezeFrame,
  notSupported,
  openDuration,
  quickText,
  setRate,
  toggleReverse,
} from './edit'
import {
  Ic3dText,
  IcAiExtend,
  IcAiMate,
  IcAiMatting,
  IcAiTracking,
  IcAudioStretch,
  IcColorPalette,
  IcImageToVideo,
  IcKeyframing,
  IcObjectRemover,
  IcQuickPreview,
  IcQuickSplit,
  IcQuickText,
  IcRenderPreview,
  IcSelectBackward,
  IcSelectForward,
  IcShapeArrow,
  IcShapeEllipse,
  IcShapeLine,
  IcShapePolygon,
  IcShapeRect,
  IcShapeStar,
  IcShapeTriangle,
  IcSlideEdit,
  IcSmartBgm,
  IcSnapTool,
  IcSpeechToText,
  IcTextToVideo,
  IcTranslate,
  IcZoomTool,
} from './icons'
import { getUi, setUi, useUi, type MouseTool } from './tlstate'
import { TipButton, type Tip } from './Tooltip'

/** What the toolbar adapts to: the kind of the (last) selected item. */
export type SelKind = 'none' | 'video' | 'audio' | 'title' | 'transition'

interface Item {
  id: string
  tip: Tip
  icon: ReactNode
  /** Menu label when the item overflows into "More". */
  label?: string
  onClick?: (e: React.MouseEvent) => void
  disabled?: boolean
  on?: boolean
  corner?: boolean
  children?: MenuItem[]
}

const SZ = 21

function useSelKind(): SelKind {
  return useEditor(s => {
    if (s.transitionSel) return 'transition'
    const p = s.snap?.project
    const id = s.selection[s.selection.length - 1]
    const f = p && id ? findClip(p, id) : null
    if (!f) return 'none'
    if (f.clip.source.type === 'title') return 'title'
    // Title templates are nested sequences holding a single title clip.
    if (f.clip.source.type === 'sequence') {
      const id = f.clip.source.sequence_id
      const inner = p?.sequences.find(q => q.id === id)?.tracks.flatMap(t => t.clips) ?? []
      if (inner.length > 0 && inner.every(c => c.source.type === 'title')) return 'title'
    }
    return f.track.kind === 'audio' ? 'audio' : 'video'
  })
}

const SPEED_SLOW = [0.5, 0.25, 0.1, 0.05, 0.01]
const SPEED_FAST = [2, 4, 8, 20, 50, 100]

function speedMenu(): MenuItem[] {
  const s = getState()
  const p = s.snap?.project
  const clip = p && s.selection.length ? findClip(p, s.selection[s.selection.length - 1])?.clip : null
  const rate = clip?.speed?.rate ?? 1
  return [
    { label: 'Uniform Speed', shortcut: '^R', action: () => setState({ propsTab: 'speed' }) },
    { label: 'Speed Ramping', action: () => setState({ propsTab: 'speed' }) },
    sep,
    { label: 'Slow', children: SPEED_SLOW.map(r => ({ label: `${r}x`, checked: rate === r, action: () => setRate(r) })) },
    { label: 'Fast', children: SPEED_FAST.map(r => ({ label: `${r}x`, checked: rate === r, action: () => setRate(r) })) },
    { label: 'Normal', checked: rate === 1, action: () => setRate(1) },
    { label: 'Add Freeze Frame', shortcut: '⌥F', action: freezeFrame, disabled: !clip || clip.source.type === 'title' },
    sep,
    { label: 'Reverse', checked: !!clip?.speed?.reverse, action: toggleReverse },
  ]
}

const TOOLS: { id: MouseTool; label: string; key: string; Icon: typeof IcPointer }[] = [
  { id: 'select', label: 'Select', key: 'V', Icon: IcPointer },
  { id: 'split', label: 'Quick Split Mode', key: 'C', Icon: IcQuickSplit },
  { id: 'forward', label: 'Select All Forward', key: 'A', Icon: IcSelectForward },
  { id: 'backward', label: 'Select All Backward', key: '⇧A', Icon: IcSelectBackward },
  { id: 'zoom', label: 'Zoom Tool', key: 'Z', Icon: IcZoomTool },
]

const SHAPES: [string, typeof IcPointer][] = [
  ['Rectangle', IcShapeRect],
  ['Arrow', IcShapeArrow],
  ['Ellipse', IcShapeEllipse],
  ['Triangle', IcShapeTriangle],
  ['Line', IcShapeLine],
  ['Polygon', IcShapePolygon],
  ['Star', IcShapeStar],
]

const shapesUnsupported = () => toast('Drawing shapes on the canvas is not supported yet')

function dropdown(e: React.MouseEvent, items: MenuItem[]) {
  const r = (e.currentTarget as HTMLElement).getBoundingClientRect()
  openMenu(r.left, r.bottom + 6, items)
}

export function TimelineToolbar() {
  const kind = useSelKind()
  const hasSel = useEditor(s => s.selection.length > 0 || !!s.transitionSel)
  const canUndo = useEditor(s => !!s.snap?.can_undo)
  const canRedo = useEditor(s => !!s.snap?.can_redo)
  const snapping = useEditor(s => s.snapping)
  const zoom = useEditor(s => s.zoom)
  const tool = useUi(s => s.tool)
  const slideEdit = useUi(s => s.slideEdit)
  const quickPreview = useUi(s => s.quickPreview)
  const meter = useUi(s => s.meter)
  const slider = useRef<HTMLDivElement>(null)
  const bar = useRef<HTMLDivElement>(null)
  const right = useRef<HTMLDivElement>(null)
  const [slots, setSlots] = useState(15)
  const k = Math.log(zoom / MIN_ZOOM) / Math.log(MAX_ZOOM / MIN_ZOOM)

  // How many left-side buttons fit before the right group (Filmora moves the rest into "More").
  useLayoutEffect(() => {
    const el = bar.current
    if (!el) return
    const measure = () => {
      const free = el.clientWidth - (right.current?.offsetWidth ?? 0) - 90
      setSlots(Math.max(6, Math.floor(free / 32)))
    }
    measure()
    const ro = new ResizeObserver(measure)
    ro.observe(el)
    return () => ro.disconnect()
  }, [])

  useToolbarShortcuts()

  const video = kind === 'video'
  const tools: Item[] = [
    {
      id: 'pointer',
      tip: { label: 'Switching Mouse Status' },
      icon: (() => {
        const T = TOOLS.find(t => t.id === tool)!.Icon
        return <T size={SZ} />
      })(),
      corner: true,
      on: true,
      onClick: e =>
        dropdown(
          e,
          TOOLS.map(t => ({ label: t.label, shortcut: t.key, checked: tool === t.id, icon: <t.Icon size={18} />, action: () => setUi({ tool: t.id }) })),
        ),
    },
    { id: 'undo', tip: { label: 'Undo', keys: ['⌘', 'Z'] }, icon: <IcUndo size={SZ} />, disabled: !canUndo, onClick: undo },
    { id: 'redo', tip: { label: 'Redo', keys: ['⌘', '⇧', 'Z'] }, icon: <IcRedo size={SZ} />, disabled: !canRedo, onClick: redo },
    { id: 'delete', tip: { label: 'Delete', keys: ['⌫'] }, icon: <IcTrash size={SZ} />, disabled: !hasSel, onClick: () => deleteSelection(false) },
    { id: 'split', tip: { label: 'Split', keys: ['⌘', 'B'] }, icon: <IcScissors size={SZ} />, onClick: splitAtPlayhead },
    ...(video ? [{ id: 'crop', tip: { label: 'Crop', keys: ['⌥', 'C'] }, icon: <IcCrop size={SZ} />, onClick: () => openCrop() } as Item] : []),
    {
      id: 'text',
      tip: { label: 'Quick Text', keys: ['T'] },
      icon: <IcQuickText size={SZ} />,
      corner: true,
      onClick: e =>
        dropdown(e, [
          { label: 'Quick Text', shortcut: 'T', icon: <IcQuickText size={18} />, action: quickText },
          { label: 'Quick 3D Text', icon: <Ic3dText size={18} />, action: () => toast('3D titles are not supported yet') },
        ]),
    },
    {
      id: 'shapes',
      tip: { label: 'Drawing Tools' },
      icon: <IcShapes size={SZ} />,
      corner: true,
      onClick: e =>
        dropdown(
          e,
          SHAPES.map(([label, I]) => ({ label, icon: <I size={18} />, action: shapesUnsupported })),
        ),
    },
    { id: 'pen', tip: { label: 'Pen Tool' }, icon: <IcPen size={SZ} />, onClick: () => toast('The pen tool is not supported yet') },
    ...(video || kind === 'audio'
      ? [{ id: 'speed', tip: { label: 'Speed' }, icon: <IcSpeed size={SZ} />, corner: true, onClick: (e: React.MouseEvent) => dropdown(e, speedMenu()) } as Item]
      : []),
    ...(kind !== 'none' ? [{ id: 'duration', tip: { label: 'Duration' }, icon: <IcDuration size={SZ} />, label: 'Duration', onClick: openDuration } as Item] : []),
    ...(video || kind === 'audio' || kind === 'title'
      ? [{ id: 'keyframe', tip: { label: 'Keyframing', keys: ['⌥', 'Left Click'] }, icon: <IcKeyframing size={SZ} />, label: 'Keyframing', disabled: kind === 'audio', onClick: addKeyframes } as Item]
      : []),
    ...(video ? [{ id: 'slide', tip: { label: 'Slide Edit', keys: ['⌥', 'T'] }, icon: <IcSlideEdit size={SZ} />, label: 'Slide Edit', on: slideEdit, onClick: () => setUi(s => ({ slideEdit: !s.slideEdit })) } as Item] : []),
  ]
  const ai: Item[] = [
    { id: 'ai-extend', tip: { label: 'AI Extend' }, label: 'AI Extend', icon: <IcAiExtend size={SZ} />, onClick: notSupported('AI Extend') },
    { id: 'audio-stretch', tip: { label: 'Audio Stretch', keys: ['S'] }, label: 'Audio Stretch', icon: <IcAudioStretch size={SZ} />, onClick: notSupported('Audio Stretch') },
    { id: 'smart-bgm', tip: { label: 'Smart BGM Generation' }, label: 'Smart BGM Generation', icon: <IcSmartBgm size={SZ} />, onClick: notSupported('Smart BGM Generation') },
    ...(video ? [{ id: 'ai-matting', tip: { label: 'AI Matting Tools' }, label: 'AI Matting Tools', icon: <IcAiMatting size={SZ} />, children: [{ label: 'AI Portrait Cutout', action: notSupported('AI Portrait Cutout') }, { label: 'Smart Cutout', action: notSupported('Smart Cutout') }] } as Item] : []),
    { id: 'object-remover', tip: { label: 'AI Object Remover' }, label: 'AI Object Remover', icon: <IcObjectRemover size={SZ} />, disabled: !video, onClick: notSupported('AI Object Remover') },
    { id: 'stt', tip: { label: 'Speech-to-Text' }, label: 'Speech-to-Text', icon: <IcSpeechToText size={SZ} />, onClick: notSupported('Speech-to-Text') },
    { id: 'translate', tip: { label: 'AI Translation' }, label: 'AI Translation', icon: <IcTranslate size={SZ} />, onClick: notSupported('AI Translation') },
    { id: 'palette', tip: { label: 'AI Color Palette' }, label: 'AI Color Palette', icon: <IcColorPalette size={SZ} />, disabled: !video, onClick: notSupported('AI Color Palette') },
    ...(video ? [{ id: 'ai-tracking', tip: { label: 'AI Tracking Tools' }, label: 'AI Tracking Tools', icon: <IcAiTracking size={SZ} />, children: [{ label: 'Motion Tracking', action: notSupported('Motion Tracking') }, { label: 'Planar Tracking', action: notSupported('Planar Tracking') }] } as Item] : []),
  ]
  const total = tools.length + ai.length
  const fit = total <= slots ? total : slots - 1
  const shownTools = tools.slice(0, fit)
  const shownAi = ai.slice(0, Math.max(0, fit - tools.length))
  const overflow = [...tools.slice(fit), ...ai.slice(Math.max(0, fit - tools.length))]

  const more = (e: React.MouseEvent) =>
    dropdown(
      e,
      overflow.map(it => ({
        label: it.label ?? it.tip.label,
        icon: it.icon,
        disabled: it.disabled,
        checked: it.id === 'slide' ? !!it.on : undefined,
        children: it.children,
        action: it.children ? undefined : () => it.onClick?.({ currentTarget: e.currentTarget } as React.MouseEvent),
      })),
    )

  const btn = (it: Item) => (
    <TipButton
      key={it.id}
      tip={it.tip}
      className={`${it.corner ? 'corner' : ''} ${it.on && it.id !== 'pointer' ? 'active' : ''}`}
      disabled={it.disabled}
      data-tb={it.id}
      onClick={e => (it.children ? dropdown(e, it.children) : it.onClick?.(e))}
    >
      {it.icon}
    </TipButton>
  )

  return (
    <div className="tl-toolbar" ref={bar}>
      <TipButton tip={{ label: 'Custom toolbar' }} data-tb="custom" onClick={e => dropdown(e, customToolbarMenu())}>
        <IcLayout size={SZ} />
      </TipButton>
      <i className="tb-sep" />
      {shownTools.map(btn)}
      {shownAi.length > 0 && <i className="tb-sep" />}
      {shownAi.map(btn)}
      {overflow.length > 0 && (
        <TipButton tip={{ label: 'More' }} data-tb="more" onClick={more}>
          <IcDoubleRight size={18} />
        </TipButton>
      )}
      <span className="spacer" />
      <div className="tb-right" ref={right}>
        <TipButton tip={{ label: 'AI Mate' }} data-tb="ai-mate" className="ai-mate" onClick={notSupported('AI Mate')}>
          <IcAiMate size={24} />
          <i className="free-badge">Free</i>
        </TipButton>
        <TipButton
          tip={{ label: snapping ? 'Disable Timeline Snap' : 'Enable Timeline Snap', keys: ['N'] }}
          data-tb="snap"
          className={snapping ? 'accent' : ''}
          onClick={() => setState({ snapping: !snapping })}
        >
          <IcSnapTool size={SZ} />
        </TipButton>
        <TipButton tip={{ label: 'Text to Video' }} data-tb="t2v" onClick={notSupported('Text to Video')}><IcTextToVideo size={SZ} /></TipButton>
        <TipButton tip={{ label: 'Image to Video' }} data-tb="i2v" onClick={notSupported('Image to Video')}><IcImageToVideo size={SZ} /></TipButton>
        <TipButton tip={{ label: 'Render Preview', keys: ['↵'] }} data-tb="render" onClick={renderPreview}><IcRenderPreview size={SZ} /></TipButton>
        <TipButton tip={{ label: 'Add Marker', keys: ['M'] }} data-tb="marker" onClick={addMarker}><IcMarker size={SZ} /></TipButton>
        <TipButton tip={{ label: 'Record Voiceover', keys: [`${mod}R`] }} data-tb="voiceover" onClick={() => setUi({ dialog: { kind: 'voiceover' } })}><IcMic size={SZ} /></TipButton>
        <TipButton tip={{ label: 'Audio Mixer' }} data-tb="mixer" onClick={() => setUi({ dialog: { kind: 'mixer' } })}><IcMixer size={SZ} /></TipButton>
        <TipButton tip={{ label: 'Quick Preview Mode', keys: ['B'] }} data-tb="quick-preview" className={quickPreview ? 'on-bg' : ''} onClick={() => setUi(s => ({ quickPreview: !s.quickPreview, skim: null }))}>
          <IcQuickPreview size={SZ} />
        </TipButton>
        <TipButton tip={{ label: 'Zoom To Fit Timeline', keys: ['⇧Z'] }} data-tb="fit" onClick={zoomToFit}><IcFit size={SZ} /></TipButton>
        <TipButton tip={{ label: 'Zoom Out', keys: [`${mod}-`] }} data-tb="zoom-out" onClick={() => zoomBy(1 / 1.5)}><IcZoomOut size={SZ} /></TipButton>
        <div
          className="zoom-slider"
          ref={slider}
          onPointerDown={e => {
            const set = (x: number) => {
              const r = slider.current!.getBoundingClientRect()
              const t = Math.min(1, Math.max(0, (x - r.left) / r.width))
              setState({ zoom: MIN_ZOOM * Math.pow(MAX_ZOOM / MIN_ZOOM, t) })
            }
            set(e.clientX)
            const mv = (ev: PointerEvent) => set(ev.clientX)
            const up = () => {
              window.removeEventListener('pointermove', mv)
              window.removeEventListener('pointerup', up)
            }
            window.addEventListener('pointermove', mv)
            window.addEventListener('pointerup', up)
          }}
        >
          <div className="slider-track" />
          <div className="slider-fill" style={{ width: `${k * 100}%` }} />
          <div className="slider-knob" style={{ left: `${k * 100}%` }} />
        </div>
        <TipButton tip={{ label: 'Zoom In', keys: [`${mod}=`] }} data-tb="zoom-in" onClick={() => zoomBy(1.5)}><IcZoomIn size={SZ} /></TipButton>
        <TipButton tip={{ label: meter ? 'Hide Audio Meter' : 'Show Audio Meter' }} data-tb="meter" className={`grid ${meter ? 'on-bg' : ''}`} onClick={() => setUi(s => ({ meter: !s.meter }))}>
          <IcGrid size={SZ} />
        </TipButton>
        <TipButton tip={{ label: 'Timeline Options' }} data-tb="options" className="xs" onClick={e => dropdown(e, optionsMenu())}>
          <IcCaretDown size={12} />
        </TipButton>
      </div>
    </div>
  )
}

function customToolbarMenu(): MenuItem[] {
  return [{ label: 'Reset Toolbar', action: () => toast('The toolbar already shows the default layout') }]
}

function optionsMenu(): MenuItem[] {
  const ui = getUi()
  const s = getState()
  return [
    { label: 'Audio Meter', checked: ui.meter, action: () => setUi({ meter: !ui.meter }) },
    { label: 'Quick Preview Mode', shortcut: 'B', checked: ui.quickPreview, action: () => setUi({ quickPreview: !ui.quickPreview, skim: null }) },
    { label: 'Enable Timeline Snapping', shortcut: 'N', checked: s.snapping, action: () => setState({ snapping: !s.snapping }) },
  ]
}

function renderPreview() {
  // Jibaru renders the preview in real time (GPU), so there is nothing to pre-render.
  toast('The preview is rendered in real time; no pre-render is needed')
}

export function openCrop() {
  const s = getState()
  const p = s.snap?.project
  const id = s.selection[s.selection.length - 1]
  const f = p && id ? findClip(p, id) : null
  if (!f || f.track.kind !== 'video' || f.clip.source.type === 'title') return
  setUi({ dialog: { kind: 'crop', id: f.clip.id } })
}

/** Toolbar keyboard shortcuts (Filmora's defaults). */
function useToolbarShortcuts() {
  useEffect(() => {
    const onKey = (e: KeyboardEvent) => {
      if (e.defaultPrevented || getState().menu || getUi().dialog) return
      const el = e.target as HTMLElement
      if (el.closest?.('input, textarea, select, [contenteditable="true"]')) return
      const k = e.key.toLowerCase()
      const code = e.code
      const run = (fn: () => unknown) => {
        e.preventDefault()
        fn()
      }
      if (e.metaKey || e.ctrlKey) {
        if (e.metaKey && !e.shiftKey && k === 'r') return run(() => setUi({ dialog: { kind: 'voiceover' } }))
        if (e.ctrlKey && !e.metaKey && k === 'r') return run(() => setState({ propsTab: 'speed' }))
        return
      }
      if (e.altKey) {
        if (code === 'KeyF') return run(freezeFrame)
        if (code === 'KeyC') return run(openCrop)
        if (code === 'KeyT') return run(() => setUi(s => ({ slideEdit: !s.slideEdit })))
        return
      }
      if (e.shiftKey) {
        if (k === 't') return run(applyQuickTransition)
        if (k === 'a') return run(() => setUi({ tool: 'backward' }))
        return
      }
      if (k === 'v') return run(() => setUi({ tool: 'select' }))
      if (k === 'c') return run(() => setUi({ tool: 'split' }))
      if (k === 'a') return run(() => setUi({ tool: 'forward' }))
      if (k === 'z') return run(() => setUi({ tool: 'zoom' }))
      if (k === 't') return run(quickText)
      if (k === 'm') return run(addMarker)
      if (k === 'b') return run(() => setUi(s => ({ quickPreview: !s.quickPreview, skim: null })))
      if (k === 'enter') return run(renderPreview)
      if (k === 'escape') return run(() => setState({ transitionSel: null }))
    }
    window.addEventListener('keydown', onKey)
    return () => window.removeEventListener('keydown', onKey)
  }, [])
}

