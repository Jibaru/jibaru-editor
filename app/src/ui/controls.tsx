// Small Filmora-style form controls used by the properties panel.
import { useEffect, useRef, useState, type ReactNode } from 'react'
import { IcCaretDown, IcCaretUp, IcChevronDown, IcDiamond, IcHelp, IcReset } from './icons'

export function Toggle({ on, onChange, disabled, title }: { on: boolean; onChange?: (v: boolean) => void; disabled?: boolean; title?: string }) {
  return (
    <button
      className={`switch ${on ? 'on' : ''}`}
      role="switch"
      aria-checked={on}
      disabled={disabled}
      title={title}
      onClick={e => {
        e.stopPropagation()
        onChange?.(!on)
      }}
    >
      <span />
    </button>
  )
}

/** Thin slider with a hollow knob. Commits on release; `onInput` fires while dragging. */
export function Slider({
  value,
  min,
  max,
  step = 0.01,
  onInput,
  onCommit,
  disabled,
  gradient,
  centered,
}: {
  value: number
  min: number
  max: number
  step?: number
  onInput?: (v: number) => void
  onCommit?: (v: number) => void
  disabled?: boolean
  gradient?: string
  centered?: boolean
}) {
  const ref = useRef<HTMLDivElement>(null)
  const k = Math.min(1, Math.max(0, (value - min) / (max - min || 1)))
  const from = centered ? Math.min(0.5, k) : 0
  const to = centered ? Math.max(0.5, k) : k
  const valueAt = (clientX: number) => {
    const r = ref.current!.getBoundingClientRect()
    const raw = min + ((clientX - r.left) / r.width) * (max - min)
    return Math.min(max, Math.max(min, Math.round(raw / step) * step))
  }
  return (
    <div
      ref={ref}
      className={`slider ${disabled ? 'disabled' : ''}`}
      onPointerDown={e => {
        if (disabled || e.button !== 0) return
        e.preventDefault()
        let v = valueAt(e.clientX)
        onInput?.(v)
        const mv = (ev: PointerEvent) => {
          v = valueAt(ev.clientX)
          onInput?.(v)
        }
        const up = () => {
          window.removeEventListener('pointermove', mv)
          window.removeEventListener('pointerup', up)
          onCommit?.(v)
        }
        window.addEventListener('pointermove', mv)
        window.addEventListener('pointerup', up)
      }}
    >
      <div className="slider-track" style={gradient ? { background: gradient } : undefined} />
      {!gradient && <div className="slider-fill" style={{ left: `${from * 100}%`, width: `${(to - from) * 100}%` }} />}
      <div className="slider-knob" style={{ left: `${k * 100}%` }} />
    </div>
  )
}

/** Numeric text field that commits on Enter/blur. */
export function NumberField({
  value,
  onCommit,
  digits = 2,
  prefix,
  suffix,
  className = '',
  disabled,
}: {
  value: number
  onCommit: (v: number) => void
  digits?: number
  prefix?: ReactNode
  suffix?: string
  className?: string
  disabled?: boolean
}) {
  const [text, setText] = useState<string | null>(null)
  const shown = text ?? value.toFixed(digits) + (suffix ?? '')
  const commit = () => {
    if (text == null) return
    const v = parseFloat(text)
    setText(null)
    if (Number.isFinite(v) && v !== value) onCommit(v)
  }
  return (
    <label className={`numfield ${className} ${disabled ? 'disabled' : ''}`}>
      {prefix && <span className="numfield-prefix">{prefix}</span>}
      <input
        value={shown}
        disabled={disabled}
        onFocus={e => {
          setText(value.toFixed(digits))
          requestAnimationFrame(() => e.target.select())
        }}
        onChange={e => setText(e.target.value)}
        onBlur={commit}
        onKeyDown={e => {
          if (e.key === 'Enter') (e.target as HTMLInputElement).blur()
          if (e.key === 'Escape') {
            setText(null)
            ;(e.target as HTMLInputElement).blur()
          }
          e.stopPropagation()
        }}
      />
    </label>
  )
}

export function Diamond({ active, onClick, disabled }: { active?: boolean; onClick?: () => void; disabled?: boolean }) {
  return (
    <button className={`kf ${active ? 'on' : ''}`} onClick={onClick} disabled={disabled} title="Add keyframe">
      <IcDiamond size={16} />
    </button>
  )
}

export function ResetIcon({ onClick }: { onClick: () => void }) {
  return (
    <button className="kf" onClick={onClick} title="Reset">
      <IcReset size={16} />
    </button>
  )
}

/** Label + slider + value + unit + keyframe diamond, the standard Filmora property row. */
export function SliderRow({
  label,
  value,
  min,
  max,
  step,
  digits = 2,
  unit,
  onInput,
  onCommit,
  keyframe = true,
  disabled,
  gradient,
  centered,
  reset,
  sublabel,
}: {
  label: ReactNode
  value: number
  min: number
  max: number
  step?: number
  digits?: number
  unit?: string
  onInput?: (v: number) => void
  onCommit: (v: number) => void
  keyframe?: boolean
  disabled?: boolean
  gradient?: string
  centered?: boolean
  reset?: () => void
  sublabel?: boolean
}) {
  return (
    <div className={`prow slider-row ${disabled ? 'disabled' : ''} ${sublabel ? 'sub' : ''}`}>
      <span className="plabel">{label}</span>
      <Slider value={value} min={min} max={max} step={step} onInput={onInput} onCommit={onCommit} disabled={disabled} gradient={gradient} centered={centered} />
      <NumberField className="pvalue" value={value} digits={digits} onCommit={v => onCommit(Math.min(max, Math.max(min, v)))} disabled={disabled} />
      <span className="punit">{unit ?? ''}</span>
      <span className="pend">
        {reset && <ResetIcon onClick={reset} />}
        {keyframe && <Diamond disabled={disabled} />}
      </span>
    </div>
  )
}

/** Collapsible property module with an optional enable toggle on the left. */
export function Module({
  title,
  toggle,
  onToggle,
  defaultOpen = true,
  help,
  badge,
  keyframe,
  extra,
  children,
}: {
  title: string
  toggle?: boolean
  onToggle?: (v: boolean) => void
  defaultOpen?: boolean
  help?: boolean
  badge?: string
  keyframe?: boolean
  extra?: ReactNode
  children?: ReactNode
}) {
  const [open, setOpen] = useState(defaultOpen)
  return (
    <section className={`module ${toggle === false ? 'off' : ''}`}>
      <div className="module-head" onClick={() => setOpen(!open)}>
        {toggle !== undefined && <Toggle on={toggle} onChange={onToggle} />}
        <span className="module-title">{title}</span>
        {help && <IcHelp size={14} className="help" />}
        {badge && <span className="new-badge">{badge}</span>}
        <span className="module-caret">{open ? <IcCaretDown size={12} /> : <IcCaretUp size={12} />}</span>
        <span className="spacer" />
        {extra}
        {keyframe && <Diamond />}
      </div>
      {open && children && <div className="module-body">{children}</div>}
    </section>
  )
}

/** Dark dropdown that opens a simple list. */
export function Select<T extends string>({
  value,
  options,
  onChange,
  className = '',
  disabled,
}: {
  value: T
  options: { value: T; label: string }[]
  onChange: (v: T) => void
  className?: string
  disabled?: boolean
}) {
  const [open, setOpen] = useState(false)
  const ref = useRef<HTMLDivElement>(null)
  useEffect(() => {
    if (!open) return
    const close = (e: MouseEvent) => {
      if (!ref.current?.contains(e.target as Node)) setOpen(false)
    }
    window.addEventListener('mousedown', close)
    return () => window.removeEventListener('mousedown', close)
  }, [open])
  return (
    <div className={`select ${className} ${disabled ? 'disabled' : ''}`} ref={ref}>
      <button className="select-btn" disabled={disabled} onClick={() => setOpen(!open)}>
        <span>{options.find(o => o.value === value)?.label ?? value}</span>
        <IcChevronDown size={14} />
      </button>
      {open && (
        <div className="select-list">
          {options.map(o => (
            <button
              key={o.value}
              className={o.value === value ? 'on' : ''}
              onClick={() => {
                setOpen(false)
                onChange(o.value)
              }}
            >
              {o.label}
            </button>
          ))}
        </div>
      )}
    </div>
  )
}

export function Pills<T extends string>({ value, options, onChange, className = '' }: { value: T; options: { value: T; label: string }[]; onChange: (v: T) => void; className?: string }) {
  return (
    <div className={`pills ${className}`}>
      {options.map(o => (
        <button key={o.value} className={o.value === value ? 'on' : ''} onClick={() => onChange(o.value)}>
          {o.label}
        </button>
      ))}
    </div>
  )
}
