import type { Project, Ticks } from '../core/types'

export interface PreviewState {
  project: Project
  /** Timeline time to show. */
  time: Ticks
  playing: boolean
  rate: number
  volume: number
  /** Backend revision, so renderers can drop cached frames after edits. */
  revision: number
}

/**
 * Draws the main sequence into the player screen. Implementations:
 * - `DomPreview`: stacks <video>/<img>/text layers with CSS transforms (no effects).
 * - `FramePreview`: shows frames rendered by the backend (`jb://frame`), which is
 *   where the native `jb-render` compositor plugs in.
 * The player only talks to this interface, so switching is one line.
 */
export interface PreviewRenderer {
  mount(screen: HTMLElement): void
  update(state: PreviewState): void
  unmount(): void
  /** Current frame as an image, for the snapshot button. */
  snapshot?(): Promise<Blob | null>
}
