// Library card previews for the in-browser backend: simple canvas drawings
// with the same still + sprite sheet contract as the native generator
// (jb_render::preview), so the UI and its tests run without the engine.

import type { PlayerFrames, PreviewRef, SpriteLayout } from './types'

export const MOCK_LAYOUT: SpriteLayout = { frame_w: 128, frame_h: 72, frames: 30, cols: 6, fps: 15 }

/** Real previews from the dev server (`JB_FX_PREVIEWS`, see dev/mediaServer.ts), when available. */
let served: { version: string; layout: SpriteLayout } | null = null

export async function mockPreviewInfo() {
  try {
    const r = await fetch('/__jb/fxpreview-info')
    if (r.ok) served = await r.json()
  } catch {
    /* no dev server */
  }
  return served ?? { version: 'mock', layout: MOCK_LAYOUT }
}

const cache = new Map<string, string>()

function hash(s: string): number {
  let h = 2166136261
  for (let i = 0; i < s.length; i++) h = Math.imul(h ^ s.charCodeAt(i), 16777619)
  return h >>> 0
}

/** Scene A (cool) or B (warm) at time `t` (0..1 of the preview). */
function scene(ctx: CanvasRenderingContext2D, x: number, y: number, w: number, h: number, b: boolean, t: number) {
  const g = ctx.createLinearGradient(0, y, 0, y + h)
  g.addColorStop(0, b ? '#3a1a5e' : '#2c5fa8')
  g.addColorStop(0.6, b ? '#ffb05a' : '#cfe6f3')
  g.addColorStop(0.6, b ? '#5a2a5a' : '#2b4a5a')
  g.addColorStop(1, b ? '#1a0a20' : '#122630')
  ctx.fillStyle = g
  ctx.fillRect(x, y, w, h)
  ctx.fillStyle = b ? '#ffe9a0' : '#ffffff'
  ctx.beginPath()
  ctx.arc(x + w * (b ? 0.65 - 0.1 * t : 0.25 + 0.1 * t), y + h * (b ? 0.55 : 0.25), h * 0.12, 0, Math.PI * 2)
  ctx.fill()
}

function frame(ctx: CanvasRenderingContext2D, x: number, y: number, w: number, h: number, item: PreviewRef, t: number) {
  ctx.save()
  ctx.beginPath()
  ctx.rect(x, y, w, h)
  ctx.clip()
  if ('title' in item) {
    scene(ctx, x, y, w, h, false, t)
    ctx.fillStyle = '#fff'
    ctx.font = `bold ${Math.round(h * item.title.font_size * 2.2)}px sans-serif`
    ctx.textAlign = 'center'
    ctx.textBaseline = 'middle'
    ctx.fillText(item.title.text.split('\n')[0], x + w * item.title.position[0], y + h * item.title.position[1])
  } else if (item.fx.includes('.anim.') || item.fx.startsWith('jb.anim')) {
    scene(ctx, x, y, w, h, true, t)
    const k = Math.min(1, t * 2)
    const s = 0.6 * k
    scene(ctx, x + (w * (1 - s)) / 2, y + (h * (1 - s)) / 2, w * s, h * s, false, t)
  } else {
    const kind = hash(item.fx) % 3
    const p = Math.min(1, Math.max(0, (t - 0.2) / 0.6))
    scene(ctx, x, y, w, h, false, t)
    if (kind === 0) {
      ctx.globalAlpha = p
      scene(ctx, x, y, w, h, true, t)
    } else if (kind === 1) {
      ctx.beginPath()
      ctx.rect(x, y, w * p, h)
      ctx.clip()
      scene(ctx, x, y, w, h, true, t)
    } else {
      ctx.fillStyle = `hsla(${hash(item.fx) % 360}, 80%, 50%, 0.45)`
      ctx.fillRect(x, y, w, h)
    }
  }
  ctx.restore()
}

/** Data URL of a card still or sprite sheet. */
export function mockPreviewUrl(item: PreviewRef, part: 'still' | 'sprite'): string {
  if (served && 'fx' in item) return `/__jb/fxpreview?fx=${encodeURIComponent(item.fx)}&part=${part}`
  const key = `${part}:${JSON.stringify(item)}`
  const hit = cache.get(key)
  if (hit) return hit
  const L = served?.layout ?? MOCK_LAYOUT
  const c = document.createElement('canvas')
  const rows = Math.ceil(L.frames / L.cols)
  c.width = part === 'still' ? L.frame_w : L.frame_w * L.cols
  c.height = part === 'still' ? L.frame_h : L.frame_h * rows
  const ctx = c.getContext('2d')
  if (!ctx) return ''
  if (part === 'still') frame(ctx, 0, 0, L.frame_w, L.frame_h, item, 0.5)
  else for (let i = 0; i < L.frames; i++) frame(ctx, (i % L.cols) * L.frame_w, Math.floor(i / L.cols) * L.frame_h, L.frame_w, L.frame_h, item, i / L.frames)
  const url = c.toDataURL('image/jpeg', 0.7)
  cache.set(key, url)
  return url
}

/**
 * Player preview frames rendered by jb-render through the dev server
 * (`/__jb/fxframes`, which runs `jb fx player-frames`); null without it.
 */
export async function mockPlayerFrames(item: PreviewRef, height: number): Promise<PlayerFrames | null> {
  const q = 'fx' in item ? `fx=${encodeURIComponent(item.fx)}` : `title=${encodeURIComponent(JSON.stringify(item.title))}`
  try {
    const r = await fetch(`/__jb/fxframes?${q}&h=${height}`)
    if (!r.ok) return null
    const j = (await r.json()) as { id: string; frames: number; width: number; height: number; duration: number }
    const urls = Array.from({ length: j.frames }, (_, i) => `/__jb/fxframe?id=${j.id}&i=${i}`)
    return { urls, width: j.width, height: j.height, duration: j.duration }
  } catch {
    return null
  }
}
