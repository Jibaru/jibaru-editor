// Audio tab: volume, balance, fades, pitch, equalizer, Auto Normalization
// and noise reduction. A camera clip's settings live on its linked audio
// clip (see `audioTarget`).

import { useEffect, useState } from 'react'
import { backend } from '../../backend'
import { defaultAudio, seconds, sourceLength, ticks } from '../../core/model'
import { EQ_FREQUENCIES, EQ_PRESETS, LOUDNESS_TARGETS } from '../../core/presets'
import type { AudioChannels, AudioSettings, Clip, MediaAsset } from '../../core/types'
import { audioTarget, setParam, valueAt } from '../../edit'
import { applyBackend, exec } from '../../store'
import { loadWave } from '../../timeline/waves'
import { KfControl, Module, NumberField, Pills, Select, Slider, SliderRow, Toggle } from '../../ui/controls'
import { IcHelp, IcMic, IcNone, IcNote } from '../../ui/icons'
import { CLOUD_NOTE, kfFor, SubTabs, useDraft, usePlayheadTick } from './common'

export function AudioTab({ clip: selected, media }: { clip: Clip; media: MediaAsset | null }) {
  const clip = audioTarget(selected)
  const [sub, setSub] = useState('Basic')
  return (
    <>
      <SubTabs
        tabs={['Basic', 'Denoise', 'Voice Filters', 'Voice Changer']}
        value={sub}
        onChange={setSub}
        more={[
          { label: 'Audio Ducking', disabled: true },
          { label: 'AI Vocal Remover', disabled: true },
          { label: 'AI Audio Stretch', disabled: true },
        ]}
      />
      <div className="prop-scroll">
        {sub === 'Basic' ? (
          <AudioBasic clip={clip} selected={selected} media={media} />
        ) : sub === 'Denoise' ? (
          <Denoise clip={clip} />
        ) : sub === 'Voice Filters' ? (
          <VoiceFilters clip={clip} />
        ) : (
          <Module title="Voice Changer" toggle={false} toggleDisabled note={CLOUD_NOTE} badge="AI" />
        )}
      </div>
    </>
  )
}

function AudioBasic({ clip, selected, media }: { clip: Clip; selected: Clip; media: MediaAsset | null }) {
  usePlayheadTick()
  const audio: AudioSettings = clip.audio ?? defaultAudio()
  // Keyframes are edited through the selected clip's time (linked clips share it).
  const gain = valueAt(clip, 'volume', 0)
  const pan = valueAt(clip, 'pan', 0)
  const [g, setG] = useDraft(gain)
  const [pn, setPn] = useDraft(pan)
  const [fades, setFades] = useDraft<[number, number]>([audio.fade_in, audio.fade_out])
  const [pitch, setPitch] = useDraft(audio.pitch ?? 0)
  const [eqOpen, setEqOpen] = useState(false)
  const len = seconds(clip.end - clip.start)
  const eq = audio.equalizer
  const eqPreset = eq ? (EQ_PRESETS.find(p => p.bands.every((b, i) => Math.abs(b - eq.bands[i]) < 0.01))?.id ?? 'custom') : 'default'
  return (
    <>
      <div className="wave-card">
        <div className="wave-card-head">
          <IcNote size={16} />
          <span>{media?.name.replace(/\.[^.]+$/, '') ?? clip.name}</span>
          <span className="spacer" />
          <Slider value={g} min={-60} max={10} step={0.1} onInput={setG} onCommit={v => setParam(selected, 'volume', v, clip)} />
        </div>
        {media && <CardWave media={media} clip={clip} />}
      </div>
      <Module title="Adjustment" keyframe={kfFor(selected, ['volume', 'pan'], [gain, pan], clip)}>
        <SliderRow label="Volume" value={g} min={-60} max={10} step={0.1} digits={1} unit="dB" onInput={setG} onCommit={v => setParam(selected, 'volume', v, clip)} keyframe={kfFor(selected, ['volume'], [gain], clip)} />
        <div className="prow select-row">
          <span className="plabel">Audio Channels <span title="Plays one channel on both speakers, mixes to mono or swaps left and right"><IcHelp size={13} /></span></span>
          <Select
            value={audio.channels ?? 'stereo'}
            options={[
              { value: 'stereo', label: 'None' },
              { value: 'left', label: 'Left to Stereo' },
              { value: 'right', label: 'Right to Stereo' },
              { value: 'mono', label: 'Mono' },
              { value: 'swap', label: 'Swap Left and Right' },
            ]}
            onChange={(v: AudioChannels) => exec({ op: 'set_audio_channels', clip_id: clip.id, channels: v })}
          />
        </div>
        <div className="prow slider-row balance">
          <span className="plabel">Sound Balance</span>
          <span className="lr">L</span>
          <Slider value={pn} min={-1} max={1} centered onInput={setPn} onCommit={v => setParam(selected, 'pan', v, clip)} />
          <span className="lr">R</span>
          <NumberField className="pvalue" digits={0} value={pn * 100} onCommit={v => setParam(selected, 'pan', Math.max(-100, Math.min(100, v)) / 100, clip)} />
          <span className="pend">
            <KfOnly clip={selected} target={clip} path="pan" value={pan} />
          </span>
        </div>
        <SliderRow label="Fade In" value={seconds(fades[0])} min={0} max={Math.max(0.01, len / 2)} unit="s" onInput={v => setFades([ticks(v), fades[1]])} onCommit={v => exec({ op: 'set_audio_fades', clip_id: clip.id, fade_in: ticks(v), fade_out: null })} />
        <SliderRow label="Fade Out" value={seconds(fades[1])} min={0} max={Math.max(0.01, len / 2)} unit="s" onInput={v => setFades([fades[0], ticks(v)])} onCommit={v => exec({ op: 'set_audio_fades', clip_id: clip.id, fade_in: null, fade_out: ticks(v) })} />
        <SliderRow label="Pitch" value={pitch} min={-12} max={12} step={0.1} digits={1} centered onInput={setPitch} onCommit={v => exec({ op: 'set_pitch', clip_id: clip.id, semitones: v })} />
        <div className="prow">
          <span className="plabel">Equalizer</span>
          <Select
            className="eq"
            value={eqPreset}
            options={[...EQ_PRESETS.map(p => ({ value: p.id, label: p.label })), ...(eqPreset === 'custom' ? [{ value: 'custom', label: 'Custom' }] : [])]}
            onChange={v => v !== 'custom' && exec({ op: 'set_equalizer', clip_id: clip.id, preset: v, bands: null })}
          />
          <button className={`btn-gray setting ${eqOpen ? 'on' : ''}`} onClick={() => setEqOpen(!eqOpen)}>Setting</button>
        </div>
        {eqOpen && <EqualizerBands clip={clip} bands={eq?.bands ?? EQ_PRESETS[0].bands} />}
        <div className="prow">
          <span className="plabel">Mute</span>
          <span className="spacer" />
          <Toggle on={!!audio.muted} onChange={v => exec({ op: 'set_mute', clip_id: clip.id, muted: v })} />
        </div>
      </Module>
      <Normalization clip={clip} audio={audio} />
    </>
  )
}

function KfOnly({ clip, target, path, value }: { clip: Clip; target: Clip; path: string; value: number }) {
  return <KfControl kf={kfFor(clip, [path], [value], target)} />
}

function EqualizerBands({ clip, bands }: { clip: Clip; bands: number[] }) {
  const [draft, setDraft] = useState(bands)
  useEffect(() => setDraft(bands), [bands.join(',')])
  const commit = (next: number[]) => exec({ op: 'set_equalizer', clip_id: clip.id, preset: null, bands: next })
  return (
    <div className="eq-bands">
      {draft.map((b, i) => (
        <div key={i} className="eq-band" title={`${EQ_FREQUENCIES[i]} Hz: ${b.toFixed(1)} dB`}>
          <input
            type="range"
            min={-12}
            max={12}
            step={0.1}
            value={b}
            aria-label={`${EQ_FREQUENCIES[i]} Hz`}
            onChange={e => setDraft(draft.map((x, j) => (j === i ? Number(e.target.value) : x)))}
            onPointerUp={() => commit(draft)}
            onKeyUp={() => commit(draft)}
          />
          <span>{EQ_FREQUENCIES[i]}</span>
        </div>
      ))}
      <button className="btn-gray" onClick={() => exec({ op: 'set_equalizer', clip_id: clip.id, preset: 'default', bands: null })}>Reset</button>
    </div>
  )
}

function Normalization({ clip, audio }: { clip: Clip; audio: AudioSettings }) {
  const n = audio.normalize
  const [busy, setBusy] = useState(false)
  const run = async (target: number | null) => {
    setBusy(true)
    try {
      await applyBackend(backend.normalizeClip(clip.id, target))
    } finally {
      setBusy(false)
    }
  }
  return (
    <Module title="Auto Normalization" toggle={!!n} onToggle={on => run(on ? (n?.target_lufs ?? -23) : null)} help="Measures the clip's loudness and adjusts it to a target level" defaultOpen={!!n}>
      <div className="prow select-row">
        <span className="plabel">Target</span>
        <Select value={String(n?.target_lufs ?? -23)} disabled={!n || busy} options={LOUDNESS_TARGETS} onChange={v => run(Number(v))} />
      </div>
      <p className="mask-hint">
        {busy ? 'Measuring loudness…' : n?.measured_lufs != null ? `Measured ${n.measured_lufs.toFixed(1)} LUFS, gain ${(n.target_lufs - n.measured_lufs).toFixed(1)} dB` : n ? 'Not measured yet' : 'Off'}
      </p>
    </Module>
  )
}

function Denoise({ clip }: { clip: Clip }) {
  const audio: AudioSettings = clip.audio ?? defaultAudio()
  const on = (audio.denoise_db ?? 0) > 0
  const level = on ? audio.denoise_db! / 0.4 : 50
  const set = (db: number | null) => exec({ op: 'set_audio', clip_id: clip.id, audio: { ...audio, denoise_db: db ?? undefined } })
  return (
    <>
      <Module title="Normal Denoise" toggle={on} onToggle={v => set(v ? 20 : null)} help="Reduces steady background noise (spectral noise reduction)">
        <SliderRow label="Strength" value={level} min={1} max={100} digits={0} disabled={!on} onCommit={v => set(Math.max(0.4, v * 0.4))} />
      </Module>
      <Module title="AI Speech Enhancement" toggle={false} toggleDisabled note={CLOUD_NOTE} badge="AI" defaultOpen={false} />
      <Module title="Wind Removal" toggle={false} toggleDisabled note={CLOUD_NOTE} badge="AI" defaultOpen={false} />
      <Module title="DeReverb" toggle={false} toggleDisabled note="Not available yet." defaultOpen={false} />
      <Module
        title="Hum Removal"
        toggle={audio.hum_removal != null}
        onToggle={v => exec({ op: 'set_audio_cleanup', clip_id: clip.id, hum_removal: v ? 50 : null })}
        help="Removes mains hum (50 or 60 Hz and its harmonics)"
        defaultOpen={audio.hum_removal != null}
      >
        <div className="prow">
          <span className="plabel">Frequency</span>
          <Pills
            value={String(audio.hum_removal ?? 50)}
            onChange={v => exec({ op: 'set_audio_cleanup', clip_id: clip.id, hum_removal: Number(v) })}
            options={[
              { value: '50', label: '50 Hz' },
              { value: '60', label: '60 Hz' },
            ]}
          />
        </div>
      </Module>
      <Module
        title="Hiss Removal"
        toggle={audio.hiss_removal != null}
        onToggle={v => exec({ op: 'set_audio_cleanup', clip_id: clip.id, hiss_removal: v ? 50 : null })}
        help="Reduces high-frequency hiss"
        defaultOpen={audio.hiss_removal != null}
      >
        <SliderRow label="Strength" value={audio.hiss_removal ?? 50} min={0} max={100} digits={0} disabled={audio.hiss_removal == null} onCommit={v => exec({ op: 'set_audio_cleanup', clip_id: clip.id, hiss_removal: v })} />
      </Module>
    </>
  )
}

const VOICE_FILTERS: { id: string; label: string }[] = [
  { id: 'telephone', label: 'Telephone' },
  { id: 'radio', label: 'Radio' },
  { id: 'megaphone', label: 'Megaphone' },
  { id: 'echo', label: 'Echo' },
  { id: 'small_room', label: 'Small Room' },
  { id: 'big_room', label: 'Big Room' },
  { id: 'robot', label: 'Robot' },
  { id: 'lofi', label: 'Lo-Fi' },
  { id: 'underwater', label: 'Underwater' },
]

function VoiceFilters({ clip }: { clip: Clip }) {
  const current = clip.audio?.voice_filter ?? null
  return (
    <div className="preset-grid">
      <button className={`preset ${!current ? 'on' : ''}`} onClick={() => current && exec({ op: 'set_voice_filter', clip_id: clip.id, filter: null })}>
        <div className="preset-thumb">
          <IcNone size={34} strokeWidth={1} />
        </div>
        <span>None</span>
      </button>
      {VOICE_FILTERS.map(f => (
        <button key={f.id} className={`preset ${current === f.id ? 'on' : ''}`} data-voice={f.id} onClick={() => exec({ op: 'set_voice_filter', clip_id: clip.id, filter: f.id })}>
          <div className="preset-thumb">
            <IcMic size={26} />
          </div>
          <span>{f.label}</span>
        </button>
      ))}
    </div>
  )
}

function CardWave({ media, clip }: { media: MediaAsset; clip: Clip }) {
  const [peaks, setPeaks] = useState<number[] | null>(null)
  useEffect(() => {
    let alive = true
    Promise.resolve().then(() =>
      loadWave(media.path).then(w => {
        if (!alive || !w) return
        const from = Math.floor(seconds(clip.source_in ?? 0) * w.pps)
        const to = Math.ceil((seconds(clip.source_in ?? 0) + seconds(sourceLength(clip))) * w.pps)
        const slice = w.peaks.slice(from, to)
        const n = Math.max(2, Math.min(150, slice.length))
        const out: number[] = []
        for (let i = 0; i < n; i++) {
          let m = 0
          for (let j = Math.floor((i * slice.length) / n); j < Math.floor(((i + 1) * slice.length) / n); j++) m = Math.max(m, slice[j])
          out.push(Math.min(1, m * 1.1))
        }
        setPeaks(out)
      }),
    )
    return () => {
      alive = false
    }
  }, [media.path, clip.source_in, clip.end, clip.start])
  return (
    <svg className="wave-card-svg" viewBox={`0 0 ${peaks?.length ?? 150} 40`} preserveAspectRatio="none">
      {peaks && <path d={`M0 40 ${peaks.map((v, i) => `L${i + 0.5} ${40 - v * 38}`).join(' ')} L${peaks.length} 40 Z`} />}
    </svg>
  )
}
