import { backend } from '../backend'
import { DomPreview } from './dom'
import { FramePreview } from './frames'
import type { PreviewRenderer } from './types'

export type { PreviewRenderer, PreviewState } from './types'

export type PreviewKind = 'dom' | 'frames'

/** Picks the preview renderer. Set `localStorage['jb.preview'] = 'frames'` to use backend frames. */
export function createPreview(kind?: PreviewKind): PreviewRenderer {
  let k = kind
  try {
    k ??= (localStorage.getItem('jb.preview') as PreviewKind | null) ?? undefined
  } catch {
    /* storage unavailable */
  }
  if (k === 'frames' && backend.kind === 'tauri') return new FramePreview()
  return new DomPreview()
}
