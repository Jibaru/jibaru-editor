// Preview driven by the native engine (src-tauri/src/playback.rs): jb-render
// composites every frame (effects, transitions, titles, color) from frames
// decoded by jb-media, and jb-media's mixer plays the audio through cpal.
// The audio device is the master clock; this class only sends transport
// commands and paints the frames the engine publishes on `jb://live`
// (long poll, raw RGBA). While playing, the playhead follows those frames.

import { backend, type LiveFrame, type NativeEngine } from '../backend'
import { getState, setNativeClock, setState } from '../store'
import type { PreviewRenderer, PreviewState } from './types'

const QUALITY_HEIGHT = { full: 1080, half: 540, quarter: 360 } as const

export class NativePreview implements PreviewRenderer {
  private engine: NativeEngine
  private screen: HTMLElement | null = null
  private canvas = document.createElement('canvas')
  private ctx = this.canvas.getContext('2d', { alpha: false })!
  private hud = document.createElement('div')
  private resize = new ResizeObserver(() => this.sendSize())
  private abort = new AbortController()
  private alive = false
  private seq = 0
  private playing = false
  private rate = 1
  /** Time last sent with seek (paused) or play. */
  private sent = -1
  /** Time of the last frame received while playing (the playhead we set). */
  private shown = -1
  private lastSeekAt = 0
  private volume = -1
  private size = ''
  private quality = ''
  private unsub: (() => void)[] = []
  private counter = { frames: 0, total: 0, since: performance.now(), fps: 0, reported: performance.now() }

  constructor(engine: NativeEngine) {
    this.engine = engine
    this.canvas.className = 'stage native-stage'
    this.hud.className = 'preview-hud'
  }

  mount(screen: HTMLElement) {
    this.screen = screen
    screen.appendChild(this.canvas)
    screen.appendChild(this.hud)
    this.resize.observe(screen)
    this.alive = true
    setNativeClock(true)
    this.unsub.push(
      this.engine.onEnded(t => {
        this.playing = false
        this.sent = t
        setState({ playing: false, playhead: t })
      }),
    )
    this.loop()
  }

  unmount() {
    this.alive = false
    this.abort.abort()
    this.resize.disconnect()
    this.unsub.forEach(u => u())
    setNativeClock(false)
    if (this.playing) this.engine.pause()
    this.canvas.remove()
    this.hud.remove()
    this.screen = null
  }

  private sendSize() {
    if (!this.screen) return
    const q = getState().previewQuality
    const dpr = devicePixelRatio || 1
    const cap = QUALITY_HEIGHT[q]
    const h = Math.max(90, Math.min(cap, Math.round(this.screen.clientHeight * dpr)))
    const w = Math.max(160, Math.min(Math.round((cap * 16) / 9) * 2, Math.round(this.screen.clientWidth * dpr)))
    const key = `${w}x${h}`
    if (key === this.size) return
    this.size = key
    this.engine.setSize(w, h)
  }

  update(s: PreviewState) {
    const q = getState().previewQuality
    if (q !== this.quality) {
      this.quality = q
      this.sendSize()
    }
    if (s.volume !== this.volume) {
      this.volume = s.volume
      this.engine.setVolume(s.volume)
    }
    const p = s.project.settings
    this.canvas.style.aspectRatio = `${p.width} / ${p.height}`
    if (s.playing !== this.playing || (s.playing && s.rate !== this.rate)) {
      this.playing = s.playing
      this.rate = s.rate
      if (s.playing) {
        this.sent = s.time
        this.shown = s.time
        this.counter = { frames: 0, total: 0, since: performance.now(), fps: 0, reported: performance.now() }
        this.engine.play(s.time, s.rate)
      } else {
        this.engine.pause()
        // Show exactly the frame the playhead is on now.
        this.sent = s.time
        this.engine.seek(s.time, false)
      }
      return
    }
    if (s.playing) {
      // The user moved the playhead while playing: restart from there.
      if (s.time !== this.shown) {
        this.shown = s.time
        this.engine.play(s.time, s.rate)
      }
      return
    }
    if (s.time !== this.sent) {
      this.sent = s.time
      // Rapid seeks mean the playhead is being dragged: play audio snippets.
      const now = performance.now()
      const scrub = now - this.lastSeekAt < 200
      this.lastSeekAt = now
      this.engine.seek(s.time, scrub)
    }
  }

  private async loop() {
    while (this.alive) {
      let f: LiveFrame | null = null
      try {
        f = await this.engine.nextFrame(this.seq, this.abort.signal)
      } catch {
        if (!this.alive) return
        await new Promise(r => setTimeout(r, 100))
        continue
      }
      if (!f || !this.alive) continue
      this.seq = f.seq
      this.draw(f)
      if (f.playing && this.playing) {
        this.shown = f.time
        setState({ playhead: f.time })
        this.count()
      }
    }
  }

  private draw(f: LiveFrame) {
    if (this.canvas.width !== f.width || this.canvas.height !== f.height) {
      this.canvas.width = f.width
      this.canvas.height = f.height
    }
    this.ctx.putImageData(new ImageData(f.pixels as Uint8ClampedArray<ArrayBuffer>, f.width, f.height), 0, 0)
  }

  private count() {
    const c = this.counter
    c.frames++
    c.total++
    const now = performance.now()
    const el = now - c.since
    if (el >= 1000) {
      c.fps = (c.frames * 1000) / el
      c.frames = 0
      c.since = now
      const s = getState()
      if (s.showStats) {
        this.engine.stats().then(st => {
          this.hud.textContent = `${c.fps.toFixed(1)} fps shown · ${st.fps.toFixed(1)} rendered · ${st.dropped} dropped · ${st.render_ms.toFixed(1)} ms · ${st.width}x${st.height}${st.audio_clock ? ' · audio clock' : ''}`
        })
      }
      if (now - c.reported > 2000) {
        c.reported = now
        this.engine.report(c.fps, c.total)
      }
    }
    this.hud.hidden = !getState().showStats
  }

  pixels(): ImageData | null {
    if (!this.canvas.width || !this.canvas.height) return null
    return this.ctx.getImageData(0, 0, this.canvas.width, this.canvas.height)
  }

  async snapshot(): Promise<Blob | null> {
    // Full-resolution frame from the renderer, not the preview canvas.
    const s = getState()
    const p = s.snap?.project
    if (!p) return null
    try {
      const r = await fetch(backend.media.frame(s.playhead / 10_000_000, p.settings.height, s.snap!.revision))
      return r.ok ? await r.blob() : null
    } catch {
      return null
    }
  }
}
