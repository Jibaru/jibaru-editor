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
  // Display P3 like the CSS colors (see dev/p3Colors.ts); hex colors are taken as P3 values.
  const ctx = (canvas.getContext('2d', { colorSpace: 'display-p3' }) ?? canvas.getContext('2d'))!
  ctx.clearRect(0, 0, w, h)
  ctx.fillStyle = hexToP3(color)
  const span = Math.max(1e-6, to - from)
  // Normalized to the loudest peak of the media, like the reference's waveforms.
  const top = peakOf(wave)
  for (let x = 0; x < w; x++) {
    const a = from + (x / w) * span
    const b = from + ((x + 1) / w) * span
    const i0 = Math.floor(a * wave.pps)
    const i1 = Math.max(i0 + 1, Math.ceil(b * wave.pps))
    let m = 0
    for (let i = i0; i < i1 && i < wave.peaks.length; i++) m = Math.max(m, wave.peaks[i] ?? 0)
    const bar = Math.min(1, Math.sqrt(m / top) * gain) * h
    if (bar > 0.5) ctx.fillRect(x, h - bar, 1, bar)
  }
}

function hexToP3(c: string): string {
  const m = /^#([0-9a-f]{2})([0-9a-f]{2})([0-9a-f]{2})$/i.exec(c)
  if (!m) return c
  const v = (x: string) => (parseInt(x, 16) / 255).toFixed(4)
  return `color(display-p3 ${v(m[1])} ${v(m[2])} ${v(m[3])})`
}

const peaks = new WeakMap<Wave, number>()
function peakOf(w: Wave): number {
  let v = peaks.get(w)
  if (v == null) {
    v = 1e-4
    for (const x of w.peaks) if (x > v) v = x
    peaks.set(w, v)
  }
  return v
}
