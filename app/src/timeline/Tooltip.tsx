// Filmora-style hover tooltips: a dark rounded box under the control with a
// notch, the label and its shortcut keys drawn as key caps.

import { useEffect, useRef, useState, type ButtonHTMLAttributes, type ReactNode } from 'react'
import { createPortal } from 'react-dom'

export interface Tip {
  label: string
  keys?: string[]
  /** Extra lines (transition tooltips). */
  lines?: string[]
}

const DELAY = 450

export function useHoverTip(tip: Tip | null) {
  const [at, setAt] = useState<{ x: number; y: number; left: number } | null>(null)
  const timer = useRef<number | undefined>(undefined)
  useEffect(() => () => window.clearTimeout(timer.current), [])
  const bind = {
    onMouseEnter: (e: React.MouseEvent<HTMLElement>) => {
      const el = e.currentTarget
      window.clearTimeout(timer.current)
      timer.current = window.setTimeout(() => {
        const r = el.getBoundingClientRect()
        setAt({ x: r.left + r.width / 2, y: r.bottom + 7, left: r.left })
      }, DELAY)
    },
    onMouseLeave: () => {
      window.clearTimeout(timer.current)
      setAt(null)
    },
    onMouseDown: () => {
      window.clearTimeout(timer.current)
      setAt(null)
    },
  }
  const node = tip && at ? <TipBox tip={tip} x={at.x} y={at.y} /> : null
  return { bind, node }
}

export function TipBox({ tip, x, y, anchor = 'center' }: { tip: Tip; x: number; y: number; anchor?: 'center' | 'cursor' }) {
  const ref = useRef<HTMLDivElement>(null)
  const [dx, setDx] = useState(0)
  useEffect(() => {
    const el = ref.current
    if (!el) return
    const r = el.getBoundingClientRect()
    const over = r.right - (window.innerWidth - 6)
    const under = 6 - r.left
    setDx(over > 0 ? -over : under > 0 ? under : 0)
  }, [x, y, tip.label])
  return createPortal(
    <div ref={ref} className={`tb-tip ${anchor} ${tip.lines ? 'multi' : ''}`} style={{ left: x + dx, top: y, ['--notch' as string]: `${-dx}px` }}>
      <span className="tb-tip-label">{tip.label}</span>
      {tip.keys?.map((k, i) => <kbd key={i}>{k}</kbd>)}
      {tip.lines?.map((l, i) => <span key={i} className="tb-tip-line">{l}</span>)}
    </div>,
    document.body,
  )
}

type BtnProps = ButtonHTMLAttributes<HTMLButtonElement> & { tip: Tip; children: ReactNode }

/** Toolbar button with a Filmora tooltip. Disabled buttons still show theirs. */
export function TipButton({ tip, children, className = '', ...rest }: BtnProps) {
  const { bind, node } = useHoverTip(tip)
  return (
    <span className="tip-wrap" {...bind}>
      <button className={`ticon ${className}`} aria-label={tip.label} {...rest}>
        {children}
      </button>
      {node}
    </span>
  )
}
