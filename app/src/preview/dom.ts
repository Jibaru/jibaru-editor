// Preview without a native renderer: every active clip gets a <video>, <audio>,
// <img> or text element, stacked with CSS transforms and kept in sync with the
// playhead. Effects and transitions are not drawn; that is jb-render's job.

import { backend } from '../backend'
import { paramValue, seconds } from '../core/model'
import type { Clip, Project } from '../core/types'
import { audioVoices, visualLayers, type ActiveClip } from './evaluate'
import type { PreviewRenderer, PreviewState } from './types'

type Node = { el: HTMLElement; media: HTMLMediaElement | null; key: string }

const css = (c: [number, number, number, number]) =>
  `rgba(${Math.round(c[0] * 255)},${Math.round(c[1] * 255)},${Math.round(c[2] * 255)},${c[3]})`

export class DomPreview implements PreviewRenderer {
  private screen: HTMLElement | null = null
  private stage = document.createElement('div')
  private nodes = new Map<string, Node>()
  private resize = new ResizeObserver(() => this.layout())
  private last: PreviewState | null = null

  constructor() {
    this.stage.className = 'stage'
  }

  mount(screen: HTMLElement) {
    this.screen = screen
    screen.appendChild(this.stage)
    this.resize.observe(screen)
  }

  unmount() {
    this.resize.disconnect()
    for (const n of this.nodes.values()) this.dispose(n)
    this.nodes.clear()
    this.stage.remove()
    this.screen = null
  }

  private layout() {
    if (!this.screen || !this.last) return
    const { width, height } = this.last.project.settings
    const W = this.screen.clientWidth
    const H = this.screen.clientHeight
    const k = Math.min(W / width, H / height)
    this.stage.style.width = `${Math.floor(width * k)}px`
    this.stage.style.height = `${Math.floor(height * k)}px`
  }

  private dispose(n: Node) {
    if (n.media) {
      n.media.pause()
      n.media.removeAttribute('src')
      n.media.load()
    }
    n.el.remove()
  }

  private nodeFor(a: ActiveClip): Node {
    const src = a.clip.source
    const key =
      src.type === 'media' ? `${a.media?.kind}:${a.media?.path}` : src.type === 'title' ? `title:${JSON.stringify(src)}` : `${src.type}:${JSON.stringify(src)}`
    const existing = this.nodes.get(a.clip.id)
    if (existing && existing.key === key) return existing
    if (existing) this.dispose(existing)
    let el: HTMLElement
    let media: HTMLMediaElement | null = null
    if (src.type === 'media' && a.media) {
      if (a.media.kind === 'image') {
        const img = document.createElement('img')
        img.src = backend.media.file(a.media.path)
        img.draggable = false
        el = img
      } else {
        const v = document.createElement(a.media.kind === 'audio' ? 'audio' : 'video')
        v.preload = 'auto'
        v.src = backend.media.file(a.media.path)
        if (v instanceof HTMLVideoElement) v.playsInline = true
        media = v
        el = v
      }
    } else if (src.type === 'title') {
      el = document.createElement('div')
      el.className = 'layer-title'
      el.textContent = src.text
    } else if (src.type === 'color') {
      el = document.createElement('div')
      el.style.background = css(src.color)
    } else {
      el = document.createElement('div')
    }
    el.classList.add('layer')
    this.stage.appendChild(el)
    const n = { el, media, key }
    this.nodes.set(a.clip.id, n)
    return n
  }

  update(s: PreviewState) {
    const sizeChanged = !this.last || this.last.project.settings.width !== s.project.settings.width || this.last.project.settings.height !== s.project.settings.height
    this.last = s
    if (sizeChanged) this.layout()
    this.stage.style.background = css(s.project.settings.background ?? [0, 0, 0, 1])
    const layers = visualLayers(s.project, s.time)
    const voices = audioVoices(s.project, s.time)
    // Warm up clips that start within the next second so cuts don't stall.
    const upcoming = s.playing ? this.upcoming(s.project, s.time) : []
    const wanted = new Set<string>()
    const stageH = this.stage.clientHeight

    layers.forEach((a, z) => {
      wanted.add(a.clip.id)
      const n = this.nodeFor(a)
      n.el.style.display = ''
      n.el.style.zIndex = String(z + 1)
      this.applyVisual(n.el, a.clip, s.time - a.clip.start, stageH)
    })
    for (const v of voices) {
      wanted.add(v.clip.id)
      const n = this.nodeFor(v)
      if (!layers.some(l => l.clip.id === v.clip.id)) n.el.style.display = 'none'
    }
    for (const a of upcoming) {
      if (wanted.has(a.clip.id)) continue
      wanted.add(a.clip.id)
      const n = this.nodeFor(a)
      n.el.style.display = 'none'
      if (n.media && n.media.paused && Math.abs(n.media.currentTime - seconds(a.source)) > 0.05) n.media.currentTime = seconds(a.source)
    }
    for (const [id, n] of this.nodes) if (!wanted.has(id)) {
      this.dispose(n)
      this.nodes.delete(id)
    }

    // Sync media elements to the playhead.
    const all = new Map<string, ActiveClip & { gain?: number }>()
    for (const l of layers) all.set(l.clip.id, l)
    for (const v of voices) all.set(v.clip.id, v)
    for (const [id, a] of all) {
      const n = this.nodes.get(id)
      const m = n?.media
      if (!m) continue
      const voice = voices.find(v => v.clip.id === id)
      m.muted = !voice
      m.volume = Math.max(0, Math.min(1, (voice?.gain ?? 0) * s.volume))
      const target = seconds(a.source)
      const rate = Math.abs(s.rate) * (a.clip.speed?.rate ?? 1)
      const forward = s.rate > 0 && !a.clip.speed?.reverse
      if (s.playing && forward) {
        if (Math.abs(m.playbackRate - rate) > 1e-3) m.playbackRate = Math.min(16, Math.max(0.0625, rate))
        if (Math.abs(m.currentTime - target) > 0.3) m.currentTime = target
        if (m.paused) m.play().catch(() => {})
      } else {
        if (!m.paused) m.pause()
        if (!m.seeking && Math.abs(m.currentTime - target) > 0.5 / 60) m.currentTime = target
      }
    }
  }

  private upcoming(p: Project, t: number): ActiveClip[] {
    const ahead = t + 10_000_000
    const out: ActiveClip[] = []
    const seq = p.sequences.find(s => s.id === p.main_sequence) ?? p.sequences[0]
    for (const track of seq.tracks)
      for (const clip of track.clips) {
        if (clip.start > t && clip.start <= ahead && clip.source.type === 'media') {
          const media = p.media.find(m => m.id === (clip.source as { media_id: string }).media_id) ?? null
          if (media && media.kind !== 'image') out.push({ clip, track, media, source: clip.source_in ?? 0 })
        }
      }
    return out
  }

  private applyVisual(el: HTMLElement, clip: Clip, local: number, stageH: number) {
    const tr = clip.transform
    const kf = (p: Parameters<typeof paramValue>[0], d: number) => sample(p, local, d)
    const x = kf(tr?.position_x, 0)
    const y = kf(tr?.position_y, 0)
    const sx = kf(tr?.scale_x, 1) * (tr?.flip_h ? -1 : 1)
    const sy = kf(tr?.scale_y, 1) * (tr?.flip_v ? -1 : 1)
    const rot = kf(tr?.rotation, 0)
    el.style.transform = `translate(${x * 100}%, ${y * 100}%) rotate(${rot}deg) scale(${sx}, ${sy})`
    el.style.opacity = String(kf(clip.opacity, 1))
    el.style.mixBlendMode = (clip.blend_mode ?? 'normal').replace('_', '-')
    const fit = tr?.fit ?? 'fit'
    el.style.objectFit = fit === 'fill' ? 'cover' : fit === 'stretch' ? 'fill' : fit === 'none' ? 'none' : 'contain'
    if (clip.source.type === 'title') {
      const t = clip.source
      if (el.textContent !== t.text) el.textContent = t.text
      el.style.fontFamily = t.font_family
      el.style.fontSize = `${t.font_size * stageH}px`
      el.style.color = css(t.color)
      el.style.fontWeight = t.bold ? '700' : '400'
      el.style.fontStyle = t.italic ? 'italic' : 'normal'
      el.style.left = `${t.position[0] * 100}%`
      el.style.top = `${t.position[1] * 100}%`
      el.style.background = t.background ? css(t.background) : ''
      el.style.webkitTextStroke = t.stroke ? `${t.stroke.width}px ${css(t.stroke.color)}` : ''
    }
  }

  async snapshot(): Promise<Blob | null> {
    const canvas = document.createElement('canvas')
    const p = this.last?.project
    if (!p) return null
    canvas.width = p.settings.width
    canvas.height = p.settings.height
    const ctx = canvas.getContext('2d')!
    ctx.fillStyle = '#000'
    ctx.fillRect(0, 0, canvas.width, canvas.height)
    const els = [...this.nodes.values()].filter(n => n.el.style.display !== 'none').sort((a, b) => Number(a.el.style.zIndex) - Number(b.el.style.zIndex))
    for (const n of els) {
      const src = n.el as HTMLVideoElement | HTMLImageElement
      if (!(src instanceof HTMLVideoElement || src instanceof HTMLImageElement)) continue
      const w = src instanceof HTMLVideoElement ? src.videoWidth : src.naturalWidth
      const h = src instanceof HTMLVideoElement ? src.videoHeight : src.naturalHeight
      if (!w || !h) continue
      const k = Math.min(canvas.width / w, canvas.height / h)
      try {
        ctx.drawImage(src, (canvas.width - w * k) / 2, (canvas.height - h * k) / 2, w * k, h * k)
      } catch {
        return null
      }
    }
    return new Promise(r => canvas.toBlob(r, 'image/png'))
  }
}

/** Samples a float param with keyframes (linear), like `Param::sample`. */
function sample(p: { value: { type: string; value: unknown }; keyframes?: { time: number; value: { type: string; value: unknown } }[] } | undefined, t: number, d: number): number {
  if (!p) return d
  const kfs = p.keyframes ?? []
  const num = (v: { type: string; value: unknown }) => (v.type === 'float' ? (v.value as number) : d)
  if (!kfs.length) return num(p.value)
  if (t <= kfs[0].time) return num(kfs[0].value)
  for (let i = 1; i < kfs.length; i++) {
    if (t < kfs[i].time) {
      const a = kfs[i - 1]
      const b = kfs[i]
      const k = (t - a.time) / Math.max(1, b.time - a.time)
      return num(a.value) + (num(b.value) - num(a.value)) * k
    }
  }
  return num(kfs[kfs.length - 1].value)
}
