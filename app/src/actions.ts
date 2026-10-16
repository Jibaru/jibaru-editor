// Menu bar definitions and global keyboard shortcuts (Filmora's defaults on macOS).

import {
  addTrack,
  copySelection,
  cutSelection,
  deleteSelection,
  duplicateSelection,
  getState,
  importMedia,
  newProject,
  openProject,
  paste,
  redo,
  saveProject,
  selectAll,
  setState,
  shuttle,
  splitAtPlayhead,
  stepFrames,
  stop,
  togglePlay,
  undo,
  zoomBy,
  zoomToFit,
  seek,
  duration,
  toast,
} from './store'
import { sep, type MenuItem } from './ui/ContextMenu'

const isMac = navigator.platform.toLowerCase().includes('mac') || navigator.userAgent.includes('Mac')
export const mod = isMac ? '⌘' : 'Ctrl+'
const shift = isMac ? '⇧' : 'Shift+'

export function menuBar(): { name: string; items: () => MenuItem[] }[] {
  const s = getState()
  const hasSel = s.selection.length > 0
  return [
    {
      name: 'File',
      items: () => [
        { label: 'New Project', shortcut: `${mod}N`, action: newProject },
        { label: 'Open Project...', shortcut: `${mod}O`, action: () => openProject() },
        sep,
        { label: 'Save Project', shortcut: `${mod}S`, action: () => saveProject() },
        { label: 'Save Project As...', shortcut: `${shift}${mod}S`, action: () => saveProject(true) },
        sep,
        { label: 'Import Media Files...', shortcut: `${mod}I`, action: () => importMedia() },
        sep,
        { label: 'Export...', shortcut: `${mod}E`, action: () => setState({ exportOpen: true }) },
      ],
    },
    {
      name: 'Edit',
      items: () => [
        { label: 'Undo', shortcut: `${mod}Z`, action: undo, disabled: !s.snap?.can_undo },
        { label: 'Redo', shortcut: `${shift}${mod}Z`, action: redo, disabled: !s.snap?.can_redo },
        sep,
        { label: 'Cut', shortcut: `${mod}X`, action: cutSelection, disabled: !hasSel },
        { label: 'Copy', shortcut: `${mod}C`, action: copySelection, disabled: !hasSel },
        { label: 'Paste', shortcut: `${mod}V`, action: paste, disabled: !s.clipboard.length },
        { label: 'Duplicate', shortcut: `${mod}D`, action: duplicateSelection, disabled: !hasSel },
        { label: 'Delete', shortcut: '⌫', action: () => deleteSelection(false), disabled: !hasSel },
        { label: 'Ripple Delete', shortcut: `${shift}⌫`, action: () => deleteSelection(true), disabled: !hasSel },
        sep,
        { label: 'Select All', shortcut: `${mod}A`, action: selectAll },
        { label: 'Split', shortcut: `${mod}B`, action: splitAtPlayhead },
      ],
    },
    {
      name: 'Tools',
      items: () => [
        { label: 'Add Video Track', action: () => addTrack('video') },
        { label: 'Add Audio Track', action: () => addTrack('audio') },
        sep,
        { label: 'Snapping', shortcut: 'N', checked: s.snapping, action: () => setState({ snapping: !s.snapping }) },
      ],
    },
    {
      name: 'View',
      items: () => [
        { label: 'Zoom In', shortcut: `${mod}=`, action: () => zoomBy(1.5) },
        { label: 'Zoom Out', shortcut: `${mod}-`, action: () => zoomBy(1 / 1.5) },
        { label: 'Zoom to Fit Timeline', shortcut: `${shift}Z`, action: zoomToFit },
        sep,
        { label: 'Play/Pause', shortcut: 'Space', action: togglePlay },
        { label: 'Stop', shortcut: `${mod}.`, action: stop },
        { label: 'Previous Frame', shortcut: '←', action: () => stepFrames(-1) },
        { label: 'Next Frame', shortcut: '→', action: () => stepFrames(1) },
      ],
    },
    {
      name: 'Help',
      items: () => [
        { label: 'About Jibaru Editor', action: () => window.open('https://github.com/Jibaru/jibaru-editor') },
        { label: 'Keyboard Shortcuts', action: () => toast(`Space play · J/K/L shuttle · ${mod}B split · ${mod}Z undo · ⌫ delete`) },
      ],
    },
  ]
}

/** Actions behind the native macOS menu items (src-tauri/src/menu.rs). */
const MENU_ACTIONS: Record<string, () => unknown> = {
  'file.new': newProject,
  'file.open': () => openProject(),
  'file.save': () => saveProject(),
  'file.save_as': () => saveProject(true),
  'file.import': () => importMedia(),
  'file.export': () => setState({ exportOpen: true }),
  'edit.undo': () => (isTyping() ? document.execCommand('undo') : undo()),
  'edit.redo': () => (isTyping() ? document.execCommand('redo') : redo()),
  'edit.duplicate': duplicateSelection,
  'edit.delete': () => deleteSelection(false),
  'edit.ripple_delete': () => deleteSelection(true),
  'edit.split': splitAtPlayhead,
  'tools.add_video_track': () => addTrack('video'),
  'tools.add_audio_track': () => addTrack('audio'),
  'tools.snapping': () => setState(s => ({ snapping: !s.snapping })),
  'view.zoom_in': () => zoomBy(1.5),
  'view.zoom_out': () => zoomBy(1 / 1.5),
  'view.zoom_fit': zoomToFit,
  'view.play': togglePlay,
  'view.stop': stop,
  'help.shortcuts': () => toast(`Space play · J/K/L shuttle · ${mod}B split · ${mod}Z undo · ⌫ delete`),
}

const isTyping = () => !!(document.activeElement as HTMLElement | null)?.closest?.('input, textarea, [contenteditable="true"]')

export function runMenuAction(id: string) {
  MENU_ACTIONS[id]?.()
}

/** True when the OS menu bar shows our menus (macOS app), so the window hides its own. */
export const nativeMenuBar = isMac && '__TAURI_INTERNALS__' in window

/** Global shortcuts. Ignored while typing in a text field. */
export function installShortcuts() {
  const onKey = (e: KeyboardEvent) => {
    const el = e.target as HTMLElement
    const typing = !!el.closest?.('input, textarea, select, [contenteditable="true"]')
    const m = isMac ? e.metaKey : e.ctrlKey
    const k = e.key.toLowerCase()
    const run = (fn: () => unknown) => {
      e.preventDefault()
      fn()
    }
    if (getState().menu) return
    // With the native menu bar, its accelerators own these keys (they arrive as
    // `menu` events or DOM clipboard events), so handling them here too would
    // run the action twice.
    if (nativeMenuBar && m && 'nosiezdbcxv=-.'.includes(k) && k !== '') return
    if (nativeMenuBar && !m && e.shiftKey && k === 'z') return
    if (m) {
      if (k === 's') return run(() => saveProject(e.shiftKey))
      if (k === 'o') return run(() => openProject())
      if (k === 'n') return run(newProject)
      if (k === 'i') return run(() => importMedia())
      if (k === 'e') return run(() => setState({ exportOpen: true }))
      if (typing) return
      if (k === 'z') return run(e.shiftKey ? redo : undo)
      if (k === 'y' && !isMac) return run(redo)
      if (k === 'b') return run(splitAtPlayhead)
      if (k === 'c') return run(copySelection)
      if (k === 'x') return run(cutSelection)
      if (k === 'v') return run(paste)
      if (k === 'd') return run(duplicateSelection)
      if (k === 'a') return run(selectAll)
      if (k === '=' || k === '+') return run(() => zoomBy(1.5))
      if (k === '-') return run(() => zoomBy(1 / 1.5))
      if (k === '.') return run(stop)
      return
    }
    if (typing) return
    if (k === ' ') return run(togglePlay)
    if (k === 'delete' || k === 'backspace') return run(() => deleteSelection(e.shiftKey))
    if (k === 'arrowleft') return run(() => stepFrames(e.shiftKey ? -5 : -1))
    if (k === 'arrowright') return run(() => stepFrames(e.shiftKey ? 5 : 1))
    if (k === 'home') return run(() => seek(0))
    if (k === 'end') return run(() => seek(duration()))
    if (k === 'j' || k === 'k' || k === 'l') return run(() => shuttle(k))
    if (k === 'n') return run(() => setState(s => ({ snapping: !s.snapping })))
    if (k === 'z' && e.shiftKey) return run(zoomToFit)
    if (k === 'escape') return run(() => setState({ selection: [], selectedMedia: [] }))
  }
  window.addEventListener('keydown', onKey)
  // No browser context menu anywhere except text fields; we show our own.
  const block = (e: MouseEvent) => {
    if (!(e.target as HTMLElement).closest?.('input, textarea')) e.preventDefault()
  }
  window.addEventListener('contextmenu', block)
  // System Cut/Copy/Paste menu items reach the page as clipboard events.
  const clip = (fn: () => unknown) => (e: ClipboardEvent) => {
    if (isTyping()) return
    e.preventDefault()
    fn()
  }
  const onCopy = clip(copySelection)
  const onCut = clip(cutSelection)
  const onPaste = clip(paste)
  document.addEventListener('copy', onCopy)
  document.addEventListener('cut', onCut)
  document.addEventListener('paste', onPaste)
  let unlisten: (() => void) | undefined
  if (nativeMenuBar)
    import('@tauri-apps/api/event').then(({ listen }) => listen<string>('menu', e => runMenuAction(e.payload)).then(u => (unlisten = u)))
  return () => {
    unlisten?.()
    document.removeEventListener('copy', onCopy)
    document.removeEventListener('cut', onCut)
    document.removeEventListener('paste', onPaste)
    window.removeEventListener('keydown', onKey)
    window.removeEventListener('contextmenu', block)
  }
}
