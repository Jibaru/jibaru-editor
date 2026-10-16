import { useEffect, useState } from 'react'
import { backend } from '../backend'

export interface Wave {
  pps: number
  peaks: number[]
}

const PPS = 20
const cache = new Map<string, Promise<Wave | null>>()

export function loadWave(path: string): Promise<Wave | null> {
  let p = cache.get(path)
  if (!p) {
    p = fetch(backend.media.wave(path, PPS))
      .then(r => (r.ok ? (r.json() as Promise<Wave>) : null))
      .catch(() => null)
    cache.set(path, p)
  }
  return p
}

export function useWave(path: string | null): Wave | null {
  const [wave, setWave] = useState<Wave | null>(null)
  useEffect(() => {
    let alive = true
    if (path) loadWave(path).then(w => alive && setWave(w))
    return () => {
      alive = false
    }
  }, [path])
  return wave
}

/** Draws filled peaks for source range [from, to] seconds into the canvas. */
export function drawWave(canvas: HTMLCanvasElement, wave: Wave, from: number, to: number, color: string, gain = 1) {
  const dpr = devicePixelRatio || 1
  const w = Math.max(1, Math.round(canvas.clientWidth * dpr))
  const h = Math.max(1, Math.round(canvas.clientHeight * dpr))
  if (canvas.width !== w) canvas.width = w
  if (canvas.height !== h) canvas.height = h
  const ctx = canvas.getContext('2d')!
  ctx.clearRect(0, 0, w, h)
  ctx.fillStyle = color
  const span = Math.max(1e-6, to - from)
  for (let x = 0; x < w; x++) {
    const a = from + (x / w) * span
    const b = from + ((x + 1) / w) * span
    const i0 = Math.floor(a * wave.pps)
    const i1 = Math.max(i0 + 1, Math.ceil(b * wave.pps))
    let m = 0
    for (let i = i0; i < i1 && i < wave.peaks.length; i++) m = Math.max(m, wave.peaks[i] ?? 0)
    const bar = Math.min(1, Math.sqrt(m) * gain) * h
    if (bar > 0.5) ctx.fillRect(x, h - bar, 1, bar)
  }
}
