import { useRef } from 'react'
import { mod } from '../actions'
import { findClip } from '../core/model'
import { deleteSelection, exec, getState, MAX_ZOOM, MIN_ZOOM, openMenu, redo, setState, splitAtPlayhead, toast, undo, useEditor, zoomBy, zoomToFit } from '../store'
import {
  IcAudioBeat,
  IcAutoEdit,
  IcCaretDown,
  IcCrop,
  IcDoubleRight,
  IcDuration,
  IcFit,
  IcFreeze,
  IcGrid,
  IcKeyframeAdd,
  IcLayout,
  IcMarker,
  IcMic,
  IcMixer,
  IcPen,
  IcPip,
  IcPointer,
  IcRedo,
  IcScissors,
  IcShapes,
  IcSnap,
  IcSpeed,
  IcTitles,
  IcTrash,
  IcUndo,
  IcZoomIn,
  IcZoomOut,
} from '../ui/icons'

const later = (what: string) => () => toast(`${what} is not available yet`)

export function TimelineToolbar() {
  const hasSel = useEditor(s => s.selection.length > 0)
  const canUndo = useEditor(s => !!s.snap?.can_undo)
  const canRedo = useEditor(s => !!s.snap?.can_redo)
  const snapping = useEditor(s => s.snapping)
  const zoom = useEditor(s => s.zoom)
  const slider = useRef<HTMLDivElement>(null)
  const k = Math.log(zoom / MIN_ZOOM) / Math.log(MAX_ZOOM / MIN_ZOOM)

  const speedMenu = (e: React.MouseEvent) => {
    const s = getState()
    const p = s.snap?.project
    const clip = p && s.selection[0] ? findClip(p, s.selection[0])?.clip : null
    if (!clip) return
    const r = (e.currentTarget as HTMLElement).getBoundingClientRect()
    openMenu(r.left, r.bottom + 4, [
      ...[0.25, 0.5, 1, 1.5, 2, 4].map(rate => ({
        label: `${rate}x`,
        checked: (clip.speed?.rate ?? 1) === rate,
        action: () => exec({ op: 'set_speed', clip_id: clip.id, rate, reverse: null }),
      })),
      { separator: true as const },
      { label: 'Uniform Speed...', action: () => setState({ propsTab: 'speed' }) },
    ])
  }

  return (
    <div className="tl-toolbar">
      <button className="ticon" title="Layout"><IcLayout size={21} /></button>
      <i className="tb-sep" />
      <button className="ticon on-bg corner" title="Selection tool"><IcPointer size={21} /></button>
      <button className="ticon" title={`Undo (${mod}Z)`} disabled={!canUndo} onClick={undo}><IcUndo size={21} /></button>
      <button className="ticon" title={`Redo (⇧${mod}Z)`} disabled={!canRedo} onClick={redo}><IcRedo size={21} /></button>
      <button className="ticon" title="Delete (⌫)" disabled={!hasSel} onClick={() => deleteSelection(false)}><IcTrash size={21} /></button>
      <button className="ticon" title={`Split (${mod}B)`} onClick={splitAtPlayhead}><IcScissors size={21} /></button>
      {hasSel && <button className="ticon" title="Crop" onClick={later('Crop')}><IcCrop size={21} /></button>}
      <button className="ticon corner" title="Text" onClick={() => setState({ libraryTab: 'titles' })}><IcTitles size={21} /></button>
      <button className="ticon corner" title="Shapes" onClick={later('Shapes')}><IcShapes size={21} /></button>
      <button className="ticon" title="Pen" onClick={later('Pen tool')}><IcPen size={21} /></button>
      {hasSel ? (
        <>
          <button className="ticon corner" title="Speed" onClick={speedMenu}><IcSpeed size={21} /></button>
          <button className="ticon" title="Duration" onClick={() => setState({ propsTab: 'speed' })}><IcDuration size={21} /></button>
          <button className="ticon" title="Add keyframe" onClick={later('Keyframes')}><IcKeyframeAdd size={21} /></button>
          <button className="ticon" title="Freeze frame" onClick={later('Freeze frame')}><IcFreeze size={21} /></button>
          <i className="tb-sep" />
          <button className="ticon" title="Auto edit" onClick={later('Auto edit')}><IcAutoEdit size={21} /></button>
          <button className="ticon" title="More"><IcDoubleRight size={18} /></button>
        </>
      ) : (
        <>
          <i className="tb-sep" />
          <button className="ticon" title="Auto edit" onClick={later('Auto edit')}><IcAutoEdit size={21} /></button>
          <button className="ticon" title="Beat detection" onClick={later('Beat detection')}><IcAudioBeat size={21} /></button>
        </>
      )}
      <span className="spacer" />
      <button className={`ticon ${snapping ? 'accent' : ''}`} title="Snapping (N)" onClick={() => setState({ snapping: !snapping })}><IcSnap size={21} /></button>
      <button className="ticon" title="Marker" onClick={later('Markers')}><IcMarker size={21} /></button>
      <button className="ticon" title="Record voiceover" onClick={later('Voiceover recording')}><IcMic size={21} /></button>
      <button className="ticon" title="Audio mixer" onClick={later('Audio mixer')}><IcMixer size={21} /></button>
      <button className="ticon" title="Picture in picture" onClick={later('Picture in picture')}><IcPip size={21} /></button>
      <button className="ticon" title="Zoom to fit (⇧Z)" onClick={zoomToFit}><IcFit size={21} /></button>
      <button className="ticon" title="Zoom out" onClick={() => zoomBy(1 / 1.5)}><IcZoomOut size={21} /></button>
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
      <button className="ticon" title="Zoom in" onClick={() => zoomBy(1.5)}><IcZoomIn size={21} /></button>
      <button className="ticon grid" title="Track layout"><IcGrid size={21} /></button>
      <button className="ticon xs" title="Options"><IcCaretDown size={12} /></button>
    </div>
  )
}
