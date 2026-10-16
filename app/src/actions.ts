// Menu bar definitions and global keyboard shortcuts (Filmora's defaults on macOS).

import {
  addTrack,
  copySelection,
  cutSelection,
  deleteSelection,
  duplicateSelection,
  getState,
  importMedia,
  mark,
  requestQuit,
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
  addMarkerAtPlayhead,
  setPlayerFullscreen,
} from './store'
import { backend } from './backend'
import { chromaKeySelection, detachAudio, freezeFrame, stabilizeSelection, toggleMute } from './edit'
import { sep, type MenuItem } from './ui/ContextMenu'

const isMac = navigator.platform.toLowerCase().includes('mac') || navigator.userAgent.includes('Mac')
export const mod = isMac ? '⌘' : 'Ctrl+'
const shift = isMac ? '⇧' : 'Shift+'

const SHORTCUTS = `Space play · J/K/L shuttle · ←/→ frame · I/O mark in/out · M marker · ${mod}B split · ${mod}Z undo · ⌫ delete · N snapping`
const REPO = 'https://github.com/Jibaru/jibaru-editor'

const importFolder = async () => {
  const dir = await backend.pickFiles({ title: 'Import Media Folder', directory: true, multiple: false })
  if (dir?.[0]) await importMedia(dir)
}

const openLink = (url: string) => backend.openUrl(url).catch(e => toast(String(e), 'error'))

/** Window menu bar (Windows, browser). Items are built when a menu opens, from the state at that moment. */
export function menuBar(): { name: string; items: () => MenuItem[] }[] {
  const st = () => {
    const s = getState()
    return { s, hasSel: s.selection.length > 0 }
  }
  // Each menu reads the state when it opens (the title bar does not re-render on selection changes).
  return [
    {
      name: 'File',
      items: () => {
        return [
        { label: 'New Project', shortcut: `${mod}N`, action: newProject },
        { label: 'Open Project...', shortcut: `${mod}O`, action: () => openProject() },
        sep,
        { label: 'Save Project', shortcut: `${mod}S`, action: () => saveProject() },
        { label: 'Save Project As...', shortcut: `${shift}${mod}S`, action: () => saveProject(true) },
        sep,
        { label: 'Import Media Files...', shortcut: `${mod}I`, action: () => importMedia() },
        { label: 'Import Media Folder...', action: importFolder },
        sep,
        { label: 'Export...', shortcut: `${mod}E`, action: () => setState({ exportOpen: true }) },
        ...(backend.engine ? [sep, { label: 'Quit', shortcut: `${mod}Q`, action: requestQuit }] : []),
      ]
      },
    },
    {
      name: 'Edit',
      items: () => {
        const { s, hasSel } = st()
        return [
        { label: 'Undo', shortcut: `${mod}Z`, action: undo, disabled: !s.snap?.can_undo },
        { label: 'Redo', shortcut: isMac ? `${shift}${mod}Z` : `${mod}Y`, action: redo, disabled: !s.snap?.can_redo },
        sep,
        { label: 'Cut', shortcut: `${mod}X`, action: cutSelection, disabled: !hasSel },
        { label: 'Copy', shortcut: `${mod}C`, action: copySelection, disabled: !hasSel },
        { label: 'Paste', shortcut: `${mod}V`, action: paste, disabled: !s.clipboard.length },
        { label: 'Duplicate', shortcut: `${mod}D`, action: duplicateSelection, disabled: !hasSel },
        { label: 'Delete', shortcut: '⌫', action: () => deleteSelection(false), disabled: !hasSel && !s.selectedMedia.length },
        { label: 'Ripple Delete', shortcut: `${shift}⌫`, action: () => deleteSelection(true), disabled: !hasSel },
        sep,
        { label: 'Select All', shortcut: `${mod}A`, action: selectAll },
        { label: 'Split', shortcut: `${mod}B`, action: splitAtPlayhead },
        { label: 'Add Marker', shortcut: 'M', action: addMarkerAtPlayhead },
      ]
      },
    },
    {
      name: 'Tools',
      items: () => {
        const { s } = st()
        return [
        { label: 'Add Video Track', action: () => addTrack('video') },
        { label: 'Add Audio Track', action: () => addTrack('audio') },
        sep,
        { label: 'Snapping', shortcut: 'N', checked: s.snapping, action: () => setState({ snapping: !s.snapping }) },
        sep,
        { label: 'Freeze Frame', action: freezeFrame },
        { label: 'Detach Audio', action: detachAudio },
        { label: 'Mute', action: toggleMute },
        sep,
        { label: 'Stabilization', action: stabilizeSelection },
        { label: 'Chroma Key', action: chromaKeySelection },
      ]
      },
    },
    {
      name: 'View',
      items: () => {
        const { s } = st()
        return [
        { label: 'Zoom In', shortcut: `${mod}=`, action: () => zoomBy(1.5) },
        { label: 'Zoom Out', shortcut: `${mod}-`, action: () => zoomBy(1 / 1.5) },
        { label: 'Zoom to Fit Timeline', shortcut: `${shift}Z`, action: zoomToFit },
        sep,
        { label: 'Play/Pause', shortcut: 'Space', action: togglePlay },
        { label: 'Stop', shortcut: `${mod}.`, action: stop },
        { label: 'Previous Frame', shortcut: '←', action: () => stepFrames(-1) },
        { label: 'Next Frame', shortcut: '→', action: () => stepFrames(1) },
        sep,
        { label: 'Mark In', shortcut: 'I', action: () => mark('in') },
        { label: 'Mark Out', shortcut: 'O', action: () => mark('out') },
        { label: 'Clear In/Out', action: () => setState({ markIn: null, markOut: null }), disabled: s.markIn == null && s.markOut == null },
        sep,
        { label: 'Full Screen Preview', checked: s.playerFullscreen, action: () => setPlayerFullscreen(!s.playerFullscreen) },
        { label: 'Show Playback Statistics', checked: s.showStats, action: () => setState({ showStats: !s.showStats }) },
      ]
      },
    },
    {
      name: 'Help',
      items: () => {
        return [
        { label: 'Keyboard Shortcuts', action: () => toast(SHORTCUTS) },
        sep,
        { label: 'Jibaru Editor on GitHub', action: () => openLink(REPO) },
        { label: 'Report an Issue', action: () => openLink(`${REPO}/issues`) },
      ]
      },
    },
  ]
}

/** Actions behind the native macOS menu items (src-tauri/src/menu.rs). */
const MENU_ACTIONS: Record<string, () => unknown> = {
  'app.quit': requestQuit,
  'file.new': newProject,
  'file.open': () => openProject(),
  'file.save': () => saveProject(),
  'file.save_as': () => saveProject(true),
  'file.import': () => importMedia(),
  'file.import_folder': importFolder,
  'file.export': () => setState({ exportOpen: true }),
  'edit.undo': () => (isTyping() ? document.execCommand('undo') : undo()),
  'edit.redo': () => (isTyping() ? document.execCommand('redo') : redo()),
  'edit.duplicate': duplicateSelection,
  'edit.delete': () => deleteSelection(false),
  'edit.ripple_delete': () => deleteSelection(true),
  'edit.split': splitAtPlayhead,
  'edit.add_marker': addMarkerAtPlayhead,
  'tools.add_video_track': () => addTrack('video'),
  'tools.add_audio_track': () => addTrack('audio'),
  'tools.snapping': () => setState(s => ({ snapping: !s.snapping })),
  'tools.freeze_frame': freezeFrame,
  'tools.detach_audio': detachAudio,
  'tools.mute_clip': toggleMute,
  'tools.stabilize': stabilizeSelection,
  'tools.chroma_key': chromaKeySelection,
  'view.zoom_in': () => zoomBy(1.5),
  'view.zoom_out': () => zoomBy(1 / 1.5),
  'view.zoom_fit': zoomToFit,
  'view.play': togglePlay,
  'view.stop': stop,
  'view.prev_frame': () => stepFrames(-1),
  'view.next_frame': () => stepFrames(1),
  'view.mark_in': () => mark('in'),
  'view.mark_out': () => mark('out'),
  'view.clear_marks': () => setState({ markIn: null, markOut: null }),
  'view.player_fullscreen': () => setPlayerFullscreen(!getState().playerFullscreen),
  'view.stats': () => setState(s => ({ showStats: !s.showStats })),
  'help.shortcuts': () => toast(SHORTCUTS),
  'help.website': () => openLink(REPO),
  'help.feedback': () => openLink(`${REPO}/issues`),
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
    if (k === 'i') return run(() => mark('in'))
    if (k === 'o') return run(() => mark('out'))
    if (k === 'm') return run(addMarkerAtPlayhead)
    if (k === 'z' && e.shiftKey) return run(zoomToFit)
    if (k === 'escape') {
      const s = getState()
      if (s.eyedropper) return run(() => setState({ eyedropper: null }))
      if (s.playerFullscreen) return run(() => setPlayerFullscreen(false))
      return run(() => setState({ selection: [], selectedMedia: [] }))
    }
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
