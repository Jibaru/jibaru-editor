// Preview backed by frames rendered in Rust (`jb://localhost/frame`). Today the
// backend extracts the top layer with FFmpeg; once `jb-render` is wired in,
// the same route returns fully composited frames and this class is unchanged.

import { backend } from '../backend'
import { fps, seconds } from '../core/model'
import type { PreviewRenderer, PreviewState } from './types'

export class FramePreview implements PreviewRenderer {
  private img = document.createElement('img')
  private wanted = ''
  private loading = false

  constructor() {
    this.img.className = 'stage frame-stage'
    this.img.draggable = false
  }

  mount(screen: HTMLElement) {
    screen.appendChild(this.img)
  }

  unmount() {
    this.img.remove()
  }

  update(s: PreviewState) {
    const rate = fps(s.project.settings.frame_rate)
    const t = Math.floor(seconds(s.time) * rate) / rate
    const h = Math.min(s.project.settings.height, Math.max(180, Math.round((this.img.parentElement?.clientHeight ?? 360) * devicePixelRatio)))
    const url = backend.media.frame(t, h, s.revision)
    this.img.style.aspectRatio = `${s.project.settings.width} / ${s.project.settings.height}`
    if (url === this.wanted) return
    this.wanted = url
    this.pump()
  }

  // Only one request in flight; always load the latest wanted frame next.
  private pump() {
    if (this.loading || !this.wanted || this.img.src === this.wanted) return
    this.loading = true
    const url = this.wanted
    const next = new Image()
    next.onload = next.onerror = () => {
      if (next.naturalWidth) this.img.src = url
      this.loading = false
      if (this.wanted !== url) this.pump()
    }
    next.src = url
  }
}
