import { useEffect, useRef, useState } from 'react'
import { backend } from '../backend'
import { fps, mainSequence, seconds, sequenceDuration, ticks, timecode } from '../core/model'
import { ASPECT_RATIOS, ratioOf, sizeForRatio } from '../core/presets'
import { createPreview, type PreviewRenderer } from '../preview'
import {
  addMarkerAtPlayhead,
  appendMedia,
  applyBackend,
  exec,
  getState,
  mark,
  openMenu,
  registerPreviewTransport,
  registerSourceClock,
  seek,
  setPlayerFullscreen,
  setPreviewQuality,
  setState,
  stepFrames,
  stop,
  subscribe,
  togglePlay,
  toast,
  useEditor,
  type PreviewTarget,
} from '../store'
import { Select } from '../ui/controls'
import {
  IcFullscreen,
  IcGear,
  IcMarker,
  IcMarkIn,
  IcMarkOut,
  IcNextFrame,
  IcPause,
  IcPlay,
  IcPlus,
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
  const previewItem = useEditor(s => s.previewItem)
  const quality = useEditor(s => s.previewQuality)
  const full = useEditor(s => s.playerFullscreen)
  const [scopes, setScopes] = useState(false)
  return (
    <section className={`panel player ${full ? 'fullscreen' : ''}`}>
      <div className="panel-head player-head">
        <button className={mode === 'timeline' ? 'on' : ''} onClick={() => setState({ playerMode: 'timeline' })}>
          Timeline
        </button>
        <button
          className={mode === 'source' || mode === 'preview' ? 'on' : ''}
          disabled={!source && !previewItem}
          title={source || previewItem ? 'Source monitor' : 'Double-click a media item, or click an effect, to open it here'}
          onClick={() => {
            if (mode === 'preview') return
            if (previewItem && !source) setState({ playerMode: 'preview', playing: false })
            else if (source) setState({ playerMode: 'source', playing: false })
          }}
        >
          Source
        </button>
        <span className="spacer" />
        <Select
          className="quality"
          value={quality}
          onChange={v => setPreviewQuality(v as 'full' | 'half' | 'quarter')}
          options={[
            { value: 'full', label: 'Full Quality' },
            { value: 'half', label: '1/2 Quality (540p)' },
            { value: 'quarter', label: '1/4 Quality (360p)' },
          ]}
        />
        <button className="ticon dim" title="Split view is not available yet" disabled>
          <IcSplitView size={20} />
        </button>
        <button className={`ticon ${scopes ? 'on' : ''}`} title="Video scopes" onClick={() => setScopes(!scopes)}>
          <IcScope size={20} />
        </button>
      </div>
      {mode === 'preview' && previewItem ? (
        <ItemPreview item={previewItem} />
      ) : mode === 'source' && source ? (
        <SourceMonitor mediaId={source} />
      ) : (
        <TimelineMonitor scopes={scopes} />
      )}
    </section>
  )
}

/** The project's aspect ratio menu (changes the project resolution, like Filmora). */
function RatioMenu() {
  const settings = useEditor(s => s.snap?.project.settings)
  if (!settings) return null
  const current = ratioOf(settings.width, settings.height) ?? 'custom'
  const options = ASPECT_RATIOS.map(r => ({ value: r.value, label: r.value }))
  if (current === 'custom') options.unshift({ value: 'custom', label: `${settings.width}x${settings.height}` })
  return (
    <Select
      className="ratio"
      value={current}
      onChange={v => {
        const r = ASPECT_RATIOS.find(a => a.value === v)
        if (!r) return
        const [width, height] = sizeForRatio(settings.width, settings.height, r.w, r.h)
        exec({ op: 'set_project_settings', width, height })
      }}
      options={options}
    />
  )
}

function TimelineMonitor({ scopes }: { scopes: boolean }) {
  const screen = useRef<HTMLDivElement>(null)
  const renderer = useRef<PreviewRenderer | null>(null)
  const playing = useEditor(s => s.playing)
  const playhead = useEditor(s => s.playhead)
  const project = useEditor(s => s.snap?.project)
  const volume = useEditor(s => s.volume)
  const markIn = useEditor(s => s.markIn)
  const markOut = useEditor(s => s.markOut)
  const showStats = useEditor(s => s.showStats)
  const picking = useEditor(s => s.eyedropper)
  const full = useEditor(s => s.playerFullscreen)
  const [volOpen, setVolOpen] = useState(false)

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
  const snapshot = async () => {
    const blob = await renderer.current?.snapshot?.()
    if (!blob) return toast('Snapshot is not available for this frame')
    const png = blob.type.includes('png') ? blob : await toPng(blob)
    const name = `${project.name} ${timecode(playhead, fr).replace(/:/g, '-')}`
    if (await applyBackend(backend.saveSnapshot(png, name))) toast(backend.engine ? `Snapshot added to Project Media` : `Snapshot saved`)
  }
  const pick = async (e: React.MouseEvent) => {
    const fn = getState().eyedropper
    if (!fn) return
    setState({ eyedropper: null })
    const rgb = await pixelAt(renderer.current, screen.current!, e.clientX, e.clientY)
    if (rgb) fn(rgb)
    else toast('Could not read the color at that point')
  }
  return (
    <>
      <div className={`screen ${picking ? 'picking' : ''}`} ref={screen} onDoubleClick={() => setPlayerFullscreen(!full)} onClick={picking ? pick : undefined}>
        {scopes && <Scopes renderer={renderer} />}
      </div>
      <div className="player-bottom">
        <div className="progress-row">
          <Progress value={playhead} max={total} onSeek={seek} marks={[markIn, markOut]} />
          <span className="tc-box">{timecode(playhead, fr)}</span>
          <span className="tc-sep">/</span>
          <span className="tc-total">{timecode(total, fr)}</span>
        </div>
        <div className="controls-row">
          <button className="ticon" title="Previous frame (←)" onClick={() => stepFrames(-1)} disabled={playhead <= 0}>
            <IcPrevFrame size={20} />
          </button>
          <button className="ticon" title="Next frame (→)" onClick={() => stepFrames(1)} disabled={playhead >= total}>
            <IcNextFrame size={20} />
          </button>
          <button className="ticon" title={playing ? 'Pause (Space)' : 'Play (Space)'} onClick={togglePlay}>
            {playing ? <IcPause size={20} /> : <IcPlay size={20} />}
          </button>
          <button className="ticon" title="Stop" onClick={stop}>
            <IcStop size={19} />
          </button>
          <span className="spacer" />
          <RatioMenu />
          <button className={`ticon ${markIn != null ? 'on' : ''}`} title="Mark in (I)" onClick={() => mark('in')}>
            <IcMarkIn size={20} />
          </button>
          <button className={`ticon ${markOut != null ? 'on' : ''}`} title="Mark out (O)" onClick={() => mark('out')}>
            <IcMarkOut size={20} />
          </button>
          <button className="ticon" title="Add marker (M)" onClick={addMarkerAtPlayhead}>
            <IcMarker size={19} />
          </button>
          <button
            className="ticon corner"
            title="Settings"
            onClick={e => {
              const r = e.currentTarget.getBoundingClientRect()
              openMenu(r.left, r.top - 150, [
                { label: 'Show Playback Statistics', checked: showStats, action: () => setState({ showStats: !showStats }) },
                { separator: true },
                { label: 'Go to Mark In', disabled: markIn == null, action: () => markIn != null && seek(markIn) },
                { label: 'Go to Mark Out', disabled: markOut == null, action: () => markOut != null && seek(markOut) },
                { label: 'Clear In/Out Marks', disabled: markIn == null && markOut == null, action: () => setState({ markIn: null, markOut: null }) },
              ])
            }}
          >
            <IcGear size={19} />
          </button>
          <button className="ticon" title="Snapshot" onClick={snapshot}>
            <IcSnapshot size={20} />
          </button>
          <span className="vol-wrap">
            <button className={`ticon ${volOpen ? 'on' : ''}`} title="Volume" onClick={() => setVolOpen(!volOpen)}>
              {volume > 0 ? <IcSpeaker size={20} /> : <IcSpeakerOff size={20} />}
            </button>
            {volOpen && <VolumePopover volume={volume} onClose={() => setVolOpen(false)} />}
          </span>
          <button className={`ticon ${full ? 'on' : ''}`} title={full ? 'Exit full screen (Esc)' : 'Full screen'} onClick={() => setPlayerFullscreen(!full)}>
            <IcFullscreen size={18} />
          </button>
        </div>
      </div>
    </>
  )
}

function VolumePopover({ volume, onClose }: { volume: number; onClose: () => void }) {
  const ref = useRef<HTMLDivElement>(null)
  const last = useRef(volume || 1)
  useEffect(() => {
    const close = (e: MouseEvent) => {
      if (!ref.current?.contains(e.target as Node) && !(e.target as HTMLElement).closest?.('.vol-wrap')) onClose()
    }
    window.addEventListener('mousedown', close)
    return () => window.removeEventListener('mousedown', close)
  }, [onClose])
  return (
    <div className="vol-pop" ref={ref}>
      <span className="vol-value">{Math.round(volume * 100)}</span>
      <input
        type="range"
        min={0}
        max={1}
        step={0.01}
        value={volume}
        aria-label="Preview volume"
        onChange={e => {
          const v = Number(e.target.value)
          if (v > 0) last.current = v
          setState({ volume: v })
        }}
        onKeyDown={e => e.stopPropagation()}
      />
      <button className="ticon sm" title={volume > 0 ? 'Mute' : 'Unmute'} onClick={() => setState({ volume: volume > 0 ? 0 : last.current })}>
        {volume > 0 ? <IcSpeaker size={16} /> : <IcSpeakerOff size={16} />}
      </button>
    </div>
  )
}

async function toPng(blob: Blob): Promise<Blob> {
  const bmp = await createImageBitmap(blob)
  const c = document.createElement('canvas')
  c.width = bmp.width
  c.height = bmp.height
  c.getContext('2d')!.drawImage(bmp, 0, 0)
  return new Promise(r => c.toBlob(b => r(b ?? blob), 'image/png'))
}

/** Current frame pixels: the renderer's own buffer, or a decoded snapshot. */
async function framePixels(r: PreviewRenderer | null): Promise<ImageData | null> {
  if (!r) return null
  const direct = r.pixels?.()
  if (direct) return direct
  const blob = await r.snapshot?.()
  if (!blob) return null
  const bmp = await createImageBitmap(blob)
  const c = document.createElement('canvas')
  const k = Math.min(1, 480 / bmp.width)
  c.width = Math.max(1, Math.round(bmp.width * k))
  c.height = Math.max(1, Math.round(bmp.height * k))
  const ctx = c.getContext('2d')!
  ctx.drawImage(bmp, 0, 0, c.width, c.height)
  return ctx.getImageData(0, 0, c.width, c.height)
}

/** Average sRGB color (0..1) around a point of the player screen. */
async function pixelAt(r: PreviewRenderer | null, screen: HTMLElement, x: number, y: number): Promise<[number, number, number] | null> {
  const stage = screen.querySelector<HTMLElement>('.stage') ?? screen
  const b = stage.getBoundingClientRect()
  const u = (x - b.left) / b.width
  const v = (y - b.top) / b.height
  if (u < 0 || u > 1 || v < 0 || v > 1) return null
  const img = await framePixels(r).catch(() => null)
  if (!img) return null
  const cx = Math.round(u * (img.width - 1))
  const cy = Math.round(v * (img.height - 1))
  const sum = [0, 0, 0]
  let n = 0
  for (let dy = -2; dy <= 2; dy++)
    for (let dx = -2; dx <= 2; dx++) {
      const px = Math.min(img.width - 1, Math.max(0, cx + dx))
      const py = Math.min(img.height - 1, Math.max(0, cy + dy))
      const i = (py * img.width + px) * 4
      for (let k = 0; k < 3; k++) sum[k] += img.data[i + k]
      n++
    }
  return [sum[0] / n / 255, sum[1] / n / 255, sum[2] / n / 255]
}

/** Video scopes: RGB histogram and luma waveform of the frame on screen. */
function Scopes({ renderer }: { renderer: React.RefObject<PreviewRenderer | null> }) {
  const hist = useRef<HTMLCanvasElement>(null)
  const wave = useRef<HTMLCanvasElement>(null)
  useEffect(() => {
    let alive = true
    let busy = false
    const draw = async () => {
      if (busy || !alive) return
      busy = true
      try {
        const img = await framePixels(renderer.current)
        if (img && alive) drawScopes(img, hist.current!, wave.current!)
      } catch {
        /* frame not ready */
      } finally {
        busy = false
      }
    }
    draw()
    const id = setInterval(draw, 500)
    return () => {
      alive = false
      clearInterval(id)
    }
  }, [renderer])
  return (
    <div className="scopes" onClick={e => e.stopPropagation()}>
      <canvas ref={hist} width={256} height={90} title="RGB histogram" />
      <canvas ref={wave} width={256} height={90} title="Luma waveform" />
    </div>
  )
}

function drawScopes(img: ImageData, hist: HTMLCanvasElement, wave: HTMLCanvasElement) {
  const bins = [new Uint32Array(256), new Uint32Array(256), new Uint32Array(256)]
  const W = wave.width
  const H = wave.height
  const wv = new Uint32Array(W * H)
  const step = Math.max(1, Math.floor((img.width * img.height) / 60000))
  const d = img.data
  for (let p = 0; p < img.width * img.height; p += step) {
    const i = p * 4
    bins[0][d[i]]++
    bins[1][d[i + 1]]++
    bins[2][d[i + 2]]++
    const x = Math.floor(((p % img.width) / img.width) * W)
    const l = 0.2126 * d[i] + 0.7152 * d[i + 1] + 0.0722 * d[i + 2]
    const y = H - 1 - Math.floor((l / 255) * (H - 1))
    wv[y * W + x]++
  }
  const hc = hist.getContext('2d')!
  hc.clearRect(0, 0, hist.width, hist.height)
  const max = Math.max(1, ...bins.flatMap(b => Array.from(b.slice(2, 254))))
  const colors = ['rgba(255,80,80,0.7)', 'rgba(80,255,120,0.7)', 'rgba(90,140,255,0.7)']
  hc.globalCompositeOperation = 'lighter'
  bins.forEach((b, c) => {
    hc.fillStyle = colors[c]
    for (let x = 0; x < 256; x++) {
      const h = Math.min(1, b[x] / max) * hist.height
      hc.fillRect(x, hist.height - h, 1, h)
    }
  })
  const wc = wave.getContext('2d')!
  const out = wc.createImageData(W, H)
  const wmax = Math.max(1, ...Array.from(wv))
  for (let i = 0; i < wv.length; i++) {
    const k = Math.min(1, Math.sqrt(wv[i] / wmax) * 1.5)
    out.data[i * 4] = 120 * k
    out.data[i * 4 + 1] = 255 * k
    out.data[i * 4 + 2] = 160 * k
    out.data[i * 4 + 3] = k > 0 ? 255 : 0
  }
  wc.putImageData(out, 0, 0)
}

function Progress({ value, max, onSeek, marks }: { value: number; max: number; onSeek: (t: number) => void; marks?: [number | null, number | null] }) {
  const ref = useRef<HTMLDivElement>(null)
  const k = max > 0 ? Math.min(1, value / max) : 0
  const at = (x: number) => {
    const r = ref.current!.getBoundingClientRect()
    return Math.max(0, Math.min(1, (x - r.left) / r.width)) * max
  }
  const pct = (t: number | null) => (t == null || max <= 0 ? null : Math.min(100, (t / max) * 100))
  const a = pct(marks?.[0] ?? null)
  const b = pct(marks?.[1] ?? null)
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
      {(a != null || b != null) && <div className="progress-range" style={{ left: `${a ?? 0}%`, width: `${(b ?? 100) - (a ?? 0)}%` }} />}
      <div className="progress-fill" style={{ width: `${k * 100}%` }} />
      <div className="progress-knob" style={{ left: `${k * 100}%` }} />
    </div>
  )
}

const PREVIEW_FPS = 30

/**
 * A library item playing in the player, as when a card is clicked in
 * Filmora: it starts right away, plays once and goes back to its first
 * frame. Transitions, animations and titles play on the sample footage;
 * effects and filters on the timeline at the playhead (5 s). The native
 * engine renders it like the timeline; the browser build plays the card's
 * sprite sheet.
 */
function ItemPreview({ item }: { item: PreviewTarget }) {
  const screen = useRef<HTMLDivElement>(null)
  const canvas = useRef<HTMLCanvasElement>(null)
  const [time, setTime] = useState(0)
  const [dur, setDur] = useState(2 * 10_000_000)
  const [playing, setPlaying] = useState(false)
  const full = useEditor(s => s.playerFullscreen)
  const info = useEditor(s => s.previewInfo)
  const ctl = useRef<{ play(t: number): void; pause(): void; seek(t: number): void }>(null)
  const state = useRef({ time: 0, dur: 2 * 10_000_000, playing: false })
  state.current = { time, dur, playing }
  const fr = { num: PREVIEW_FPS, den: 1 }
  const frame = 10_000_000 / PREVIEW_FPS

  // Back to the timeline when the preview closes.
  useEffect(() => () => void backend.engine?.previewStop(getState().playhead), [])

  useEffect(() => {
    let alive = true
    const c = canvas.current!
    const g = c.getContext('2d', { alpha: false })!
    const engine = backend.engine
    if (engine) {
      // Native: the engine shows the preview project; paint its frames.
      const abort = new AbortController()
      let seq = 0
      let playingNow = false
      const unEnded = engine.onEnded(() => {
        if (!alive) return
        playingNow = false
        setPlaying(false)
        setTime(0)
        engine.seek(0, false)
      })
      ctl.current = {
        play: t => (playingNow = true, setPlaying(true), engine.play(t, 1)),
        pause: () => (playingNow = false, setPlaying(false), engine.pause()),
        seek: t => (setTime(t), engine.seek(t, false)),
      }
      engine.previewStart(item.ref, getState().playhead).then(
        async d => {
          if (!alive) return
          setDur(d)
          ctl.current?.play(0)
          while (alive) {
            const f = await engine.nextFrame(seq, abort.signal).catch(() => null)
            if (!f || !alive) continue
            seq = f.seq
            if (c.width !== f.width || c.height !== f.height) {
              c.width = f.width
              c.height = f.height
            }
            g.putImageData(new ImageData(f.pixels as Uint8ClampedArray<ArrayBuffer>, f.width, f.height), 0, 0)
            if (playingNow && f.playing) setTime(f.time)
          }
        },
        e => toast(String(e), 'error'),
      )
      return () => {
        alive = false
        abort.abort()
        unEnded()
      }
    }
    // Browser: step through the card's sprite sheet on a clock.
    if (!info) return
    const L = info.layout
    const img = new Image()
    img.src = backend.media.fxPreview(item.ref, 'sprite', info.version)
    c.width = L.frame_w * 2
    c.height = L.frame_h * 2
    const total = 2 * 10_000_000
    setDur(total)
    let t = 0
    let run = true
    let last = performance.now()
    let raf = 0
    const draw = () => {
      const i = Math.min(L.frames - 1, Math.floor((t / total) * L.frames))
      if (img.complete && img.naturalWidth) g.drawImage(img, (i % L.cols) * L.frame_w, Math.floor(i / L.cols) * L.frame_h, L.frame_w, L.frame_h, 0, 0, c.width, c.height)
    }
    const tick = (now: number) => {
      if (!alive) return
      if (run) {
        t += (now - last) * 10_000
        if (t >= total) {
          t = 0
          run = false
          setPlaying(false)
        }
        setTime(t)
      }
      last = now
      draw()
      raf = requestAnimationFrame(tick)
    }
    ctl.current = {
      play: from => ((t = from), (run = true), setPlaying(true)),
      pause: () => ((run = false), setPlaying(false)),
      seek: to => ((t = Math.max(0, Math.min(total - 1, to))), setTime(t)),
    }
    setPlaying(true)
    raf = requestAnimationFrame(tick)
    return () => {
      alive = false
      cancelAnimationFrame(raf)
    }
  }, [item.key, item.ref, info])

  useEffect(
    () =>
      registerPreviewTransport({
        toggle: () => {
          const s = state.current
          if (s.playing) ctl.current?.pause()
          else ctl.current?.play(s.time >= s.dur - frame ? 0 : s.time)
        },
        stop: () => {
          ctl.current?.pause()
          ctl.current?.seek(0)
        },
        step: n => {
          const s = state.current
          ctl.current?.pause()
          ctl.current?.seek(Math.max(0, Math.min(s.dur - frame, s.time + n * frame + 1)))
        },
      }),
    [frame],
  )

  return (
    <>
      <div className="screen item-preview" ref={screen} data-preview={item.key} onDoubleClick={() => setPlayerFullscreen(!full)}>
        <canvas ref={canvas} className="stage native-stage" />
      </div>
      <div className="player-bottom">
        <div className="progress-row">
          <Progress value={time} max={dur} onSeek={t => (ctl.current?.pause(), ctl.current?.seek(t))} />
          <span className="tc-box">{timecode(time, fr)}</span>
          <span className="tc-sep">/</span>
          <span className="tc-total">{timecode(dur, fr)}</span>
        </div>
        <div className="controls-row">
          <button className="ticon" title="Previous frame (←)" onClick={() => stepFrames(-1)} disabled={time <= 0}>
            <IcPrevFrame size={20} />
          </button>
          <button className="ticon" title="Next frame (→)" onClick={() => stepFrames(1)}>
            <IcNextFrame size={20} />
          </button>
          <button className="ticon" title={playing ? 'Pause (Space)' : 'Play (Space)'} onClick={togglePlay}>
            {playing ? <IcPause size={20} /> : <IcPlay size={20} />}
          </button>
          <button className="ticon" title="Stop" onClick={stop}>
            <IcStop size={19} />
          </button>
          <span className="spacer" />
          <span className="preview-name" title={item.label}>
            {item.label}
          </span>
          <button className="ticon" title="Mark in (not available for previews)" disabled>
            <IcMarkIn size={20} />
          </button>
          <button className="ticon" title="Mark out (not available for previews)" disabled>
            <IcMarkOut size={20} />
          </button>
          <button className="ticon" title="Back to timeline" onClick={() => setState({ playerMode: 'timeline' })}>
            <IcFullscreen size={18} />
          </button>
        </div>
      </div>
    </>
  )
}

/** Plays a single media file from the library ("Source" tab). */
function SourceMonitor({ mediaId }: { mediaId: string }) {
  const media = useEditor(s => s.snap?.project.media.find(m => m.id === mediaId))
  const projectRate = useEditor(s => s.snap?.project.settings.frame_rate) ?? { num: 30, den: 1 }
  const range = useEditor(s => s.sourceRanges[mediaId] ?? null)
  const volume = useEditor(s => s.volume)
  const video = useRef<HTMLVideoElement>(null)
  const [time, setTime] = useState(0)
  const [playing, setPlaying] = useState(false)
  const fr = media?.frame_rate ?? projectRate
  const frame = 1 / (fps(fr) || 30)
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
  useEffect(() => {
    if (video.current) video.current.volume = Math.max(0, Math.min(1, volume))
  }, [volume])
  useEffect(() => registerSourceClock(() => video.current?.currentTime ?? 0), [mediaId])
  if (!media) return null
  const total = seconds(media.duration)
  const setRange = (r: [number, number] | null) => setState(s => {
    const next = { ...s.sourceRanges }
    if (r) next[mediaId] = r
    else delete next[mediaId]
    return { sourceRanges: next }
  })
  const toggle = () => {
    const v = video.current
    if (!v) return
    if (v.paused) v.play().then(() => setPlaying(true), () => {})
    else {
      v.pause()
      setPlaying(false)
    }
  }
  const go = (t: number) => video.current && (video.current.currentTime = Math.max(0, Math.min(total, t)))
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
          <Progress value={ticks(time)} max={media.duration} onSeek={t => go(seconds(t))} marks={range ? [ticks(range[0]), ticks(range[1])] : undefined} />
          <span className="tc-box">{timecode(ticks(time), fr)}</span>
          <span className="tc-sep">/</span>
          <span className="tc-total">{timecode(ticks(total), fr)}</span>
        </div>
        <div className="controls-row">
          <button className="ticon" title="Previous frame" onClick={() => go(time - frame)} disabled={media.kind === 'image'}>
            <IcPrevFrame size={20} />
          </button>
          <button className="ticon" title="Next frame" onClick={() => go(time + frame)} disabled={media.kind === 'image'}>
            <IcNextFrame size={20} />
          </button>
          <button className="ticon" title="Play" onClick={toggle} disabled={media.kind === 'image'}>
            {playing ? <IcPause size={20} /> : <IcPlay size={20} />}
          </button>
          <button className="ticon" title="Stop" onClick={() => video.current && ((video.current.currentTime = range?.[0] ?? 0), video.current.pause(), setPlaying(false))}>
            <IcStop size={19} />
          </button>
          <span className="spacer" />
          <span className="source-range">{range ? `${timecode(ticks(range[0]), fr)} – ${timecode(ticks(range[1]), fr)}` : ''}</span>
          <button className={`ticon ${range ? 'on' : ''}`} title="Mark in (I)" onClick={() => setRange([Math.min(time, (range?.[1] ?? total) - frame), range?.[1] ?? total])} disabled={media.kind === 'image'}>
            <IcMarkIn size={20} />
          </button>
          <button className={`ticon ${range ? 'on' : ''}`} title="Mark out (O)" onClick={() => setRange([range?.[0] ?? 0, Math.max(time, (range?.[0] ?? 0) + frame)])} disabled={media.kind === 'image'}>
            <IcMarkOut size={20} />
          </button>
          <button className="ticon" title="Clear marks" onClick={() => setRange(null)} disabled={!range}>
            <span className="clear-x">×</span>
          </button>
          <button className="ticon" title="Add to timeline (the marked range)" onClick={() => appendMedia(media)}>
            <IcPlus size={18} />
          </button>
          <button className="ticon" title="Back to timeline" onClick={() => setState({ playerMode: 'timeline' })}>
            <IcFullscreen size={18} />
          </button>
        </div>
      </div>
    </>
  )
}
