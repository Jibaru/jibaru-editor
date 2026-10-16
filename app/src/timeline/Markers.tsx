// Timeline and clip markers on the ruler: small shields in the marker color.
// Double-click opens the Marker Editor, right-click offers edit
// and delete.

import type { Project } from '../core/types'
import { exec, openMenu } from '../store'
import { sep } from '../ui/ContextMenu'
import { allMarkers, MARKER_COLORS, removeAllMarkers } from './edit'
import { HEADER_W, tx } from './geometry'
import { setUi } from './tlstate'

const DEFAULT_JB = [0, 206 / 255, 208 / 255]

/**
 * CSS color of a marker, in Display P3 like the rest of the UI (see
 * dev/p3Colors.ts). jb-core's default color shows as Filmora's default swatch.
 */
export function markerCss(c?: [number, number, number, number]): string {
  const same = (a: number[], b: number[]) => a.slice(0, 3).every((x, i) => Math.abs(x - b[i]) < 1e-3)
  // The default swatch is drawn a little darker on the ruler, as in the reference.
  const v = !c || same(c, DEFAULT_JB) || same(c, MARKER_COLORS[1]) ? [0x3c / 255, 0x9a / 255, 0x86 / 255] : c
  const n = (x: number) => Math.min(1, Math.max(0, x)).toFixed(4)
  return `color(display-p3 ${n(v[0])} ${n(v[1])} ${n(v[2])})`
}

export function Markers({ project, pps, from, width }: { project: Project; pps: number; from: number; width: number }) {
  const list = allMarkers(project)
  if (!list.length) return null
  return (
    <>
      {list.map(({ marker, clip, at }) => {
        const x = tx(at, pps)
        if (x < from - 20 || x > from + width + 20) return null
        return (
          <div
            key={marker.id}
            className={`tl-marker ${clip ? 'clip-marker' : ''}`}
            data-marker={marker.id}
            title={marker.name || undefined}
            style={{ left: HEADER_W + x, ['--mk' as string]: markerCss(marker.color) }}
            onPointerDown={e => e.stopPropagation()}
            onDoubleClick={e => {
              e.stopPropagation()
              setUi({ dialog: { kind: 'marker', id: marker.id } })
            }}
            onContextMenu={e => {
              e.preventDefault()
              e.stopPropagation()
              openMenu(e.clientX, e.clientY, [
                { label: 'Edit Marker', action: () => setUi({ dialog: { kind: 'marker', id: marker.id } }) },
                { label: 'Delete Marker', action: () => exec({ op: 'remove_marker', marker_id: marker.id }) },
                sep,
                { label: 'Delete All Markers', action: removeAllMarkers },
              ])
            }}
          />
        )
      })}
    </>
  )
}

/** A color swatch in Display P3. */
export const swatchCss = (c: number[]) => `color(display-p3 ${c.slice(0, 3).map(x => x.toFixed(4)).join(" ")})`
