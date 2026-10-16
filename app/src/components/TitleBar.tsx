import { menuBar, nativeMenuBar } from '../actions'
import { backend } from '../backend'
import { openMenu, saveProject, setLayout, setState, toast, useEditor } from '../store'
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
const REPO = 'https://github.com/Jibaru/jibaru-editor'
const link = (url: string) => backend.openUrl(url).catch(e => toast(String(e), 'error'))
const menuAt = (e: React.MouseEvent, items: import('../ui/ContextMenu').MenuItem[]) => {
  const r = e.currentTarget.getBoundingClientRect()
  openMenu(r.right - 200, r.bottom + 4, items)
}

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
        <button className="ticon" title="What's new" onClick={() => link(`${REPO}/releases`)}><IcGift size={18} /></button>
        <button className="ticon" title="Share to the cloud is not available in Jibaru Editor" disabled><IcSend size={18} /></button>
        <button className="ticon" title="Feedback" onClick={() => link(`${REPO}/issues/new`)}><IcFeedback size={18} /></button>
        <button className="ticon" title="Layout" onClick={e => menuAt(e, [
          { label: 'Default Layout', action: () => setLayout('default') },
          { label: 'Larger Player', action: () => setLayout('player') },
          { label: 'Larger Timeline', action: () => setLayout('timeline') },
        ])}><IcMonitor size={18} /></button>
        <button className="ticon" title={`Save (${isMac ? '⌘' : 'Ctrl+'}S)`} onClick={() => saveProject()}><IcSave size={18} /></button>
        <button className="ticon" title="Cloud backup is not available in Jibaru Editor" disabled><IcCloud size={18} /></button>
        <button className="ticon" title="Help and documentation" onClick={() => link(`${REPO}#readme`)}><IcHeadset size={18} /></button>
        <button className="ticon" title="More" onClick={e => menuAt(e, [
          { label: 'About Jibaru Editor', action: () => toast('Jibaru Editor 0.1.0 · Apache-2.0 · Rust engine, Tauri + React UI') },
          { label: 'Keyboard Shortcuts', action: () => import('../actions').then(m => m.runMenuAction('help.shortcuts')) },
          { label: 'Jibaru Editor on GitHub', action: () => link(REPO) },
        ])}><IcApps size={18} /></button>
        <button className="export-btn" onClick={() => setState({ exportOpen: true })}>
          Export <IcChevronDown size={13} strokeWidth={2} />
        </button>
      </div>
    </header>
  )
}
