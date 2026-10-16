import { useCallback, useEffect, useLayoutEffect, useMemo, useRef, useState, type MouseEvent as RME, type PointerEvent as RPE } from 'react'
import { registerDropTarget, useLibDrag, type LibDrag } from '../drag'
import { displayTracks, findClip, fps, frameTicks, mainSequence, newId, seconds, sequenceDuration, ticks } from '../core/model'
import { TICKS_PER_SECOND, type Clip, type Command, type MediaAsset, type Project, type Ticks, type Track } from '../core/types'
import {
  addMediaToTimeline,
  addTrack,
  copySelection,
  cutSelection,
  deleteSelection,
  duplicateSelection,
  exec,
  getState,
  openMenu,
  paste,
  seek,
  select,
  setState,
  setTimelineViewWidth,
  splitAtPlayhead,
  toast,
  useEditor,
  zoomBy,
} from '../store'
import { sep, type MenuItem } from '../ui/ContextMenu'
import { IcEye, IcEyeOff, IcGear, IcLink, IcLinkOff, IcLock, IcPlus, IcScissors, IcSpeaker, IcSpeakerOff, IcTrackHeight, IcUnlock, IcWand } from '../ui/icons'
import { ClipView, type Edge } from './ClipView'
import { HEADER_W, RULER_H, insertCommands, layoutRows, overlaps, rulerLabel, rulerStep, snap, snapPoints, tx, xt, type Row } from './geometry'
import { TimelineToolbar } from './Toolbar'

type Op =
  | { type: 'move'; ids: string[]; dt: Ticks; trackId: string | null; insertAt: Ticks | null; invalid: boolean }
  | { type: 'trim'; id: string; edge: 'start' | 'end'; start: Ticks; end: Ticks }
  | { type: 'marquee'; x0: number; y0: number; x1: number; y1: number }

const OVERSCAN = 400

export function Timeline() {
  const project = useEditor(s => s.snap?.project)
  const pps = useEditor(s => s.zoom)
  const selection = useEditor(s => s.selection)
  const scroller = useRef<HTMLDivElement>(null)
  const [view, setView] = useState({ left: 0, top: 0, width: 800, height: 300 })
  const [op, setOp] = useState<Op | null>(null)
  const [snapAt, setSnapAt] = useState<Ticks | null>(null)
  const libDrag = useLibDrag()

  useLayoutEffect(() => {
    const el = scroller.current!
    const update = () => {
      setView({ left: el.scrollLeft, top: el.scrollTop, width: el.clientWidth, height: el.clientHeight })
      setTimelineViewWidth(el.clientWidth - HEADER_W)
    }
    update()
    const ro = new ResizeObserver(update)
    ro.observe(el)
    el.addEventListener('scroll', update, { passive: true })
    return () => {
      ro.disconnect()
      el.removeEventListener('scroll', update)
    }
  }, [])

  // Keep the playhead where it is on screen when zooming.
  const prevZoom = useRef(pps)
  useLayoutEffect(() => {
    const el = scroller.current
    if (!el || prevZoom.current === pps) return
    const ph = getState().playhead
    const before = tx(ph, prevZoom.current) - el.scrollLeft
    el.scrollLeft = Math.max(0, tx(ph, pps) - before)
    prevZoom.current = pps
  }, [pps])

  // Cmd/Ctrl + wheel zooms (non-passive so we can stop page zoom).
  useEffect(() => {
    const el = scroller.current!
    const onWheel = (e: WheelEvent) => {
      if (!(e.metaKey || e.ctrlKey)) return
      e.preventDefault()
      zoomBy(Math.exp(-e.deltaY * 0.01))
    }
    el.addEventListener('wheel', onWheel, { passive: false })
    return () => el.removeEventListener('wheel', onWheel)
  }, [])

  const seq = project ? mainSequence(project) : null
  const tracks = useMemo(() => (seq ? displayTracks(seq) : []), [seq])
  const { rows, height: contentH } = useMemo(() => layoutRows(tracks), [tracks])
  const duration = seq ? sequenceDuration(seq) : 0
  const contentW = HEADER_W + Math.max(tx(duration, pps) + 300, view.width)

  const timeAt = useCallback(
    (clientX: number) => {
      const el = scroller.current!
      const r = el.getBoundingClientRect()
      return Math.max(0, xt(clientX - r.left + el.scrollLeft - HEADER_W, getState().zoom))
    },
    [],
  )
  const rowAt = useCallback(
    (clientY: number): Row | null => {
      const el = scroller.current!
      const y = clientY - el.getBoundingClientRect().top + el.scrollTop
      return rows.find(r => y >= r.top - 1 && y < r.top + r.height + 1) ?? null
    },
    [rows],
  )

  // ---------- Library drops ----------
  const dropTarget = useCallback(
    (d: LibDrag): { row: Row | null; time: Ticks; kind: 'video' | 'audio' } | null => {
      const el = scroller.current
      if (!el || !project) return null
      const r = el.getBoundingClientRect()
      if (d.x < r.left + HEADER_W || d.x > r.right || d.y < r.top || d.y > r.bottom) return null
      const media = d.item.type === 'media' ? project.media.find(m => m.id === d.item.id) : null
      const kind = media?.kind === 'audio' ? 'audio' : 'video'
      return { row: rowAt(d.y), time: snapToFrameT(timeAt(d.x), project), kind }
    },
    [project, rowAt, timeAt],
  )

  useEffect(
    () =>
      registerDropTarget(d => {
        const t = dropTarget(d)
        if (!t || !project) return false
        handleDrop(project, d, t.row?.track ?? null, t.time)
        return true
      }),
    [dropTarget, project],
  )

  if (!project || !seq) return <section className="panel timeline" />

  const playhead = getState().playhead
  const sel = new Set(selection)

  // ---------- Clip pointer interactions ----------
  const onClipDown = (e: RPE, clip: Clip, edge: Edge) => {
    const found = findClip(project, clip.id)
    if (!found) return
    const additive = e.metaKey || e.ctrlKey || e.shiftKey
    let ids: string[]
    if (additive) {
      select([clip.id], 'toggle')
      ids = getState().selection
      if (!ids.includes(clip.id)) return
    } else if (!sel.has(clip.id)) {
      select([clip.id])
      ids = [clip.id]
    } else ids = selection
    setState({ propsTab: found.track.kind === 'audio' ? 'audio' : 'video' })
    if (found.track.locked) return
    if (edge === 'body') startMove(e, clip, ids)
    else startTrim(e, clip, found.track, edge)
  }

  const startMove = (e: RPE, anchor: Clip, ids: string[]) => {
    const x0 = e.clientX
    const y0 = e.clientY
    const moving = ids.map(id => findClip(project, id)).filter((x): x is { track: Track; clip: Clip } => !!x && !x.track.locked)
    const movingIds = new Set(moving.map(m => m.clip.id))
    // Linked partners move with their clip in the core; exclude them from snapping too.
    for (const m of moving) if (m.clip.link_group) for (const t of seq.tracks) for (const c of t.clips) if (c.link_group === m.clip.link_group) movingIds.add(c.id)
    const anchorTrack = findClip(project, anchor.id)!.track
    const minStart = Math.min(...moving.map(m => m.clip.start))
    const points = snapPoints(seq, movingIds, playhead)
    let started = false
    let current: Op | null = null
    const mv = (ev: PointerEvent) => {
      if (!started && Math.hypot(ev.clientX - x0, ev.clientY - y0) < 4) return
      started = true
      let dt = xt(ev.clientX - x0, getState().zoom)
      if (minStart + dt < 0) dt = -minStart
      let shown: Ticks | null = null
      if (getState().snapping) {
        const vals = moving.flatMap(m => [m.clip.start + dt, m.clip.end + dt])
        const s = snap(vals, points, getState().zoom)
        if (s && minStart + dt + s.delta >= 0) {
          dt += s.delta
          shown = s.at
        }
      }
      // A single clip can change track (same kind) and insert between clips.
      let trackId: string | null = null
      let insertAt: Ticks | null = null
      let invalid = false
      if (moving.length === 1) {
        const row = rowAt(ev.clientY)
        const target = row && row.track.kind === anchorTrack.kind && !row.track.locked ? row.track : anchorTrack
        if (target.id !== anchorTrack.id) trackId = target.id
        const start = anchor.start + dt
        const hit = overlaps(target, start, start + (anchor.end - anchor.start), movingIds)
        if (hit) {
          const t = timeAt(ev.clientX)
          insertAt = t < (hit.start + hit.end) / 2 ? hit.start : hit.end
          shown = insertAt
        }
      } else {
        invalid = moving.some(m => !!overlaps(m.track, m.clip.start + dt, m.clip.end + dt, movingIds))
      }
      current = { type: 'move', ids: moving.map(m => m.clip.id), dt, trackId, insertAt, invalid }
      setOp(current)
      setSnapAt(shown)
    }
    const up = async () => {
      window.removeEventListener('pointermove', mv)
      window.removeEventListener('pointerup', up)
      setOp(null)
      setSnapAt(null)
      const o = current as Op | null
      if (!started || !o || o.type !== 'move') return
      if (o.invalid) return toast('Clips cannot overlap')
      if (o.insertAt != null) {
        const target = o.trackId ? seq.tracks.find(t => t.id === o.trackId)! : anchorTrack
        await exec({ op: 'batch', commands: insertCommands(target, anchor, anchorTrack, o.insertAt) })
        return
      }
      if (o.dt === 0 && !o.trackId) return
      // Move right-most first when moving right (and vice versa) to avoid transient overlaps.
      const seen = new Set<string>()
      const ordered = [...moving].sort((a, b) => (o.dt > 0 ? b.clip.start - a.clip.start : a.clip.start - b.clip.start))
      const commands: Command[] = []
      for (const m of ordered) {
        const key = m.clip.link_group ?? m.clip.id
        if (seen.has(key)) continue
        seen.add(key)
        commands.push({ op: 'move_clip', clip_id: m.clip.id, start: m.clip.start + o.dt, track_id: m.clip.id === anchor.id ? o.trackId : null })
      }
      await exec(commands.length === 1 ? commands[0] : { op: 'batch', commands })
    }
    window.addEventListener('pointermove', mv)
    window.addEventListener('pointerup', up)
  }

  const startTrim = (e: RPE, clip: Clip, track: Track, edge: 'start' | 'end') => {
    const x0 = e.clientX
    const media = clip.source.type === 'media' ? project.media.find(m => m.id === (clip.source as { media_id: string }).media_id) : null
    const bounded = !!media && media.kind !== 'image'
    const rate = clip.speed?.rate ?? 1
    const minLen = frameTicks(project.settings.frame_rate)
    const prevEnd = Math.max(0, ...track.clips.filter(c => c.id !== clip.id && c.end <= clip.start).map(c => c.end))
    const nextStart = Math.min(Infinity, ...track.clips.filter(c => c.id !== clip.id && c.start >= clip.end).map(c => c.start))
    const lo = bounded ? Math.max(prevEnd, clip.start - Math.floor((clip.source_in ?? 0) / rate)) : prevEnd
    const hi = bounded ? Math.min(nextStart, clip.start + Math.floor((media!.duration - (clip.source_in ?? 0)) / rate)) : nextStart
    const points = snapPoints(seq, new Set([clip.id]), playhead)
    let current: Op | null = null
    const mv = (ev: PointerEvent) => {
      let dt = xt(ev.clientX - x0, getState().zoom)
      let shown: Ticks | null = null
      const base = edge === 'start' ? clip.start : clip.end
      if (getState().snapping) {
        const s = snap([base + dt], points, getState().zoom)
        if (s) {
          dt += s.delta
          shown = s.at
        }
      }
      let start = clip.start
      let end = clip.end
      if (edge === 'start') start = Math.min(clip.end - minLen, Math.max(lo, clip.start + dt))
      else end = Math.max(clip.start + minLen, Math.min(hi, clip.end + dt))
      current = { type: 'trim', id: clip.id, edge, start, end }
      setOp(current)
      setSnapAt(shown)
    }
    const up = async () => {
      window.removeEventListener('pointermove', mv)
      window.removeEventListener('pointerup', up)
      setOp(null)
      setSnapAt(null)
      const o = current as Op | null
      if (!o || o.type !== 'trim') return
      if (o.start === clip.start && o.end === clip.end) return
      await exec({ op: 'trim_clip', clip_id: clip.id, start: o.edge === 'start' ? o.start : null, end: o.edge === 'end' ? o.end : null })
    }
    window.addEventListener('pointermove', mv)
    window.addEventListener('pointerup', up)
  }

  // Marquee selection on empty lane space.
  const onLaneDown = (e: RPE) => {
    if (e.button !== 0) return
    const el = scroller.current!
    const r = el.getBoundingClientRect()
    const cx = (x: number) => x - r.left + el.scrollLeft
    const cy = (y: number) => y - r.top + el.scrollTop
    const x0 = cx(e.clientX)
    const y0 = cy(e.clientY)
    const additive = e.metaKey || e.ctrlKey || e.shiftKey
    const base = additive ? getState().selection : []
    if (!additive) select([])
    let moved = false
    const mv = (ev: PointerEvent) => {
      const x1 = cx(ev.clientX)
      const y1 = cy(ev.clientY)
      if (!moved && Math.hypot(x1 - x0, y1 - y0) < 4) return
      moved = true
      setOp({ type: 'marquee', x0, y0, x1, y1 })
      const [ax, bx] = [Math.min(x0, x1), Math.max(x0, x1)]
      const [ay, by] = [Math.min(y0, y1), Math.max(y0, y1)]
      const hit: string[] = []
      for (const row of rows) {
        if (row.top + row.height < ay || row.top > by) continue
        for (const c of row.track.clips) {
          const l = HEADER_W + tx(c.start, pps)
          const rr = HEADER_W + tx(c.end, pps)
          if (rr >= ax && l <= bx) hit.push(c.id)
        }
      }
      select([...new Set([...base, ...hit])])
    }
    const up = () => {
      window.removeEventListener('pointermove', mv)
      window.removeEventListener('pointerup', up)
      setOp(null)
    }
    window.addEventListener('pointermove', mv)
    window.addEventListener('pointerup', up)
  }

  const scrub = (e: RPE) => {
    if (e.button !== 0) return
    setState({ playing: false })
    seek(timeAt(e.clientX))
    const mv = (ev: PointerEvent) => seek(timeAt(ev.clientX))
    const up = () => {
      window.removeEventListener('pointermove', mv)
      window.removeEventListener('pointerup', up)
    }
    window.addEventListener('pointermove', mv)
    window.addEventListener('pointerup', up)
  }

  // ---------- Context menus ----------
  const clipMenu = (e: RME, clip: Clip) => {
    e.preventDefault()
    e.stopPropagation()
    if (!sel.has(clip.id)) select([clip.id])
    const found = findClip(project, clip.id)!
    const media = clip.source.type === 'media' ? project.media.find(m => m.id === (clip.source as { media_id: string }).media_id) : null
    const ph = getState().playhead
    const items: MenuItem[] = [
      { label: 'Cut', shortcut: '⌘X', action: cutSelection },
      { label: 'Copy', shortcut: '⌘C', action: copySelection },
      { label: 'Paste', shortcut: '⌘V', action: paste, disabled: !getState().clipboard.length },
      { label: 'Duplicate', shortcut: '⌘D', action: duplicateSelection },
      { label: 'Delete', shortcut: '⌫', action: () => deleteSelection(false) },
      { label: 'Ripple Delete', shortcut: '⇧⌫', action: () => deleteSelection(true) },
      sep,
      { label: 'Split', shortcut: '⌘B', action: splitAtPlayhead, disabled: !(ph > clip.start && ph < clip.end) },
      {
        label: 'Speed',
        children: [0.25, 0.5, 1, 1.5, 2, 4].map(r => ({
          label: `${r}x`,
          checked: (clip.speed?.rate ?? 1) === r,
          action: () => exec({ op: 'set_speed', clip_id: clip.id, rate: r, reverse: null }),
        })),
      },
      {
        label: 'Reverse',
        checked: !!clip.speed?.reverse,
        action: () => exec({ op: 'set_speed', clip_id: clip.id, rate: clip.speed?.rate ?? 1, reverse: !clip.speed?.reverse }),
      },
      sep,
      {
        label: clip.audio?.muted ? 'Unmute' : 'Mute',
        disabled: !media || !(media.has_audio || media.kind === 'audio'),
        action: () => clip.audio && exec({ op: 'set_audio', clip_id: clip.id, audio: { ...clip.audio, muted: !clip.audio.muted } }),
      },
      {
        label: 'Remove Transition',
        disabled: !clip.transition_out,
        action: () => exec({ op: 'set_transition', clip_id: clip.id, transition: null }),
      },
      {
        label: 'Remove Effects',
        disabled: !clip.effects?.length,
        action: () => exec({ op: 'batch', commands: (clip.effects ?? []).map(fx => ({ op: 'remove_effect', clip_id: clip.id, effect_id: fx.id })) }),
      },
      sep,
      { label: 'Select All Clips on Track', action: () => select(found.track.clips.map(c => c.id)) },
      { label: 'Properties', action: () => setState({ propsTab: found.track.kind === 'audio' ? 'audio' : 'video' }) },
    ]
    openMenu(e.clientX, e.clientY, items)
  }

  const laneMenu = (e: RME, track: Track | null) => {
    e.preventDefault()
    const t = timeAt(e.clientX)
    openMenu(e.clientX, e.clientY, [
      {
        label: 'Paste',
        shortcut: '⌘V',
        disabled: !getState().clipboard.length,
        action: () => {
          seek(t)
          paste()
        },
      },
      { label: 'Move Playhead Here', action: () => seek(t) },
      sep,
      { label: 'Add Video Track', action: () => addTrack('video') },
      { label: 'Add Audio Track', action: () => addTrack('audio') },
      ...(track ? trackItems(track, seq.tracks) : []),
      sep,
      { label: 'Snapping', checked: getState().snapping, action: () => setState(s => ({ snapping: !s.snapping })) },
    ])
  }

  // ---------- Rendering ----------
  const vx0 = view.left - HEADER_W - OVERSCAN
  const vx1 = view.left + view.width + OVERSCAN
  const mov = op?.type === 'move' ? op : null
  const trim = op?.type === 'trim' ? op : null
  const movingSet = new Set(mov?.ids ?? [])
  const ghostRows = (id: string, track: Track) => (mov && mov.ids.length === 1 && mov.ids[0] === id && mov.trackId ? rows.find(r => r.track.id === mov.trackId)! : rows.find(r => r.track.id === track.id)!)

  const hover = libDrag ? dropTarget(libDrag) : null
  const hoverMedia = libDrag?.item.type === 'media' ? project.media.find(m => m.id === libDrag.item.id) : null

  return (
    <section className="panel timeline">
      <TimelineToolbar />
      <div className="tl-scroll" ref={scroller} onContextMenu={e => laneMenu(e, null)}>
        <div className="tl-content" style={{ width: contentW, height: Math.max(contentH, view.height) }}>
          <div className="tl-ruler-row" style={{ width: contentW }}>
            <div className="tl-corner">
              <button className="ticon sm" title="Timeline settings"><IcGear size={17} /></button>
              <button className="ticon sm" title="Unlink"><IcLinkOff size={17} /></button>
              <button className="ticon sm" title="Link"><IcLink size={17} /></button>
              <button className="ticon sm dim" title="Track height"><IcTrackHeight size={17} /></button>
            </div>
            <Ruler pps={pps} from={view.left - HEADER_W} width={view.width} fpsVal={fps(project.settings.frame_rate)} onDown={scrub} />
            <div className="render-bar" style={{ left: HEADER_W, width: tx(duration, pps) }} />
            <PlayheadHead pps={pps} onDown={scrub} />
          </div>

          <div className="tl-top-gap" style={{ top: RULER_H + 2 }} onPointerDown={onLaneDown} />

          {rows.map((row, i) => (
            <div key={row.track.id} className={`tl-row ${row.track.kind} ${i % 2 ? 'odd' : 'even'}`} style={{ top: row.top, height: row.height, width: contentW }}>
              <TrackHeader track={row.track} tracks={seq.tracks} />
              <div
                className={`tl-lane ${row.track.locked ? 'locked' : ''} ${hover?.row?.track.id === row.track.id ? 'drop' : ''}`}
                onPointerDown={onLaneDown}
                onContextMenu={e => {
                  e.stopPropagation()
                  laneMenu(e, row.track)
                }}
              >
                {row.track.clips.map(c => {
                  const l = tx(c.start, pps)
                  const r = tx(c.end, pps)
                  if (r < vx0 || l > vx1) return null
                  const t = trim && trim.id === c.id ? trim : null
                  const left = t ? tx(t.start, pps) : l
                  const width = t ? tx(t.end, pps) - left : r - l
                  const moving = movingSet.has(c.id)
                  const media = c.source.type === 'media' ? project.media.find(m => m.id === (c.source as { media_id: string }).media_id) ?? null : null
                  return (
                    <ClipView
                      key={c.id}
                      clip={c}
                      track={row.track}
                      media={media}
                      left={left}
                      width={width}
                      height={row.height - 4}
                      pps={pps}
                      selected={sel.has(c.id)}
                      ghost={moving}
                      view={[vx0 - left, vx1 - left]}
                      onPointerDown={onClipDown}
                      onContextMenu={clipMenu}
                    />
                  )
                })}
                {trim && row.track.clips.some(c => c.id === trim.id) && (
                  <div className="trim-tip" style={{ left: tx(trim.edge === 'start' ? trim.start : trim.end, pps) }}>
                    {formatDur(trim.end - trim.start, project)}
                  </div>
                )}
              </div>
            </div>
          ))}

          {/* Clips being dragged, drawn at their target position. */}
          {mov &&
            mov.insertAt == null &&
            mov.ids.map(id => {
              const f = findClip(project, id)
              if (!f) return null
              const row = ghostRows(id, f.track)
              if (!row) return null
              const media = f.clip.source.type === 'media' ? project.media.find(m => m.id === (f.clip.source as { media_id: string }).media_id) ?? null : null
              const left = tx(f.clip.start + mov.dt, pps)
              return (
                <div key={id} className={`drag-layer ${mov.invalid ? 'invalid' : ''}`} style={{ top: row.top, left: HEADER_W, height: row.height }}>
                  <ClipView clip={f.clip} track={row.track} media={media} left={left} width={tx(f.clip.end - f.clip.start, pps)} height={row.height - 4} pps={pps} selected view={[vx0 - left, vx1 - left]} />
                </div>
              )
            })}
          {mov?.insertAt != null && (() => {
            const row = rows.find(r => r.track.id === (mov.trackId ?? findClip(project, mov.ids[0])?.track.id))
            return row ? <div className="insert-line" style={{ left: HEADER_W + tx(mov.insertAt, pps), top: row.top - 2, height: row.height + 4 }} /> : null
          })()}
          {hover && hoverMedia && (() => {
            const row = hover.row && hover.row.track.kind === hover.kind ? hover.row : null
            const top = row ? row.top : hover.kind === 'video' ? RULER_H + 2 + 8 : contentH - 40
            return <div className="drop-ghost" style={{ left: HEADER_W + tx(hover.time, pps), top: top + 2, width: tx(hoverMedia.duration, pps), height: (row?.height ?? 40) - 4 }} />
          })()}
          {snapAt != null && <div className="snap-line" style={{ left: HEADER_W + tx(snapAt, pps), height: Math.max(contentH, view.height) }} />}
          {op?.type === 'marquee' && (
            <div className="marquee" style={{ left: Math.min(op.x0, op.x1), top: Math.min(op.y0, op.y1), width: Math.abs(op.x1 - op.x0), height: Math.abs(op.y1 - op.y0) }} />
          )}
          <PlayheadLine pps={pps} height={Math.max(contentH, view.height)} scroller={scroller} firstRowTop={rows[0]?.top ?? RULER_H + 50} />
        </div>
      </div>
      <div className="tl-footer">
        <button className="add-track" title="Add track" onClick={e => openMenu(e.clientX, e.clientY - 70, [
          { label: 'Add Video Track', action: () => addTrack('video') },
          { label: 'Add Audio Track', action: () => addTrack('audio') },
        ])}>
          <IcPlus size={16} />
        </button>
        <HScroll scroller={scroller} contentW={contentW} view={view} />
      </div>
    </section>
  )
}

function formatDur(t: Ticks, p: Project) {
  const r = fps(p.settings.frame_rate)
  const total = Math.round(seconds(t) * r)
  const f = total % Math.round(r)
  const s = Math.floor(total / Math.round(r))
  const pad = (n: number) => String(n).padStart(2, '0')
  return `${pad(Math.floor(s / 3600))}:${pad(Math.floor(s / 60) % 60)}:${pad(s % 60)}:${pad(f)}`
}

function snapToFrameT(t: Ticks, p: Project): Ticks {
  const f = frameTicks(p.settings.frame_rate)
  return Math.round(t / f) * f
}

function trackItems(track: Track, all: Track[]): MenuItem[] {
  const flags = (patch: { locked?: boolean; muted?: boolean; hidden?: boolean }) =>
    exec({ op: 'set_track_flags', track_id: track.id, locked: patch.locked ?? null, muted: patch.muted ?? null, hidden: patch.hidden ?? null })
  return [
    sep,
    { label: track.locked ? 'Unlock Track' : 'Lock Track', action: () => flags({ locked: !track.locked }) },
    { label: track.muted ? 'Unmute Track' : 'Mute Track', action: () => flags({ muted: !track.muted }) },
    ...(track.kind === 'video' ? [{ label: track.hidden ? 'Show Track' : 'Hide Track', action: () => flags({ hidden: !track.hidden }) }] : []),
    {
      label: 'Delete Track',
      disabled: all.filter(t => t.kind === track.kind).length <= 1 && track.clips.length > 0,
      action: () => exec({ op: 'remove_track', track_id: track.id }),
    },
  ]
}

function TrackHeader({ track, tracks }: { track: Track; tracks: Track[] }) {
  const flags = (patch: { locked?: boolean; muted?: boolean; hidden?: boolean }) =>
    exec({ op: 'set_track_flags', track_id: track.id, locked: patch.locked ?? null, muted: patch.muted ?? null, hidden: patch.hidden ?? null })
  return (
    <div
      className="tl-header"
      onContextMenu={e => {
        e.preventDefault()
        e.stopPropagation()
        openMenu(e.clientX, e.clientY, [
          { label: 'Add Video Track', action: () => addTrack('video') },
          { label: 'Add Audio Track', action: () => addTrack('audio') },
          ...trackItems(track, tracks),
        ])
      }}
    >
      <span className="track-name">{track.name}</span>
      <div className="track-icons">
        <button className={`ticon xs ${track.locked ? 'on' : ''}`} title={track.locked ? 'Unlock track' : 'Lock track'} onClick={() => flags({ locked: !track.locked })}>
          {track.locked ? <IcLock size={18} /> : <IcUnlock size={18} />}
        </button>
        <button className={`ticon xs ${track.muted ? 'on' : ''}`} title={track.muted ? 'Unmute track' : 'Mute track'} onClick={() => flags({ muted: !track.muted })}>
          {track.muted ? <IcSpeakerOff size={18} /> : <IcSpeaker size={18} />}
        </button>
        <button className="ticon xs" title="Auto enhance"><IcWand size={18} /></button>
        {track.kind === 'video' && (
          <button className={`ticon xs ${track.hidden ? 'on' : ''}`} title={track.hidden ? 'Show track' : 'Hide track'} onClick={() => flags({ hidden: !track.hidden })}>
            {track.hidden ? <IcEyeOff size={18} /> : <IcEye size={18} />}
          </button>
        )}
      </div>
    </div>
  )
}

function Ruler({ pps, from, width, fpsVal, onDown }: { pps: number; from: number; width: number; fpsVal: number; onDown: (e: RPE) => void }) {
  const { major, minor } = rulerStep(pps)
  const start = Math.max(0, Math.floor(Math.max(0, from) / pps / minor) * minor)
  const end = (Math.max(0, from) + width + 200) / pps
  const ticksEls = []
  for (let i = Math.round(start / minor); i * minor <= end; i++) {
    const s = i * minor
    const isMajor = i % 5 === 0
    const x = s * pps
    ticksEls.push(
      <div key={i} className={`rtick ${isMajor ? 'major' : ''}`} style={{ left: x }}>
        {isMajor && <span>{rulerLabel(s, major, fpsVal)}</span>}
      </div>,
    )
  }
  return (
    <div className="ruler" onPointerDown={onDown}>
      {ticksEls}
    </div>
  )
}

function PlayheadHead({ pps, onDown }: { pps: number; onDown: (e: RPE) => void }) {
  const ph = useEditor(s => s.playhead)
  return <div className="ph-head" style={{ left: HEADER_W + tx(ph, pps) }} onPointerDown={onDown} />
}

function PlayheadLine({ pps, height, scroller, firstRowTop }: { pps: number; height: number; scroller: React.RefObject<HTMLDivElement | null>; firstRowTop: number }) {
  const ph = useEditor(s => s.playhead)
  const playing = useEditor(s => s.playing)
  const x = HEADER_W + tx(ph, pps)
  // Page the view while playing, like Filmora.
  useEffect(() => {
    const el = scroller.current
    if (!el || !playing) return
    if (x > el.scrollLeft + el.clientWidth - 30) el.scrollLeft = x - HEADER_W - 40
    else if (x < el.scrollLeft + HEADER_W) el.scrollLeft = Math.max(0, x - HEADER_W - 40)
  }, [x, playing, scroller])
  return (
    <>
      <div className="ph-line" style={{ left: x, height }} />
      <button className="ph-split" style={{ left: x, top: firstRowTop + 4 }} title="Split (⌘B)" onPointerDown={e => e.stopPropagation()} onClick={splitAtPlayhead}>
        <IcScissors size={14} />
      </button>
    </>
  )
}

function HScroll({ scroller, contentW, view }: { scroller: React.RefObject<HTMLDivElement | null>; contentW: number; view: { left: number; width: number } }) {
  const ref = useRef<HTMLDivElement>(null)
  const k = Math.min(1, view.width / contentW)
  const pos = contentW > view.width ? view.left / (contentW - view.width) : 0
  return (
    <div className="hscroll" ref={ref}>
      <div
        className="hscroll-thumb"
        style={{ width: `${k * 100}%`, left: `${pos * (1 - k) * 100}%` }}
        onPointerDown={e => {
          const x0 = e.clientX
          const l0 = scroller.current!.scrollLeft
          const w = ref.current!.clientWidth
          const mv = (ev: PointerEvent) => {
            scroller.current!.scrollLeft = l0 + ((ev.clientX - x0) / w) * contentW
          }
          const up = () => {
            window.removeEventListener('pointermove', mv)
            window.removeEventListener('pointerup', up)
          }
          window.addEventListener('pointermove', mv)
          window.addEventListener('pointerup', up)
        }}
      />
    </div>
  )
}

// ---------- Drops from the library ----------

const defaultTitle = {
  text: 'Title',
  font_family: 'Helvetica Neue',
  font_size: 0.09,
  color: [1, 1, 1, 1] as [number, number, number, number],
  position: [0.5, 0.5] as [number, number],
  bold: true,
}

async function handleDrop(project: Project, d: LibDrag, track: Track | null, time: Ticks) {
  const seq = mainSequence(project)
  const item = d.item
  if (item.type === 'media') {
    const m = project.media.find(x => x.id === item.id)
    if (m) await addMediaToTimeline(m as MediaAsset, time, track?.id)
    return
  }
  if (item.type === 'title') {
    const len = 5 * TICKS_PER_SECOND
    const title = { ...defaultTitle, ...(item.id === 'lower_third' ? { position: [0.3, 0.82] as [number, number], font_size: 0.06 } : item.id === 'subtitle' ? { position: [0.5, 0.9] as [number, number], font_size: 0.05, bold: false } : {}) }
    const clip: Clip = { id: newId(), name: item.label, source: { type: 'title', ...title }, start: time, end: time + len }
    const target =
      track && track.kind === 'video' && !track.locked && !overlaps(track, time, time + len, new Set())
        ? track
        : seq.tracks.find(t => t.kind === 'video' && !t.locked && !overlaps(t, time, time + len, new Set()) && t !== seq.tracks.find(v => v.kind === 'video'))
    if (target) {
      if (await exec({ op: 'add_clip', track_id: target.id, clip })) select([clip.id])
      return
    }
    const before = new Set(seq.tracks.map(t => t.id))
    if (!(await exec({ op: 'add_track', sequence_id: null, kind: 'video', index: seq.tracks.filter(t => t.kind === 'video').length }))) return
    const added = mainSequence(getState().snap!.project).tracks.find(t => !before.has(t.id))
    if (added && (await exec({ op: 'add_clip', track_id: added.id, clip }))) select([clip.id])
    return
  }
  if (!track) return toast('Drop onto a clip in the timeline')
  if (item.type === 'transition') {
    // The cut nearest to the drop point, between two touching clips.
    const frame = frameTicks(project.settings.frame_rate)
    let best: Clip | null = null
    for (let i = 0; i + 1 < track.clips.length; i++) {
      const a = track.clips[i]
      const b = track.clips[i + 1]
      if (b.start - a.end > frame) continue
      if (!best || Math.abs(a.end - time) < Math.abs(best.end - time)) best = a
    }
    if (!best || Math.abs(best.end - time) > ticks(3)) return toast('Drop the transition between two adjacent clips')
    const dur = Math.min(TICKS_PER_SECOND, Math.floor((best.end - best.start) / 2))
    await exec({ op: 'set_transition', clip_id: best.id, transition: { id: newId(), kind: item.id, duration: dur, alignment: 'center', params: {} } })
    select([best.id])
    return
  }
  if (item.type === 'effect') {
    const clip = track.clips.find(c => time >= c.start && time < c.end)
    if (!clip || track.kind !== 'video') return toast('Drop the effect onto a video clip')
    await exec({ op: 'add_effect', clip_id: clip.id, effect: { id: newId(), kind: item.id, enabled: true, params: {} } })
    select([clip.id])
  }
}
