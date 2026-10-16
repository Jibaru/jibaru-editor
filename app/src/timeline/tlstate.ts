// View state owned by the timeline and its toolbar (mouse tool, modes,
// dialogs). Project edits still go through the store's `exec`.

import { useSyncExternalStore } from 'react'
import type { Ticks } from '../core/types'

/** Filmora's "Switching Mouse Status" tools. */
export type MouseTool = 'select' | 'split' | 'forward' | 'backward' | 'zoom'

export type Dialog =
  | { kind: 'duration'; target: 'clip' | 'transition'; id: string }
  | { kind: 'marker'; id: string }
  | { kind: 'crop'; id: string }
  | { kind: 'mixer' }
  | { kind: 'voiceover' }
  | null

export interface TimelineUi {
  tool: MouseTool
  slideEdit: boolean
  /** Quick Preview Mode: hovering the timeline previews the frame under the pointer. */
  quickPreview: boolean
  meter: boolean
  /** Track height: compact rows (Filmora's "Track height" toggle). */
  compactTracks: boolean
  dialog: Dialog
  /** Time the pointer hovers in quick preview mode. */
  skim: Ticks | null
}

let ui: TimelineUi = { tool: 'select', slideEdit: false, quickPreview: false, meter: false, compactTracks: false, dialog: null, skim: null }
const listeners = new Set<() => void>()

export const getUi = () => ui
export function setUi(patch: Partial<TimelineUi> | ((s: TimelineUi) => Partial<TimelineUi>)) {
  ui = { ...ui, ...(typeof patch === 'function' ? patch(ui) : patch) }
  listeners.forEach(l => l())
}
export function useUi<T>(sel: (s: TimelineUi) => T): T {
  return useSyncExternalStore(
    l => {
      listeners.add(l)
      return () => void listeners.delete(l)
    },
    () => sel(ui),
  )
}

// Development automation (JB_AUTOMATION_DIR scripts) reaches the timeline view state here.
if (import.meta.env.DEV) (window as unknown as { __jbTl: unknown }).__jbTl = { getUi, setUi }
