/** A 6px gutter that resizes neighbouring panels by dragging. */
export function Splitter({ dir, onDrag }: { dir: 'x' | 'y'; onDrag: (delta: number) => void }) {
  return (
    <div
      className={`splitter ${dir}`}
      onPointerDown={e => {
        e.preventDefault()
        let last = dir === 'x' ? e.clientX : e.clientY
        document.body.classList.add(dir === 'x' ? 'resizing-x' : 'resizing-y')
        const mv = (ev: PointerEvent) => {
          const now = dir === 'x' ? ev.clientX : ev.clientY
          onDrag(now - last)
          last = now
        }
        const up = () => {
          document.body.classList.remove('resizing-x', 'resizing-y')
          window.removeEventListener('pointermove', mv)
          window.removeEventListener('pointerup', up)
        }
        window.addEventListener('pointermove', mv)
        window.addEventListener('pointerup', up)
      }}
    />
  )
}
