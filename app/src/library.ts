// Library content. Transitions, effects, filters and animations come from the
// renderer's registry (`jb-render`), so every item shown is one it can draw.

import type { FxEntry } from './backend'
import { getState, type LibraryTab } from './store'

export interface LibraryItem {
  type: 'media' | 'title' | 'transition' | 'effect'
  id: string
  label: string
  category?: string
  sample?: string
}

export const titleItems: LibraryItem[] = [
  { type: 'title', id: 'default', label: 'Default Title', category: 'Default' },
  { type: 'title', id: 'basic', label: 'Basic Title', category: 'Default' },
  { type: 'title', id: 'lower_third', label: 'Lower Third', category: 'Lower Thirds' },
  { type: 'title', id: 'subtitle', label: 'Subtitle', category: 'Subtitles' },
]

const TAB_KIND: Partial<Record<LibraryTab, FxEntry['kind']>> = { transitions: 'transition', effects: 'effect', filters: 'filter' }

export function tabItems(tab: LibraryTab, fx: FxEntry[]): LibraryItem[] {
  if (tab === 'titles') return titleItems
  const kind = TAB_KIND[tab]
  if (!kind) return []
  return fx
    .filter(f => f.kind === kind && !f.id.startsWith('jb.color.'))
    .map(f => ({ type: kind === 'transition' ? 'transition' : 'effect', id: f.id, label: f.name, category: f.category, sample: kind }))
}

export function tabCategories(tab: LibraryTab, fx: FxEntry[]): string[] {
  return [...new Set(tabItems(tab, fx).map(i => i.category ?? 'Default'))]
}

/** Entry (in) vs exit (out) animations, following the registry descriptions. */
export function animationsFor(fx: FxEntry[], dir: 'in' | 'out'): FxEntry[] {
  return fx.filter(f => f.kind === 'animation').filter(f => {
    const out = f.description.startsWith('Out:') || f.id.endsWith('_out')
    return dir === 'out' ? out : !out
  })
}

export function catalogLabel(kind: string): string {
  return getState().fx.find(f => f.id === kind)?.name ?? kind.replace(/^(jb|filmora)[.:]/, '').replace(/[._]/g, ' ')
}
