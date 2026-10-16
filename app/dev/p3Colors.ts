import type { Plugin } from 'vite'

/**
 * Declares every CSS color as Display P3.
 *
 * The UI's colors were measured from the reference app, which draws its
 * values straight into the display's (P3) color space. A plain `#237865` in
 * CSS is sRGB, so WebKit converts it and it shows up less saturated than the
 * reference; `color(display-p3 …)` with the same numbers matches it exactly
 * (checked by screen capture: panel, header and accent colors are identical).
 * Hex and rgb()/rgba() values in declarations are rewritten at build time.
 */
export function p3Colors(): Plugin {
  return {
    name: 'jb-p3-colors',
    enforce: 'pre',
    transform(code, id) {
      if (!/\.css($|\?)/.test(id) || id.includes('node_modules')) return null
      return { code: toP3(code), map: null }
    },
  }
}

const f = (n: number) => String(Math.round((n / 255) * 10000) / 10000)

function p3(r: number, g: number, b: number, a = 1): string {
  return a >= 1 ? `color(display-p3 ${f(r)} ${f(g)} ${f(b)})` : `color(display-p3 ${f(r)} ${f(g)} ${f(b)} / ${Math.round(a * 1000) / 1000})`
}

function hex(h: string): string {
  let s = h.slice(1)
  if (s.length === 3 || s.length === 4) s = [...s].map(c => c + c).join('')
  const n = (i: number) => parseInt(s.slice(i, i + 2), 16)
  return p3(n(0), n(2), n(4), s.length === 8 ? n(6) / 255 : 1)
}

function value(v: string): string {
  return v
    .replace(/#(?:[0-9a-fA-F]{8}|[0-9a-fA-F]{6}|[0-9a-fA-F]{3,4})\b/g, hex)
    .replace(/rgba?\(\s*([\d.]+)[\s,]+([\d.]+)[\s,]+([\d.]+)(?:[\s,/]+([\d.]+%?))?\s*\)/g, (_m, r, g, b, a) =>
      p3(+r, +g, +b, a == null ? 1 : a.endsWith('%') ? parseFloat(a) / 100 : +a),
    )
}

export function toP3(css: string): string {
  // Only declaration values (after a property name), never selectors such as `#root`.
  return css.replace(/([{;]\s*(?:--)?[a-zA-Z-]+\s*:)([^;{}]*)/g, (_m, prop: string, v: string) => prop + value(v))
}
