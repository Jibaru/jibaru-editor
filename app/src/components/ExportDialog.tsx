import { useState } from 'react'
import { fps, mainSequence, sequenceDuration, timecode } from '../core/model'
import { setState, toast, useEditor } from '../store'
import { Pills, Select } from '../ui/controls'
import { IcClose, IcFolder } from '../ui/icons'

export function ExportDialog() {
  const project = useEditor(s => s.snap?.project)
  const path = useEditor(s => s.snap?.path)
  const [name, setName] = useState(project?.name ?? 'My Video')
  const [format, setFormat] = useState('mp4')
  const [quality, setQuality] = useState<'low' | 'medium' | 'high'>('high')
  if (!project) return null
  const s = project.settings
  const dir = path ? path.replace(/[/\\][^/\\]*$/, '') : '~/Movies'
  const close = () => setState({ exportOpen: false })
  return (
    <div className="modal-backdrop" onMouseDown={e => e.target === e.currentTarget && close()}>
      <div className="modal export-modal" role="dialog" aria-label="Export">
        <header>
          <h2>Export</h2>
          <button className="ticon" onClick={close}><IcClose size={18} /></button>
        </header>
        <div className="export-body">
          <nav className="export-targets">
            <button className="on">Local</button>
            <button disabled>Device</button>
            <button disabled>Social</button>
          </nav>
          <div className="export-form">
            <div className="prow"><span className="plabel">Name</span><input className="field" value={name} onChange={e => setName(e.target.value)} onKeyDown={e => e.stopPropagation()} /></div>
            <div className="prow"><span className="plabel">Save to</span><input className="field" value={dir} readOnly /><button className="ticon"><IcFolder size={18} /></button></div>
            <div className="prow select-row">
              <span className="plabel">Format</span>
              <Select value={format} onChange={setFormat} options={[{ value: 'mp4', label: 'MP4' }, { value: 'mov', label: 'MOV' }, { value: 'webm', label: 'WEBM' }, { value: 'gif', label: 'GIF' }, { value: 'mp3', label: 'MP3' }]} />
            </div>
            <div className="prow">
              <span className="plabel">Quality</span>
              <Pills value={quality} onChange={setQuality} options={[{ value: 'low', label: 'Lower' }, { value: 'medium', label: 'Recommended' }, { value: 'high', label: 'Higher' }]} />
            </div>
            <div className="prow"><span className="plabel">Resolution</span><span>{s.width} x {s.height}</span></div>
            <div className="prow"><span className="plabel">Frame Rate</span><span>{fps(s.frame_rate).toFixed(2).replace(/\.00$/, '')} fps</span></div>
            <div className="prow"><span className="plabel">Duration</span><span>{timecode(sequenceDuration(mainSequence(project)), s.frame_rate)}</span></div>
          </div>
        </div>
        <footer>
          <span className="muted">Rendering uses the jb-export engine.</span>
          <button
            className="export-btn big"
            onClick={() => {
              close()
              toast('Export is not available in this build yet', 'error')
            }}
          >
            Export
          </button>
        </footer>
      </div>
    </div>
  )
}
