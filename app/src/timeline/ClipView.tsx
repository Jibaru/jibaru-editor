import { memo, useEffect, useRef, type PointerEvent as RPE, type MouseEvent as RME } from 'react'
import { backend } from '../backend'
import { paramValue, seconds } from '../core/model'
import type { Clip, MediaAsset, Track } from '../core/types'
import { catalogLabel } from '../library'
import { IcNote, IcTitles, IcVideoBadge } from '../ui/icons'
import { drawWave, useWave } from './waves'

export type Edge = 'body' | 'start' | 'end'

interface Props {
  clip: Clip
  track: Track
  media: MediaAsset | null
  left: number
  width: number
  height: number
  pps: number
  selected: boolean
  ghost?: boolean
  /** Visible horizontal window in clip-local pixels. */
  view: [number, number]
  onPointerDown?: (e: RPE, clip: Clip, edge: Edge) => void
  onContextMenu?: (e: RME, clip: Clip) => void
}

function Thumbs({ clip, media, width, height, pps, view }: { clip: Clip; media: MediaAsset; width: number; height: number; pps: number; view: [number, number] }) {
  const aspect = media.width && media.height ? media.width / media.height : 16 / 9
  const tileW = Math.max(16, Math.round(height * aspect))
  const first = Math.max(0, Math.floor(view[0] / tileW))
  const last = Math.min(Math.ceil(width / tileW), Math.ceil(view[1] / tileW))
  const rate = clip.speed?.rate ?? 1
  const tiles = []
  for (let i = first; i < last; i++) {
    const x = i * tileW
    const local = (x / pps) * rate
    const t = media.kind === 'image' ? 0 : Math.round((seconds(clip.source_in ?? 0) + local) * 2) / 2
    tiles.push(
      <img
        key={i}
        src={backend.media.thumb(media.path, Math.min(t, Math.max(0, seconds(media.duration) - 0.1)), 72)}
        style={{ left: x, width: tileW, height }}
        draggable={false}
        alt=""
        onError={e => ((e.target as HTMLImageElement).style.visibility = 'hidden')}
      />,
    )
  }
  return <div className="strip">{tiles}</div>
}

function WaveCanvas({ clip, media, width, view, pps, color }: { clip: Clip; media: MediaAsset; width: number; view: [number, number]; pps: number; color: string }) {
  const wave = useWave(media.has_audio || media.kind === 'audio' ? media.path : null)
  const ref = useRef<HTMLCanvasElement>(null)
  const x0 = Math.max(0, view[0])
  const x1 = Math.min(width, view[1])
  const rate = clip.speed?.rate ?? 1
  const gain = Math.pow(10, paramValue(clip.audio?.gain_db, 0) / 20)
  useEffect(() => {
    if (!wave || !ref.current) return
    const src0 = seconds(clip.source_in ?? 0) + (x0 / pps) * rate
    const src1 = seconds(clip.source_in ?? 0) + (x1 / pps) * rate
    drawWave(ref.current, wave, src0, src1, color, gain)
  }, [wave, x0, x1, pps, rate, clip.source_in, color, gain])
  if (x1 <= x0) return null
  return <canvas ref={ref} className="wave" style={{ left: x0, width: x1 - x0 }} />
}

export const ClipView = memo(function ClipView({ clip, track, media, left, width, height, pps, selected, ghost, view, onPointerDown, onContextMenu }: Props) {
  const src = clip.source
  const kind = src.type === 'title' ? 'title' : track.kind === 'audio' ? 'audio' : 'video'
  const tiny = width < 22
  const down = (edge: Edge) => (e: RPE) => {
    if (e.button !== 0) return
    e.stopPropagation()
    onPointerDown?.(e, clip, edge)
  }
  const label = src.type === 'title' ? src.text || 'Title' : clip.name
  const audible = media && (media.has_audio || media.kind === 'audio')
  const fadeIn = clip.audio?.fade_in ?? 0
  const fadeOut = clip.audio?.fade_out ?? 0
  return (
    <div
      className={`clip ${kind} ${selected ? 'sel' : ''} ${ghost ? 'ghost' : ''} ${track.locked ? 'locked' : ''} ${clip.disabled ? 'disabled' : ''}`}
      data-clip={clip.id}
      style={{ left, width: Math.max(2, width), height }}
      onPointerDown={down('body')}
      onContextMenu={e => onContextMenu?.(e, clip)}
    >
      {kind === 'video' && (
        <>
          <div className="clip-head">
            {!tiny && <IcVideoBadge size={15} className="badge" />}
            {width > 40 && <span className="clip-name">{label}</span>}
            {clip.effects?.length ? <span className="fx-dot" title={clip.effects.map(e => catalogLabel(e.kind)).join(', ')} /> : null}
          </div>
          <div className="clip-body">
            {media && media.kind !== 'audio' && <Thumbs clip={clip} media={media} width={width} height={height - 17 - 14} pps={pps} view={view} />}
          </div>
          <div className="clip-audio">
            {audible && media && <WaveCanvas clip={clip} media={media} width={width} view={view} pps={pps} color="#47bea3" />}
            {audible && <div className="vol-line" />}
          </div>
        </>
      )}
      {kind === 'audio' && (
        <>
          <div className="clip-head audio-head">
            {!tiny && <IcNote size={15} className="badge" />}
            {width > 40 && <span className="clip-name">{label}</span>}
          </div>
          <div className="clip-audio tall">
            {media && <WaveCanvas clip={clip} media={media} width={width} view={view} pps={pps} color="#47bea3" />}
            <div className="vol-line" />
          </div>
        </>
      )}
      {kind === 'title' && (
        <div className="clip-head title-head">
          {!tiny && <IcTitles size={14} className="badge" />}
          {width > 30 && <span className="clip-name">{label}</span>}
        </div>
      )}
      {selected && fadeIn >= 0 && kind !== 'title' && <i className="fade-handle in" style={{ left: Math.min(width - 10, (seconds(fadeIn) * pps)) }} />}
      {selected && fadeOut >= 0 && kind !== 'title' && <i className="fade-handle out" style={{ right: Math.min(width - 10, seconds(fadeOut) * pps) }} />}
      {clip.transition_out && (
        <div className="transition-mark" style={{ width: Math.max(8, seconds(clip.transition_out.duration) * pps / 2) }} title={catalogLabel(clip.transition_out.kind)} />
      )}
      {!track.locked && !ghost && (
        <>
          <div className="edge start" onPointerDown={down('start')} />
          <div className="edge end" onPointerDown={down('end')} />
        </>
      )}
    </div>
  )
})
