// Built-in library content. Effect and transition `kind`s are the stable ids
// the renderer's registry resolves (unknown ids render as pass-through).

import type { LibraryTab } from './store'

export interface LibraryItem {
  type: 'media' | 'title' | 'transition' | 'effect'
  id: string
  label: string
  sample?: string
}

export const catalog: Partial<Record<LibraryTab, LibraryItem[]>> = {
  titles: [
    { type: 'title', id: 'default', label: 'Default Title' },
    { type: 'title', id: 'basic', label: 'Basic Title' },
    { type: 'title', id: 'lower_third', label: 'Lower Third' },
    { type: 'title', id: 'subtitle', label: 'Subtitle' },
  ],
  transitions: [
    { type: 'transition', id: 'jb.dissolve', label: 'Dissolve' },
    { type: 'transition', id: 'jb.fade_black', label: 'Fade to Black' },
    { type: 'transition', id: 'jb.wipe.left', label: 'Wipe Left' },
    { type: 'transition', id: 'jb.slide.left', label: 'Slide Left' },
    { type: 'transition', id: 'jb.push.left', label: 'Push Left' },
    { type: 'transition', id: 'jb.zoom.in', label: 'Zoom In' },
  ],
  effects: [
    { type: 'effect', id: 'jb.blur.gaussian', label: 'Gaussian Blur' },
    { type: 'effect', id: 'jb.vignette', label: 'Vignette' },
    { type: 'effect', id: 'jb.mosaic', label: 'Mosaic' },
    { type: 'effect', id: 'jb.shake', label: 'Camera Shake' },
    { type: 'effect', id: 'jb.glow', label: 'Glow' },
  ],
  filters: [
    { type: 'effect', id: 'jb.filter.bw', label: 'Black & White' },
    { type: 'effect', id: 'jb.filter.sepia', label: 'Sepia' },
    { type: 'effect', id: 'jb.filter.warm', label: 'Warm' },
    { type: 'effect', id: 'jb.filter.cool', label: 'Cool' },
    { type: 'effect', id: 'jb.filter.vivid', label: 'Vivid' },
  ],
}

export function catalogLabel(kind: string): string {
  for (const items of Object.values(catalog)) for (const i of items ?? []) if (i.id === kind) return i.label
  return kind.replace(/^jb\./, '').replace(/[._]/g, ' ')
}
