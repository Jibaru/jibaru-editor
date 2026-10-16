import { backend } from '../backend'
import { DomPreview } from './dom'
import { FramePreview } from './frames'
import { NativePreview } from './native'
import type { PreviewRenderer } from './types'

export type { PreviewRenderer, PreviewState } from './types'

export type PreviewKind = 'native' | 'dom' | 'frames'

/**
 * Picks the preview renderer: the native engine inside the app (real-time
 * jb-render compositing with audio), DOM layers in a plain browser. Override
 * with `localStorage['jb.preview']`.
 */
export function createPreview(kind?: PreviewKind): PreviewRenderer {
  let k = kind
  try {
    k ??= (localStorage.getItem('jb.preview') as PreviewKind | null) ?? undefined
  } catch {
    /* storage unavailable */
  }
  if (k === 'dom' || !backend.engine) return new DomPreview()
  if (k === 'frames') return new FramePreview()
  return new NativePreview(backend.engine)
}
