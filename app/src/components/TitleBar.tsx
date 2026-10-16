import { menuBar, nativeMenuBar } from '../actions'
import { openMenu, saveProject, setState, useEditor } from '../store'
import {
  IcApps,
  IcChevronDown,
  IcCloud,
  IcFeedback,
  IcGift,
  IcHeadset,
  IcMonitor,
  IcSave,
  IcSend,
} from '../ui/icons'

const isMac = navigator.userAgent.includes('Mac')

/** Export progress and background proxy generation, like Filmora's task chip. */
function BackgroundStatus() {
  const job = useEditor(s => s.exportJob)
  const proxies = useEditor(s => s.proxies)
  const open = useEditor(s => s.exportOpen)
  const list = Object.values(proxies)
  const pending = list.filter(p => p.state === 'queued' || p.state === 'generating')
  const current = list.find(p => p.state === 'generating')
  const parts: string[] = []
  if (job?.state === 'running' && !open) {
    const p = job.progress
    parts.push(`Exporting ${p ? Math.floor((p.frame / Math.max(1, p.total)) * 100) : 0}%`)
  }
  if (pending.length) parts.push(`Generating proxies: ${pending.length} left${current ? ` (${Math.round(current.progress * 100)}%)` : ''}`)
  if (!parts.length) return null
  return (
    <span className="bg-status" title={pending.map(p => p.path.split(/[/\\]/).pop()).join('\n')} onClick={() => job && setState({ exportOpen: true })}>
      {parts.join(' · ')}
    </span>
  )
}

export function TitleBar() {
  const name = useEditor(s => s.snap?.project.name ?? '')
  const dirty = useEditor(s => s.snap?.dirty ?? false)
  const openName = useEditor(s => s.menu?.bar)

  return (
    <header className="titlebar" data-tauri-drag-region>
      {isMac && <div className="traffic-space" data-tauri-drag-region />}
      <nav className="menubar" hidden={nativeMenuBar}>
        {menuBar().map(m => (
          <button
            key={m.name}
            className={openName === m.name ? 'on' : ''}
            onMouseDown={e => {
              e.preventDefault()
              const r = e.currentTarget.getBoundingClientRect()
              if (openName === m.name) return setState({ menu: null })
              openMenu(r.left, r.bottom + 2, m.items(), m.name)
            }}
          >
            {m.name}
          </button>
        ))}
      </nav>
      <div className="title" data-tauri-drag-region>
        {name}
        {dirty && <span className="dirty-dot" title="Unsaved changes" />}
      </div>
      <div className="title-actions">
        <BackgroundStatus />
        <button className="ticon" title="What's new"><IcGift size={18} /></button>
        <button className="ticon" title="Share"><IcSend size={18} /></button>
        <button className="ticon" title="Feedback"><IcFeedback size={18} /></button>
        <button className="ticon" title="Layout"><IcMonitor size={18} /></button>
        <button className="ticon" title="Save (⌘S)" onClick={() => saveProject()}><IcSave size={18} /></button>
        <button className="ticon" title="Cloud backup"><IcCloud size={18} /></button>
        <button className="ticon" title="Support"><IcHeadset size={18} /></button>
        <button className="ticon" title="More"><IcApps size={18} /></button>
        <button className="export-btn" onClick={() => setState({ exportOpen: true })}>
          Export <IcChevronDown size={13} strokeWidth={2} />
        </button>
      </div>
    </header>
  )
}
