// Pointer-based drag from the library to the timeline. HTML5 drag-and-drop
// is avoided because Tauri's native file-drop handling disables it on
// Windows; this works the same everywhere.

import { useSyncExternalStore, type PointerEvent as ReactPointerEvent } from 'react'
import type { LibraryItem } from './library'

export interface LibDrag {
  item: LibraryItem
  x: number
  y: number
}

let drag: LibDrag | null = null
const listeners = new Set<() => void>()
type DropTarget = (d: LibDrag) => boolean
const targets = new Set<DropTarget>()

function set(d: LibDrag | null) {
  drag = d
  listeners.forEach(l => l())
}

export function useLibDrag(): LibDrag | null {
  return useSyncExternalStore(
    l => {
      listeners.add(l)
      return () => listeners.delete(l)
    },
    () => drag,
  )
}

/** Registers a drop handler; it returns true when it accepted the drop. */
export function registerDropTarget(t: DropTarget) {
  targets.add(t)
  return () => {
    targets.delete(t)
  }
}

export function startLibraryDrag(e: ReactPointerEvent, item: LibraryItem) {
  const x0 = e.clientX
  const y0 = e.clientY
  let active = false
  const move = (ev: PointerEvent) => {
    if (!active && Math.hypot(ev.clientX - x0, ev.clientY - y0) < 5) return
    active = true
    document.body.classList.add('dragging')
    set({ item, x: ev.clientX, y: ev.clientY })
  }
  const up = () => {
    window.removeEventListener('pointermove', move)
    window.removeEventListener('pointerup', up)
    document.body.classList.remove('dragging')
    if (active && drag) {
      const d = drag
      for (const t of targets) if (t(d)) break
    }
    set(null)
  }
  window.addEventListener('pointermove', move)
  window.addEventListener('pointerup', up)
}
