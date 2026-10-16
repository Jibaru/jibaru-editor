// Update notice (bottom-right card, like Filmora's): "A new version is
// available" with the release notes and Update now / Remind me later / Skip
// this version, download progress, and the result of a manual check.

import {
  autoCheckEnabled,
  dismiss,
  remindLater,
  setAutoCheck,
  skipVersion,
  updateNow,
  useUpdatePhase,
} from '../updates'
import { useEditor } from '../store'
import { IcClose } from '../ui/icons'

const mb = (n: number) => `${(n / 1048576).toFixed(1)} MB`

/** GitHub's generated notes are Markdown: keep the text, drop the markup. */
function plainNotes(md: string): string {
  return md
    .replace(/\r/g, '')
    .replace(/<!--[\s\S]*?-->/g, '')
    .replace(/\[([^\]]+)\]\([^)]+\)/g, '$1')
    .replace(/^#+\s*/gm, '')
    .replace(/^\s*[*-]\s+/gm, '• ')
    .replace(/\*\*|__|`/g, '')
    .replace(/\n{3,}/g, '\n\n')
    .trim()
}

export function UpdateNotice() {
  const phase = useUpdatePhase()
  const exporting = useEditor(s => s.exportJob?.state === 'running')
  if (phase.kind === 'hidden') return null

  const close = (
    <button className="ticon" title="Close" onClick={phase.kind === 'available' ? remindLater : dismiss}>
      <IcClose size={14} />
    </button>
  )
  const autoToggle = (
    <label className="update-auto">
      <input type="checkbox" checked={autoCheckEnabled()} onChange={e => setAutoCheck(e.target.checked)} />
      Check for updates automatically
    </label>
  )

  let title: string
  let body: React.ReactNode = null
  let actions: React.ReactNode = null
  switch (phase.kind) {
    case 'checking':
      title = 'Checking for updates...'
      body = <div className="update-bar indeterminate"><div /></div>
      break
    case 'current':
      title = "You're up to date"
      body = <p>Jibaru Editor {phase.version} is the latest version.</p>
      actions = <><span className="spacer" />{autoToggle}<button className="btn-accent" onClick={dismiss}>OK</button></>
      break
    case 'error':
      title = 'Update'
      body = <p className="update-error">{phase.message}</p>
      actions = <><span className="spacer" /><button className="btn-gray" onClick={dismiss}>Close</button></>
      break
    case 'available': {
      const notes = plainNotes(phase.info.notes)
      title = `A new version ${phase.info.version} is available`
      body = (
        <>
          <p className="update-sub">You have {phase.info.current}.{phase.info.date ? ` Released ${new Date(phase.info.date).toLocaleDateString()}.` : ''}</p>
          {notes && <pre className="update-notes">{notes}</pre>}
          {exporting && <p className="update-sub">The update will install after the current export finishes.</p>}
        </>
      )
      actions = (
        <>
          <button className="btn-link" onClick={skipVersion}>Skip this version</button>
          <span className="spacer" />
          <button className="btn-gray" onClick={remindLater}>Remind me later</button>
          <button className="btn-accent" onClick={() => updateNow()}>Update now</button>
        </>
      )
      break
    }
    case 'waiting-export':
      title = `Jibaru Editor ${phase.info.version}`
      body = <p>Waiting for the export to finish before installing the update...</p>
      break
    case 'downloading': {
      const pct = phase.total ? Math.min(100, (phase.done / phase.total) * 100) : null
      title = `Downloading ${phase.info.version}...`
      body = (
        <>
          <div className={`update-bar ${pct == null ? 'indeterminate' : ''}`}><div style={pct == null ? undefined : { width: `${pct}%` }} /></div>
          <p className="update-sub">{pct == null ? mb(phase.done) : `${Math.floor(pct)}% · ${mb(phase.done)} of ${mb(phase.total!)}`}</p>
        </>
      )
      break
    }
    case 'installing':
      title = `Installing ${phase.info.version}...`
      body = <p className="update-sub">Jibaru Editor will restart in a moment.</p>
      break
  }

  const closable = phase.kind !== 'downloading' && phase.kind !== 'installing' && phase.kind !== 'checking'
  return (
    <div className="update-notice" role="dialog" aria-label={title}>
      <header>
        <h3>{title}</h3>
        {closable && close}
      </header>
      {body}
      {actions && <footer>{actions}</footer>}
    </div>
  )
}
