// Export dialog (Filmora's "Local" tab): format, encoder, resolution, frame
// rate, quality and range, then a progress view while jb-export renders on a
// background thread. The job lives in the store, so the dialog can be closed
// and reopened while it runs.

import { useEffect, useMemo, useState } from 'react'
import { backend, type ExportRequest } from '../backend'
import { fps, mainSequence, seconds, sequenceDuration, timecode } from '../core/model'
import type { Rational, Ticks } from '../core/types'
import { setState, toast, useEditor } from '../store'
import { Pills, Select, Toggle } from '../ui/controls'
import { IcClose, IcFolder } from '../ui/icons'

type Range = 'all' | 'marks' | 'custom'

const RATES: { value: string; label: string; rate: Rational }[] = [
  { value: '24000/1001', label: '23.976 fps', rate: { num: 24000, den: 1001 } },
  { value: '24/1', label: '24 fps', rate: { num: 24, den: 1 } },
  { value: '25/1', label: '25 fps', rate: { num: 25, den: 1 } },
  { value: '30000/1001', label: '29.97 fps', rate: { num: 30000, den: 1001 } },
  { value: '30/1', label: '30 fps', rate: { num: 30, den: 1 } },
  { value: '50/1', label: '50 fps', rate: { num: 50, den: 1 } },
  { value: '60000/1001', label: '59.94 fps', rate: { num: 60000, den: 1001 } },
  { value: '60/1', label: '60 fps', rate: { num: 60, den: 1 } },
]

const HEIGHTS = [2160, 1440, 1080, 720, 540, 480]

const stored = (k: string, d: string) => {
  try {
    return localStorage.getItem(`jb.export.${k}`) ?? d
  } catch {
    return d
  }
}
const store = (k: string, v: string) => {
  try {
    localStorage.setItem(`jb.export.${k}`, v)
  } catch {
    /* storage unavailable */
  }
}

/** Parses `hh:mm:ss:ff`, `mm:ss` or plain seconds into ticks. */
function parseTime(s: string, rate: Rational): Ticks | null {
  const parts = s.trim().split(/[:;]/).map(Number)
  if (!parts.length || parts.some(n => !Number.isFinite(n))) return null
  const f = fps(rate)
  let sec = 0
  if (parts.length === 4) sec = parts[0] * 3600 + parts[1] * 60 + parts[2] + parts[3] / f
  else if (parts.length === 3) sec = parts[0] * 3600 + parts[1] * 60 + parts[2]
  else if (parts.length === 2) sec = parts[0] * 60 + parts[1]
  else sec = parts[0]
  return Math.round(sec * 10_000_000)
}

const clock = (s: number) => {
  if (!Number.isFinite(s) || s < 0) return '--:--'
  const m = Math.floor(s / 60)
  return `${m}:${String(Math.floor(s % 60)).padStart(2, '0')}`
}

export function ExportDialog() {
  const job = useEditor(s => s.exportJob)
  const close = () => setState({ exportOpen: false })
  return (
    <div className="modal-backdrop" onMouseDown={e => e.target === e.currentTarget && job?.state !== 'running' && close()}>
      <div className="modal export-modal" role="dialog" aria-label="Export">
        <header>
          <h2>Export</h2>
          <button className="ticon" onClick={close} title={job?.state === 'running' ? 'Hide (export continues)' : 'Close'}>
            <IcClose size={18} />
          </button>
        </header>
        {job ? <ExportProgressView /> : <ExportForm />}
      </div>
    </div>
  )
}

function ExportForm() {
  const project = useEditor(s => s.snap?.project)
  const markIn = useEditor(s => s.markIn)
  const markOut = useEditor(s => s.markOut)
  const engine = backend.engine
  const [name, setName] = useState(project?.name ?? 'My Video')
  const [dir, setDir] = useState(stored('dir', ''))
  const [format, setFormat] = useState<'mp4' | 'mov'>(stored('format', 'mp4') as 'mp4' | 'mov')
  const s = project?.settings
  const big = !!s && s.width * s.height > 1920 * 1080
  const [codec, setCodec] = useState<'h264' | 'hevc'>(stored('codec', big ? 'hevc' : 'h264') as 'h264' | 'hevc')
  const [height, setHeight] = useState(String(s?.height ?? 1080))
  const [rate, setRate] = useState('project')
  const [quality, setQuality] = useState<'low' | 'medium' | 'high'>(stored('quality', 'medium') as 'low' | 'medium' | 'high')
  const [audio, setAudio] = useState(true)
  const hasMarks = markIn != null || markOut != null
  const [range, setRange] = useState<Range>(hasMarks ? 'marks' : 'all')
  const [from, setFrom] = useState('')
  const [to, setTo] = useState('')
  const [unknown, setUnknown] = useState<string[]>([])

  useEffect(() => {
    if (!engine) return
    if (!dir) engine.defaultExportDir().then(setDir, () => {})
    engine.exportCheck().then(setUnknown, () => {})
  }, [])

  const total = project ? sequenceDuration(mainSequence(project)) : 0
  useEffect(() => {
    if (!project) return
    setFrom(timecode(markIn ?? 0, project.settings.frame_rate))
    setTo(timecode(markOut ?? total, project.settings.frame_rate))
  }, [project?.id])

  const heights = useMemo(() => {
    if (!s) return []
    const list = [...new Set([s.height, ...HEIGHTS.filter(h => h < s.height)])]
    return list.map(h => {
      const w = Math.round((s.width * h) / s.height / 2) * 2
      return { value: String(h), label: `${w} x ${h}${h === s.height ? ' (project)' : ''}` }
    })
  }, [s?.width, s?.height])

  if (!project || !s) return null
  const outH = Number(height)
  const outW = Math.round((s.width * outH) / s.height / 2) * 2
  const outRate = rate === 'project' ? s.frame_rate : (RATES.find(r => r.value === rate)?.rate ?? s.frame_rate)
  const span: [Ticks, Ticks] | null =
    range === 'all'
      ? null
      : range === 'marks'
        ? [markIn ?? 0, markOut ?? total]
        : (() => {
            const a = parseTime(from, s.frame_rate)
            const b = parseTime(to, s.frame_rate)
            return a != null && b != null ? [Math.max(0, a), Math.min(total, b)] : null
          })()
  const length = span ? Math.max(0, span[1] - span[0]) : total
  // Mirrors jb-media's default_bit_rate (0.2 bits per pixel per frame).
  const base = outW * outH * fps(outRate) * 0.2
  const bitRate = quality === 'low' ? base / 2 : quality === 'high' ? (base * 8) / 5 : base
  const sizeMb = ((bitRate + (audio ? 192_000 : 0)) * seconds(length)) / 8 / 1e6
  const sep = navigator.userAgent.includes('Windows') ? '\\' : '/'
  const file = `${dir.replace(/[/\\]$/, '')}${sep}${name.replace(/[/\\:]/g, '_')}.${format}`
  const invalid = !name.trim() || !dir || length <= 0 || (range === 'custom' && !span)

  const start = async () => {
    if (!engine || invalid) return
    store('dir', dir)
    store('format', format)
    store('codec', codec)
    store('quality', quality)
    const req: ExportRequest = {
      path: file,
      codec,
      width: outW,
      height: outH,
      frame_rate: rate === 'project' ? null : outRate,
      quality,
      range: span,
      audio,
    }
    setState({ exportJob: { state: 'running', path: file, progress: null, started: Date.now() } })
    try {
      await engine.exportStart(req)
    } catch (e) {
      setState({ exportJob: null })
      toast(String(e), 'error')
    }
  }

  return (
    <>
      <div className="export-body">
        <nav className="export-targets">
          <button className="on">Local</button>
          <button disabled>Device</button>
          <button disabled>Social</button>
        </nav>
        <div className="export-form">
          <div className="prow"><span className="plabel">Name</span><input className="field" value={name} onChange={e => setName(e.target.value)} onKeyDown={e => e.stopPropagation()} /></div>
          <div className="prow">
            <span className="plabel">Save to</span>
            <input className="field" value={dir} onChange={e => setDir(e.target.value)} onKeyDown={e => e.stopPropagation()} />
            <button
              className="ticon"
              title="Choose folder"
              onClick={async () => {
                const r = await backend.pickFiles({ title: 'Export to', directory: true, multiple: false })
                if (r?.[0]) setDir(r[0])
              }}
            >
              <IcFolder size={18} />
            </button>
          </div>
          <div className="prow select-row">
            <span className="plabel">Format</span>
            <Select value={format} onChange={setFormat} options={[{ value: 'mp4', label: 'MP4' }, { value: 'mov', label: 'MOV' }]} />
          </div>
          <div className="prow select-row">
            <span className="plabel">Encoder</span>
            <Select value={codec} onChange={setCodec} options={[{ value: 'h264', label: 'H.264' }, { value: 'hevc', label: 'HEVC (H.265)' }]} />
          </div>
          <div className="prow select-row">
            <span className="plabel">Resolution</span>
            <Select value={height} onChange={setHeight} options={heights} />
          </div>
          <div className="prow select-row">
            <span className="plabel">Frame Rate</span>
            <Select
              value={rate}
              onChange={setRate}
              options={[{ value: 'project', label: `${fps(s.frame_rate).toFixed(3).replace(/\.?0+$/, '')} fps (project)` }, ...RATES.map(r => ({ value: r.value, label: r.label }))]}
            />
          </div>
          <div className="prow">
            <span className="plabel">Quality</span>
            <Pills value={quality} onChange={setQuality} options={[{ value: 'low', label: 'Lower' }, { value: 'medium', label: 'Recommended' }, { value: 'high', label: 'Higher' }]} />
          </div>
          <div className="prow">
            <span className="plabel">Range</span>
            <Pills
              value={range}
              onChange={setRange}
              options={[
                { value: 'all', label: 'Entire project' },
                ...(hasMarks ? [{ value: 'marks' as Range, label: 'In/Out marks' }] : []),
                { value: 'custom', label: 'Custom' },
              ]}
            />
          </div>
          {range === 'custom' && (
            <div className="prow">
              <span className="plabel" />
              <input className="field tc-field" value={from} onChange={e => setFrom(e.target.value)} onKeyDown={e => e.stopPropagation()} title="Start (hh:mm:ss:ff)" />
              <span className="muted">to</span>
              <input className="field tc-field" value={to} onChange={e => setTo(e.target.value)} onKeyDown={e => e.stopPropagation()} title="End (hh:mm:ss:ff)" />
            </div>
          )}
          <div className="prow"><span className="plabel">Audio</span><Toggle on={audio} onChange={setAudio} /></div>
          <div className="prow export-summary">
            <span className="plabel">Output</span>
            <span>
              {timecode(length, s.frame_rate)} · {(bitRate / 1e6).toFixed(1)} Mbps · about {sizeMb >= 1000 ? `${(sizeMb / 1000).toFixed(1)} GB` : `${Math.max(1, Math.round(sizeMb))} MB`}
            </span>
          </div>
          {unknown.length > 0 && (
            <div className="export-warning">
              {unknown.length} effect{unknown.length > 1 ? 's are' : ' is'} not supported yet and will render without the effect:{' '}
              <span title={unknown.join('\n')}>{unknown.slice(0, 3).map(k => k.replace(/^filmora:/, '')).join(', ')}{unknown.length > 3 ? '…' : ''}</span>
            </div>
          )}
        </div>
      </div>
      <footer>
        <span className="muted export-path" title={file}>{file}</span>
        <button className="export-btn big" disabled={invalid || !engine} onClick={start}>
          Export
        </button>
      </footer>
    </>
  )
}

function ExportProgressView() {
  const job = useEditor(s => s.exportJob)!
  const engine = backend.engine
  const [now, setNow] = useState(Date.now())
  useEffect(() => {
    const id = setInterval(() => setNow(Date.now()), 500)
    return () => clearInterval(id)
  }, [])
  const fileName = job.path.split(/[/\\]/).pop()
  if (job.state === 'running') {
    const p = job.progress
    const k = p ? p.frame / Math.max(1, p.total) : 0
    return (
      <>
        <div className="export-progress">
          <div className="export-file">{fileName}</div>
          <div className="export-bar"><div style={{ width: `${(k * 100).toFixed(1)}%` }} /></div>
          <div className="export-stats">
            <span>{(k * 100).toFixed(1)}%</span>
            <span>{p ? `${p.frame} / ${p.total} frames` : 'Preparing…'}</span>
            <span>{p ? `${p.fps.toFixed(1)} fps` : ''}</span>
            <span>Elapsed {clock((now - job.started) / 1000)}</span>
            <span>Remaining {p && p.fps > 0 ? clock(p.eta) : '--:--'}</span>
          </div>
        </div>
        <footer>
          <span className="muted">Rendering with jb-render and jb-export. You can keep editing.</span>
          <button className="export-btn big danger" onClick={() => engine?.exportCancel()}>Cancel</button>
        </footer>
      </>
    )
  }
  const d = job.done
  return (
    <>
      <div className="export-progress">
        <div className="export-file">{d.ok ? 'Export complete' : d.cancelled ? 'Export cancelled' : 'Export failed'}</div>
        {d.ok ? (
          <div className="export-result">
            <div title={d.path}>{d.path}</div>
            <div className="muted">
              {d.width} x {d.height} · {d.frames} frames in {clock(d.elapsed)} ({d.fps.toFixed(1)} fps) · {d.encoder}
              {d.hardware_encoder ? ' (hardware)' : ''}
            </div>
            {d.unknown_effects.length > 0 && (
              <div className="export-warning">Rendered without {d.unknown_effects.length} unsupported effect{d.unknown_effects.length > 1 ? 's' : ''}: {d.unknown_effects.slice(0, 4).map(k => k.replace(/^filmora:/, '')).join(', ')}</div>
            )}
            {d.warnings.length > 0 && <div className="export-warning" title={d.warnings.join('\n')}>{d.warnings.length} warning{d.warnings.length > 1 ? 's' : ''}: {d.warnings[0]}</div>}
          </div>
        ) : (
          <div className="export-result"><div className={d.cancelled ? 'muted' : 'export-error'}>{d.error}</div></div>
        )}
      </div>
      <footer>
        <button className="ticon-text" onClick={() => setState({ exportJob: null })}>New export</button>
        <span className="spacer" />
        {d.ok && (
          <button className="export-btn big" onClick={() => engine?.reveal(d.path)}>
            Open Folder
          </button>
        )}
        <button className="export-btn big ghost" onClick={() => setState({ exportOpen: false, exportJob: null })}>Close</button>
      </footer>
    </>
  )
}
