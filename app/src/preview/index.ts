import { backend } from '../backend'
import { DomPreview } from './dom'
import { FramePreview } from './frames'
import type { PreviewRenderer, PreviewState } from './types'

export type { PreviewRenderer, PreviewState } from './types'

export type PreviewKind = 'dom' | 'frames' | 'hybrid'

/**
 * DOM layers for smooth playback with audio, and the native jb-render frame
 * (with effects, transitions and color) on top whenever playback is paused.
 */
class HybridPreview implements PreviewRenderer {
  private dom = new DomPreview()
  private frames = new FramePreview()
  private overlay = document.createElement('div')

  mount(screen: HTMLElement) {
    this.dom.mount(screen)
    this.overlay.className = 'frame-overlay'
    screen.appendChild(this.overlay)
    this.frames.mount(this.overlay)
  }

  update(s: PreviewState) {
    this.dom.update(s)
    this.overlay.style.visibility = s.playing ? 'hidden' : 'visible'
    if (!s.playing) this.frames.update(s)
  }

  unmount() {
    this.frames.unmount()
    this.overlay.remove()
    this.dom.unmount()
  }

  snapshot() {
    return this.dom.snapshot()
  }
}

/** Picks the preview renderer; override with `localStorage['jb.preview']`. */
export function createPreview(kind?: PreviewKind): PreviewRenderer {
  let k = kind
  try {
    k ??= (localStorage.getItem('jb.preview') as PreviewKind | null) ?? undefined
  } catch {
    /* storage unavailable */
  }
  if (backend.kind !== 'tauri') return new DomPreview()
  if (k === 'dom') return new DomPreview()
  if (k === 'frames') return new FramePreview()
  return new HybridPreview()
}
