// Preset tables shared with jb-core (crates/jb-core/src/{model,speed}.rs).
// Keep in sync: the UI draws curves and band sliders from them, the core
// applies them.

import type { BlendMode, SpeedPoint } from './types'

/** Equalizer presets: 10 band gains in dB (31 Hz .. 16 kHz). */
export const EQ_PRESETS: { id: string; label: string; bands: number[] }[] = [
  { id: 'default', label: 'Default', bands: [0, 0, 0, 0, 0, 0, 0, 0, 0, 0] },
  { id: 'classic', label: 'Classic', bands: [0, 0, 0, 0, 0, 0, 2.16, 0.36, -7.92, -10.08] },
  { id: 'dance', label: 'Dance', bands: [-0.72, 3.96, 5.04, 0.72, -1.08, -1.08, 0, 0.36, 3.96, 3.96] },
  { id: 'low_bass', label: 'Low Bass', bands: [2.88, 3.96, 2.88, 2.88, 2.16, -2.16, -7.92, -10.08, -12.96, -10.08] },
  { id: 'sparkle', label: 'Sparkle', bands: [-3.96, -3.96, -3.96, -2.16, -1.08, 2.16, 5.76, 7.92, 7.92, 7.92] },
  { id: 'pop', label: 'Pop', bands: [-1.08, -2.16, 3.96, 5.76, 5.76, 3.96, -1.08, -2.16, -2.16, -2.16] },
  { id: 'rock_n_roll', label: "Rock 'n' Roll", bands: [-1.08, 2.88, 2.16, 2.88, -1.08, -1.08, 0, 2.16, 2.16, 3.96] },
  { id: 'soft', label: 'Soft', bands: [0, 3.96, 2.16, -1.08, -2.16, -1.08, 3.96, 5.76, 5.76, 7.92] },
  { id: 'techno', label: 'Techno', bands: [-5.76, 1.08, 3.96, -1.08, -1.08, -2.16, 0, 0, 5.04, 5.04] },
  { id: 'hard_rock', label: 'Hard Rock', bands: [-3.96, 0, 0, 0, 0, 0, 2.88, 0, 2.88, 1.08] },
  { id: 'ska', label: 'Ska', bands: [0, -2.16, -3.96, -2.16, 0, 3.96, 5.76, 7.92, 5.76, 5.76] },
  { id: 'country', label: 'Country', bands: [0, -1.08, 0, 0, 2.16, 2.16, 0, 0, 2.88, 2.88] },
  { id: 'acoustic', label: 'Acoustic', bands: [0, 1.08, 2.16, 0, 0, 0, 0, 0, 2.16, 2.16] },
  { id: 'folk', label: 'Folk', bands: [-1.08, 0, 1.08, 0, 2.16, 0, 0, 0, 2.16, 0] },
  { id: 'blues', label: 'Blues', bands: [-1.44, 0, 2.16, 1.08, 0, 0, 0, 0, -1.08, -2.88] },
]

export const EQ_FREQUENCIES = ['31', '62', '125', '250', '500', '1K', '2K', '4K', '8K', '16K']

/** Speed ramping presets: (position in the source range, speed). */
export const RAMP_PRESETS: { id: string; label: string; points: SpeedPoint[] }[] = (
  [
    ['montage', 'Montage', [[0, 0.9], [0.1, 0.9], [0.5, 6.8], [0.65, 0.3], [0.8, 1], [1, 1]]],
    ['hero_moment', 'Hero Moment', [[0, 1], [0.05, 1], [0.35, 5.5], [0.45, 0.5], [0.55, 0.5], [0.65, 5.5], [0.95, 1], [1, 1]]],
    ['bullet_time', 'Bullet Time', [[0, 5.2], [0.4, 5.2], [0.46, 0.5], [0.54, 0.5], [0.6, 5.2], [1, 5.2]]],
    ['jumper', 'Jumper', [[0, 0.6], [0.43, 0.6], [0.5, 6], [0.57, 0.6], [1, 0.6]]],
    ['flash_in', 'Flash In', [[0, 5.2], [0.4, 5.2], [0.6, 1], [1, 1]]],
    ['flash_out', 'Flash Out', [[0, 1], [0.4, 1], [0.6, 5.2], [1, 5.2]]],
  ] as [string, string, [number, number][]][]
).map(([id, label, pts]) => ({ id, label, points: pts.map(([position, speed]) => ({ position, speed })) }))

/** Default custom ramp: flat 1x with five editable points, like Filmora. */
export const CUSTOM_RAMP: SpeedPoint[] = [0, 0.25, 0.5, 0.75, 1].map(position => ({ position, speed: 1 }))

/** Speed at a position of a ramp (linear between points). */
export function rampSpeed(points: SpeedPoint[], pos: number): number {
  if (!points.length) return 1
  if (pos <= points[0].position) return points[0].speed
  for (let i = 1; i < points.length; i++)
    if (pos <= points[i].position) {
      const a = points[i - 1]
      const b = points[i]
      const k = (pos - a.position) / Math.max(1e-9, b.position - a.position)
      return a.speed + (b.speed - a.speed) * k
    }
  return points[points.length - 1].speed
}

/** Timeline length factor of a ramp: integral of 1/speed over the source range. */
export function rampStretch(points: SpeedPoint[]): number {
  const n = 400
  let sum = 0
  for (let i = 0; i < n; i++) sum += 1 / Math.max(0.01, rampSpeed(points, (i + 0.5) / n))
  return sum / n
}

/** Filmora's blend mode menu (Compositing > Blend Mode). */
export const BLEND_MODES: { value: BlendMode; label: string }[] = [
  { value: 'normal', label: 'Normal' },
  { value: 'darken', label: 'Darken' },
  { value: 'multiply', label: 'Multiply' },
  { value: 'color_burn', label: 'Color Burn' },
  { value: 'linear_burn', label: 'Linear Burn' },
  { value: 'darker_color', label: 'Darker Color' },
  { value: 'lighten', label: 'Lighten' },
  { value: 'screen', label: 'Screen' },
  { value: 'color_dodge', label: 'Color Dodge' },
  { value: 'add', label: 'Linear Dodge' },
  { value: 'lighter_color', label: 'Lighter Color' },
  { value: 'overlay', label: 'Overlay' },
  { value: 'soft_light', label: 'Soft Light' },
  { value: 'hard_light', label: 'Hard Light' },
  { value: 'vivid_light', label: 'Vivid Light' },
  { value: 'linear_light', label: 'Linear Light' },
  { value: 'pin_light', label: 'Pin Light' },
  { value: 'hard_mix', label: 'Hard Mix' },
  { value: 'difference', label: 'Difference' },
  { value: 'exclusion', label: 'Exclusion' },
  { value: 'subtract', label: 'Subtract' },
  { value: 'divide', label: 'Divide' },
]

/** Auto Normalization targets offered by Filmora (LUFS). */
export const LOUDNESS_TARGETS = [
  { value: '-23', label: 'EBU R128 (-23 LUFS)' },
  { value: '-16', label: 'Podcast (-16 LUFS)' },
  { value: '-14', label: 'Streaming (-14 LUFS)' },
  { value: '-12', label: 'Loud (-12 LUFS)' },
]

/** Aspect ratios of the player's ratio menu. */
export const ASPECT_RATIOS: { value: string; w: number; h: number }[] = [
  { value: '16:9', w: 16, h: 9 },
  { value: '9:16', w: 9, h: 16 },
  { value: '1:1', w: 1, h: 1 },
  { value: '4:3', w: 4, h: 3 },
  { value: '3:4', w: 3, h: 4 },
  { value: '4:5', w: 4, h: 5 },
  { value: '21:9', w: 21, h: 9 },
]

/** Project size for an aspect ratio, keeping the shorter side (even pixels). */
export function sizeForRatio(width: number, height: number, w: number, h: number): [number, number] {
  const short = Math.min(width, height)
  const even = (v: number) => Math.max(2, Math.round(v / 2) * 2)
  return w >= h ? [even((short * w) / h), even(short)] : [even(short), even((short * h) / w)]
}

/** Ratio label of a project size ("16:9"), or null when it is not in the menu. */
export function ratioOf(width: number, height: number): string | null {
  const r = width / height
  return ASPECT_RATIOS.find(a => Math.abs(a.w / a.h - r) < 0.01)?.value ?? null
}
