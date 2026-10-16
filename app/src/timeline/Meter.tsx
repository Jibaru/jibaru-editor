// Audio meter panel at the right of the timeline (the toolbar's meter button),
// fed by the native engine's playback level.

import { useEffect, useState } from 'react'
import { backend } from '../backend'
import { useEditor } from '../store'
import { IcCaretUp } from '../ui/icons'
import { setUi } from './tlstate'

const SCALE = [0, -6, -12, -18, -24, -30, -36, -42, -48, -54]

export function AudioMeter() {
  const playing = useEditor(s => s.playing)
  const [db, setDb] = useState(-Infinity)
  useEffect(() => {
    const eng = backend.engine
    if (!eng || !playing) {
      setDb(-Infinity)
      return
    }
    let alive = true
    const tick = async () => {
      try {
        const s = await eng.stats()
        if (alive) setDb(s.audio_rms > 0 ? 20 * Math.log10(s.audio_rms) + 3 : -Infinity)
      } catch {
        /* engine busy */
      }
      if (alive) window.setTimeout(tick, 80)
    }
    tick()
    return () => {
      alive = false
    }
  }, [playing])
  const k = Number.isFinite(db) ? Math.max(0, Math.min(1, (db + 60) / 60)) : 0
  return (
    <aside className="tl-meter" aria-label="Audio meter">
      <button className="meter-head" onClick={() => setUi({ meter: false })}>
        Meter <IcCaretUp size={10} />
      </button>
      <div className="meter-body">
        <div className="meter-bars">
          <i style={{ ['--k' as string]: k }} />
          <i style={{ ['--k' as string]: k * 0.97 }} />
        </div>
        <div className="meter-scale">
          {SCALE.map(d => (
            <span key={d} style={{ top: `${(-d / 60) * 100}%` }}>{d}</span>
          ))}
          <span className="db">dB</span>
        </div>
      </div>
    </aside>
  )
}
