import { useEffect } from 'react'
import { installShortcuts } from './actions'
import { backend } from './backend'
import { ExportDialog } from './components/ExportDialog'
import { Library } from './components/Library'
import { Player } from './components/Player'
import { Properties } from './components/Properties'
import { TitleBar } from './components/TitleBar'
import { useLibDrag } from './drag'
import { importMedia, load, setState, useEditor } from './store'
import { Timeline } from './timeline/Timeline'
import { ContextMenu } from './ui/ContextMenu'
import { Splitter } from './ui/Splitter'

const clampW = (v: number, a: number, b: number) => Math.min(b, Math.max(a, v))

export default function App() {
  const ready = useEditor(s => !!s.snap)
  const menu = useEditor(s => s.menu)
  const toast = useEditor(s => s.toast)
  const exportOpen = useEditor(s => s.exportOpen)
  const drag = useLibDrag()
  const { libW, propsW, topH } = useEditor(s => s.layout)
  const fullscreen = useEditor(s => s.playerFullscreen)
  type L = { libW: number; propsW: number; topH: number }
  const setL = (f: (l: L) => Partial<L>) => setState(s => ({ layout: { ...s.layout, ...f(s.layout) } }))

  useEffect(() => {
    load()
    return installShortcuts()
  }, [])

  // Files dropped from the OS are imported into the project.
  useEffect(() => {
    if (backend.kind !== 'tauri') return
    let un: (() => void) | undefined
    let cancelled = false
    import('@tauri-apps/api/webview').then(({ getCurrentWebview }) =>
      getCurrentWebview()
        .onDragDropEvent(e => {
          if (e.payload.type === 'drop' && e.payload.paths.length) importMedia(e.payload.paths)
        })
        .then(f => (cancelled ? f() : (un = f))),
    )
    return () => {
      cancelled = true
      un?.()
    }
  }, [])

  if (!ready) return <div className="app loading" />

  return (
    <div className={`app ${fullscreen ? 'player-fs' : ''}`}>
      <TitleBar />
      <div className="workspace" style={{ gridTemplateColumns: `minmax(0,1fr) 6px ${propsW}px` }}>
        <div className="left-col" style={{ gridTemplateRows: `${topH}px 6px minmax(0,1fr)` }}>
          <div className="top-row" style={{ gridTemplateColumns: `${libW}px 6px minmax(0,1fr)` }}>
            <Library />
            <Splitter dir="x" onDrag={d => setL(l => ({ libW: clampW(l.libW + d, 340, 900) }))} />
            <Player />
          </div>
          <Splitter dir="y" onDrag={d => setL(l => ({ topH: clampW(l.topH + d, 260, innerHeight - 260) }))} />
          <Timeline />
        </div>
        <Splitter dir="x" onDrag={d => setL(l => ({ propsW: clampW(l.propsW - d, 300, 560) }))} />
        <Properties />
      </div>
      {menu && <ContextMenu x={menu.x} y={menu.y} items={menu.items} onClose={() => setState({ menu: null })} />}
      {exportOpen && <ExportDialog />}
      {toast && <div key={toast.id} className={`toast ${toast.kind}`}>{toast.text}</div>}
      {drag && (
        <div className="drag-ghost" style={{ left: drag.x + 10, top: drag.y + 10 }}>
          {drag.item.label}
        </div>
      )}
    </div>
  )
}
