import { useEffect, useRef, useState } from 'react'
import { backend } from '../backend'
import { mainSequence, seconds, sequenceDuration, ticks, timecode } from '../core/model'
import { createPreview, type PreviewRenderer } from '../preview'
import { getState, seek, setState, stepFrames, stop, subscribe, togglePlay, toast, useEditor } from '../store'
import { Select } from '../ui/controls'
import {
  IcFullscreen,
  IcGear,
  IcMarkIn,
  IcMarkOut,
  IcNextFrame,
  IcPause,
  IcPlay,
  IcPrevFrame,
  IcScope,
  IcSnapshot,
  IcSpeaker,
  IcSpeakerOff,
  IcSplitView,
  IcStop,
} from '../ui/icons'

export function Player() {
  const mode = useEditor(s => s.playerMode)
  const source = useEditor(s => s.sourceMedia)
  const [quality, setQuality] = useState('full')
  const [ratio, setRatio] = useState('16:9')
  return (
    <section className="panel player">
      <div className="panel-head player-head">
        <button className={mode === 'timeline' ? 'on' : ''} onClick={() => setState({ playerMode: 'timeline' })}>Timeline</button>
        <button className={mode === 'source' ? 'on' : ''} onClick={() => source && setState({ playerMode: 'source', playing: false })}>Source</button>
        <span className="spacer" />
        <Select
          className="quality"
          value={quality}
          onChange={setQuality}
          options={[
            { value: 'full', label: 'Full Quality' },
            { value: 'half', label: '1/2 Quality' },
            { value: 'quarter', label: '1/4 Quality' },
          ]}
        />
        <button className="ticon dim" title="Split view"><IcSplitView size={20} /></button>
        <button className="ticon" title="Video scopes"><IcScope size={20} /></button>
      </div>
      {mode === 'source' && source ? <SourceMonitor mediaId={source} /> : <TimelineMonitor ratio={ratio} setRatio={setRatio} />}
    </section>
  )
}

function TimelineMonitor({ ratio, setRatio }: { ratio: string; setRatio: (r: string) => void }) {
  const screen = useRef<HTMLDivElement>(null)
  const renderer = useRef<PreviewRenderer | null>(null)
  const playing = useEditor(s => s.playing)
  const playhead = useEditor(s => s.playhead)
  const project = useEditor(s => s.snap?.project)
  const volume = useEditor(s => s.volume)

  // The renderer follows the store directly (not through React) so playback
  // doesn't re-render the component tree every frame.
  useEffect(() => {
    const r = createPreview()
    renderer.current = r
    r.mount(screen.current!)
    const push = () => {
      const s = getState()
      if (!s.snap) return
      r.update({ project: s.snap.project, time: s.playhead, playing: s.playing, rate: s.rate, volume: s.volume, revision: s.snap.revision })
    }
    push()
    const un = subscribe(push)
    return () => {
      un()
      r.unmount()
    }
  }, [])

  if (!project) return <div className="screen" ref={screen} />
  const fr = project.settings.frame_rate
  const total = sequenceDuration(mainSequence(project))
  return (
    <>
      <div className="screen" ref={screen} onDoubleClick={() => screen.current?.requestFullscreen?.()} />
      <div className="player-bottom">
        <div className="progress-row">
          <Progress value={playhead} max={total} onSeek={seek} />
          <span className="tc-box">{timecode(playhead, fr)}</span>
          <span className="tc-sep">/</span>
          <span className="tc-total">{timecode(total, fr)}</span>
        </div>
        <div className="controls-row">
          <button className="ticon" title="Previous frame (←)" onClick={() => stepFrames(-1)}><IcPrevFrame size={20} /></button>
          <button className="ticon" title="Next frame (→)" onClick={() => stepFrames(1)} disabled={playhead >= total}><IcNextFrame size={20} /></button>
          <button className="ticon" title="Play (Space)" onClick={togglePlay}>{playing ? <IcPause size={20} /> : <IcPlay size={20} />}</button>
          <button className="ticon" title="Stop" onClick={stop}><IcStop size={19} /></button>
          <span className="spacer" />
          <Select
            className="ratio"
            value={ratio}
            onChange={setRatio}
            options={['16:9', '9:16', '1:1', '4:3', '21:9'].map(v => ({ value: v, label: v }))}
          />
          <button className="ticon" title="Mark in"><IcMarkIn size={20} /></button>
          <button className="ticon" title="Mark out"><IcMarkOut size={20} /></button>
          <button className="ticon" title="Settings"><IcGear size={19} /></button>
          <button
            className="ticon"
            title="Snapshot"
            onClick={async () => {
              const blob = await renderer.current?.snapshot?.()
              if (!blob) return toast('Snapshot is not available for this frame')
              const a = document.createElement('a')
              a.href = URL.createObjectURL(blob)
              a.download = `${project.name} ${timecode(playhead, fr).replace(/:/g, '-')}.png`
              a.click()
            }}
          >
            <IcSnapshot size={20} />
          </button>
          <button className="ticon" title="Volume" onClick={() => setState({ volume: volume > 0 ? 0 : 1 })}>
            {volume > 0 ? <IcSpeaker size={20} /> : <IcSpeakerOff size={20} />}
          </button>
          <button className="ticon" title="Full screen" onClick={() => screen.current?.requestFullscreen?.()}><IcFullscreen size={18} /></button>
        </div>
      </div>
    </>
  )
}

function Progress({ value, max, onSeek }: { value: number; max: number; onSeek: (t: number) => void }) {
  const ref = useRef<HTMLDivElement>(null)
  const k = max > 0 ? Math.min(1, value / max) : 0
  const at = (x: number) => {
    const r = ref.current!.getBoundingClientRect()
    return Math.max(0, Math.min(1, (x - r.left) / r.width)) * max
  }
  return (
    <div
      className="progress"
      ref={ref}
      onPointerDown={e => {
        onSeek(at(e.clientX))
        const mv = (ev: PointerEvent) => onSeek(at(ev.clientX))
        const up = () => {
          window.removeEventListener('pointermove', mv)
          window.removeEventListener('pointerup', up)
        }
        window.addEventListener('pointermove', mv)
        window.addEventListener('pointerup', up)
      }}
    >
      <div className="progress-track" />
      <div className="progress-fill" style={{ width: `${k * 100}%` }} />
      <div className="progress-knob" style={{ left: `${k * 100}%` }} />
    </div>
  )
}

/** Plays a single media file from the library ("Source" tab). */
function SourceMonitor({ mediaId }: { mediaId: string }) {
  const media = useEditor(s => s.snap?.project.media.find(m => m.id === mediaId))
  const fr = useEditor(s => s.snap?.project.settings.frame_rate) ?? { num: 30, den: 1 }
  const video = useRef<HTMLVideoElement>(null)
  const [time, setTime] = useState(0)
  const [playing, setPlaying] = useState(false)
  const [range, setRange] = useState<[number, number] | null>(null)
  useEffect(() => {
    const v = video.current
    if (!v) return
    let id = 0
    const loop = () => {
      setTime(v.currentTime)
      id = requestAnimationFrame(loop)
    }
    id = requestAnimationFrame(loop)
    return () => cancelAnimationFrame(id)
  }, [mediaId])
  if (!media) return null
  const total = seconds(media.duration)
  const toggle = () => {
    const v = video.current!
    if (v.paused) v.play().then(() => setPlaying(true), () => {})
    else {
      v.pause()
      setPlaying(false)
    }
  }
  return (
    <>
      <div className="screen source">
        {media.kind === 'image' ? (
          <img className="stage" src={backend.media.file(media.path)} alt="" />
        ) : (
          <video ref={video} className="stage" src={backend.media.file(media.path)} preload="auto" onEnded={() => setPlaying(false)} />
        )}
      </div>
      <div className="player-bottom">
        <div className="progress-row">
          <Progress value={ticks(time)} max={media.duration} onSeek={t => video.current && (video.current.currentTime = seconds(t))} />
          <span className="tc-box">{timecode(ticks(time), fr)}</span>
          <span className="tc-sep">/</span>
          <span className="tc-total">{timecode(ticks(total), fr)}</span>
        </div>
        <div className="controls-row">
          <button className="ticon" onClick={() => video.current && (video.current.currentTime = Math.max(0, time - 1 / 30))}><IcPrevFrame size={20} /></button>
          <button className="ticon" onClick={() => video.current && (video.current.currentTime = time + 1 / 30)}><IcNextFrame size={20} /></button>
          <button className="ticon" onClick={toggle}>{playing ? <IcPause size={20} /> : <IcPlay size={20} />}</button>
          <button className="ticon" onClick={() => video.current && ((video.current.currentTime = 0), video.current.pause(), setPlaying(false))}><IcStop size={19} /></button>
          <span className="spacer" />
          <span className="source-range">{range ? `${timecode(ticks(range[0]), fr)} – ${timecode(ticks(range[1]), fr)}` : ''}</span>
          <button className="ticon" title="Mark in (I)" onClick={() => setRange([time, range?.[1] ?? total])}><IcMarkIn size={20} /></button>
          <button className="ticon" title="Mark out (O)" onClick={() => setRange([range?.[0] ?? 0, time])}><IcMarkOut size={20} /></button>
          <button className="ticon" title="Back to timeline" onClick={() => setState({ playerMode: 'timeline' })}><IcFullscreen size={18} /></button>
        </div>
      </div>
    </>
  )
}
