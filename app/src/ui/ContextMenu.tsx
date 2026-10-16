import { useEffect, useLayoutEffect, useRef, useState } from 'react'
import { IcCheck, IcChevronRight } from './icons'

export type MenuItem =
  | { separator: true }
  | {
      label: string
      shortcut?: string
      action?: () => void
      disabled?: boolean
      checked?: boolean
      children?: MenuItem[]
    }

export const sep: MenuItem = { separator: true }

/** Custom right-click / dropdown menu with nested submenus that stay on screen. */
export function ContextMenu({ x, y, items, onClose }: { x: number; y: number; items: MenuItem[]; onClose: () => void }) {
  useEffect(() => {
    const close = (e: Event) => {
      if (!(e.target as HTMLElement).closest?.('.ctx-menu')) onClose()
    }
    const esc = (e: KeyboardEvent) => {
      if (e.key === 'Escape') {
        e.stopPropagation()
        onClose()
      }
    }
    window.addEventListener('mousedown', close, true)
    window.addEventListener('wheel', onClose, { passive: true })
    window.addEventListener('blur', onClose)
    window.addEventListener('keydown', esc, true)
    return () => {
      window.removeEventListener('mousedown', close, true)
      window.removeEventListener('wheel', onClose)
      window.removeEventListener('blur', onClose)
      window.removeEventListener('keydown', esc, true)
    }
  }, [onClose])
  return <List items={items} x={x} y={y} onClose={onClose} />
}

function List({ items, x, y, onClose, parentLeft }: { items: MenuItem[]; x: number; y: number; onClose: () => void; parentLeft?: number }) {
  const ref = useRef<HTMLDivElement>(null)
  const [pos, setPos] = useState({ x, y, ready: false })
  const [open, setOpen] = useState<number | null>(null)
  const [sub, setSub] = useState({ x: 0, y: 0, left: 0 })

  // Keep inside the window: flip left (to the parent's left edge for submenus) and shift up.
  useLayoutEffect(() => {
    const r = ref.current!.getBoundingClientRect()
    const flipX = x + r.width > innerWidth - 4
    setPos({
      x: flipX ? Math.max(4, (parentLeft ?? x) - r.width + (parentLeft != null ? 2 : 0)) : x,
      y: y + r.height > innerHeight - 4 ? Math.max(4, innerHeight - r.height - 4) : y,
      ready: true,
    })
  }, [x, y, parentLeft])

  const hasChecks = items.some(i => !('separator' in i) && i.checked !== undefined)

  return (
    <>
      <div
        className="ctx-menu"
        ref={ref}
        style={{ left: pos.x, top: pos.y, visibility: pos.ready ? 'visible' : 'hidden' }}
        onContextMenu={e => e.preventDefault()}
      >
        {items.map((item, i) =>
          'separator' in item ? (
            <div key={i} className="ctx-sep" />
          ) : (
            <button
              key={i}
              className={open === i ? 'open' : ''}
              disabled={item.disabled}
              onMouseEnter={e => {
                if (item.children && !item.disabled) {
                  const r = e.currentTarget.getBoundingClientRect()
                  const box = ref.current!.getBoundingClientRect()
                  setSub({ x: box.right - 2, y: r.top - 5, left: box.left })
                  setOpen(i)
                } else setOpen(null)
              }}
              onClick={() => {
                if (item.children) return
                onClose()
                item.action?.()
              }}
            >
              {hasChecks && <span className="check">{item.checked ? <IcCheck size={12} /> : null}</span>}
              <span className="label">{item.label}</span>
              {item.shortcut && <kbd>{item.shortcut}</kbd>}
              {item.children && <IcChevronRight size={12} />}
            </button>
          ),
        )}
      </div>
      {open != null && 'children' in items[open] && (items[open] as { children?: MenuItem[] }).children && (
        <List items={(items[open] as { children: MenuItem[] }).children} x={sub.x} y={sub.y} parentLeft={sub.left} onClose={onClose} />
      )}
    </>
  )
}
