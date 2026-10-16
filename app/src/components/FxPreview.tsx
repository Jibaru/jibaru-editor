// Card thumbnails of library items (transitions, effects, filters,
// animations, titles): a still rendered by the engine, and on hover the
// looping preview, played from a sprite sheet (see jb_render::preview).
// Stills load only once the card scrolls into view.

import { useEffect, useRef, useState } from 'react'
import { backend, type PreviewRef, type SpriteLayout } from '../backend'
import { useEditor } from '../store'

// One observer for every card: cheap with hundreds of them.
const watchers = new Map<Element, () => void>()
let observer: IntersectionObserver | null = null
function observe(el: Element, onVisible: () => void): () => void {
  if (typeof IntersectionObserver === 'undefined') {
    onVisible()
    return () => {}
  }
  observer ??= new IntersectionObserver(
    entries => {
      for (const e of entries) {
        if (!e.isIntersecting) continue
        watchers.get(e.target)?.()
        watchers.delete(e.target)
        observer?.unobserve(e.target)
      }
    },
    // Start loading a little before the card appears.
    { rootMargin: '150px 0px' },
  )
  watchers.set(el, onVisible)
  observer.observe(el)
  return () => {
    watchers.delete(el)
    observer?.unobserve(el)
  }
}

/** True once `ref` has been (nearly) visible. */
export function useSeen(ref: React.RefObject<Element | null>): boolean {
  const [seen, setSeen] = useState(false)
  useEffect(() => {
    if (seen || !ref.current) return
    return observe(ref.current, () => setSeen(true))
  }, [ref, seen])
  return seen
}

/** Background position of frame `i` in a sprite sheet. */
export function spriteFrame(layout: SpriteLayout, i: number): string {
  const rows = Math.ceil(layout.frames / layout.cols)
  const col = i % layout.cols
  const row = Math.floor(i / layout.cols)
  const x = layout.cols > 1 ? (col / (layout.cols - 1)) * 100 : 0
  const y = rows > 1 ? (row / (rows - 1)) * 100 : 0
  return `${x}% ${y}%`
}

/** Plays a sprite sheet in a loop while mounted, on top of the still. */
function Sprite({ url, layout }: { url: string; layout: SpriteLayout }) {
  const el = useRef<HTMLDivElement>(null)
  const [ready, setReady] = useState(false)
  useEffect(() => {
    let alive = true
    const img = new Image()
    img.onload = () => alive && setReady(true)
    img.src = url
    return () => {
      alive = false
    }
  }, [url])
  useEffect(() => {
    if (!ready) return
    const rows = Math.ceil(layout.frames / layout.cols)
    const d = el.current!
    d.style.backgroundImage = `url("${url}")`
    d.style.backgroundSize = `${layout.cols * 100}% ${rows * 100}%`
    let i = 0
    d.style.backgroundPosition = spriteFrame(layout, 0)
    const id = setInterval(() => {
      i = (i + 1) % layout.frames
      d.style.backgroundPosition = spriteFrame(layout, i)
      d.dataset.frame = String(i)
    }, 1000 / layout.fps)
    return () => clearInterval(id)
  }, [ready, url, layout])
  return <div ref={el} className={`fx-sprite ${ready ? 'ready' : ''}`} data-frame="0" />
}

/**
 * Still + hover animation of a library item, filling its `.thumb`. With
 * `still: false` whatever is under it (a clip thumbnail) shows until hovered.
 */
export function FxThumb({ item, hover, still = true }: { item: PreviewRef; hover: boolean; still?: boolean }) {
  const info = useEditor(s => s.previewInfo)
  const box = useRef<HTMLDivElement>(null)
  const seen = useSeen(box)
  const [broken, setBroken] = useState(false)
  const v = info?.version ?? ''
  return (
    <div className="fx-thumb" ref={box}>
      {still && seen && info && !broken && <img className="fx-still" src={backend.media.fxPreview(item, 'still', v)} alt="" draggable={false} onError={() => setBroken(true)} />}
      {hover && info && !broken && <Sprite url={backend.media.fxPreview(item, 'sprite', v)} layout={info.layout} />}
    </div>
  )
}
