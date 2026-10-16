// Title editing (Filmora's Titles tab): text, font, size, style, alignment,
// spacing, color, stroke, shadow and background. Every edit is one
// `set_title_style` command, which also works on Filmora title templates.

import { useState } from 'react'
import type { Clip, TextAlignment, TitleStylePatch } from '../../core/types'
import { exec } from '../../store'
import { Module, NumberField, Select, SliderRow } from '../../ui/controls'
import { IcAlignCenter, IcAlignLeft, IcAlignRight } from '../../ui/icons'
import { ColorInput, fromHex, hex, useDraft } from './common'

const FONTS = ['Arial', 'Helvetica', 'Helvetica Neue', 'Avenir', 'Futura', 'Georgia', 'Times New Roman', 'Courier New', 'Verdana', 'Trebuchet MS', 'Impact', 'Gill Sans', 'Menlo', 'Segoe UI', 'Tahoma']

export function TitleTab({ clip }: { clip: Clip }) {
  const src = clip.source.type === 'title' ? clip.source : null
  const [text, setText] = useState<string | null>(null)
  const [size, setSize] = useDraft(src ? src.font_size * 100 : 5)
  if (!src) return null
  const style = (patch: TitleStylePatch) => exec({ op: 'set_title_style', clip_id: clip.id, ...patch })
  const fonts = FONTS.includes(src.font_family) ? FONTS : [src.font_family, ...FONTS]
  const align = src.alignment ?? 'center'
  return (
    <div className="prop-scroll">
      <Module title="Text">
        <textarea
          className="title-text"
          value={text ?? src.text}
          onChange={e => setText(e.target.value)}
          onBlur={() => {
            if (text != null && text !== src.text) style({ text })
            setText(null)
          }}
          onKeyDown={e => e.stopPropagation()}
        />
        <div className="prow select-row">
          <span className="plabel">Font</span>
          <Select value={src.font_family} options={fonts.map(f => ({ value: f, label: f }))} onChange={v => style({ font_family: v })} />
        </div>
        <SliderRow label="Size" value={size} min={1} max={40} step={0.1} digits={1} onInput={setSize} onCommit={v => style({ font_size: v / 100 })} />
        <div className="prow">
          <span className="plabel">Style</span>
          <button className={`sq-btn ${src.bold ? 'on' : ''}`} title="Bold" onClick={() => style({ bold: !src.bold })}><b>B</b></button>
          <button className={`sq-btn ${src.italic ? 'on' : ''}`} title="Italic" onClick={() => style({ italic: !src.italic })}><i>I</i></button>
          <span className="spacer" />
          {(['left', 'center', 'right'] as TextAlignment[]).map(a => {
            const Icon = a === 'left' ? IcAlignLeft : a === 'center' ? IcAlignCenter : IcAlignRight
            return (
              <button key={a} className={`sq-btn ${align === a ? 'on' : ''}`} title={`Align ${a}`} onClick={() => style({ alignment: a })}>
                <Icon size={16} />
              </button>
            )
          })}
        </div>
        <div className="prow">
          <span className="plabel">Color</span>
          <span className="spacer" />
          <ColorInput value={hex(src.color)} title="Text color" onChange={h => style({ color: fromHex(h, src.color[3]) })} />
        </div>
        <SliderRow label="Opacity" value={src.color[3] * 100} min={0} max={100} digits={0} unit="%" onCommit={v => style({ color: [src.color[0], src.color[1], src.color[2], v / 100] })} />
        <SliderRow label="Letter Spacing" value={src.letter_spacing ?? 0} min={-0.5} max={2} step={0.01} digits={2} onCommit={v => style({ letter_spacing: v })} />
        <SliderRow label="Line Spacing" value={src.line_spacing ?? 0} min={-0.5} max={2} step={0.01} digits={2} onCommit={v => style({ line_spacing: v })} />
        <div className="prow xy-row">
          <span className="plabel">Position</span>
          <NumberField className="xy" prefix="X" value={src.position[0] * 100} digits={1} onCommit={v => style({ position: [v / 100, src.position[1]] })} />
          <NumberField className="xy" prefix="Y" value={src.position[1] * 100} digits={1} onCommit={v => style({ position: [src.position[0], v / 100] })} />
          <span className="pend" />
        </div>
      </Module>
      <Module title="Stroke" toggle={!!src.stroke} onToggle={on => style({ stroke: on ? { color: [0, 0, 0, 1], width: 0.003 } : null })} defaultOpen={!!src.stroke}>
        {src.stroke && (
          <>
            <div className="prow">
              <span className="plabel">Color</span>
              <span className="spacer" />
              <ColorInput value={hex(src.stroke.color)} onChange={h => style({ stroke: { ...src.stroke!, color: fromHex(h, src.stroke!.color[3]) } })} />
            </div>
            <SliderRow label="Width" value={src.stroke.width * 1000} min={0} max={20} step={0.1} digits={1} onCommit={v => style({ stroke: { ...src.stroke!, width: v / 1000 } })} />
          </>
        )}
      </Module>
      <Module title="Shadow" toggle={!!src.shadow} onToggle={on => style({ shadow: on ? { color: [0, 0, 0, 0.6], offset: [0.004, 0.004], blur: 0.004 } : null })} defaultOpen={!!src.shadow}>
        {src.shadow && (
          <>
            <div className="prow">
              <span className="plabel">Color</span>
              <span className="spacer" />
              <ColorInput value={hex(src.shadow.color)} onChange={h => style({ shadow: { ...src.shadow!, color: fromHex(h, src.shadow!.color[3]) } })} />
            </div>
            <SliderRow label="Opacity" value={src.shadow.color[3] * 100} min={0} max={100} digits={0} unit="%" onCommit={v => style({ shadow: { ...src.shadow!, color: [src.shadow!.color[0], src.shadow!.color[1], src.shadow!.color[2], v / 100] } })} />
            <SliderRow label="Distance" value={Math.hypot(...src.shadow.offset) * 1000} min={0} max={50} digits={1} onCommit={v => style({ shadow: { ...src.shadow!, offset: [(v / 1000) * Math.SQRT1_2, (v / 1000) * Math.SQRT1_2] } })} />
            <SliderRow label="Blur" value={src.shadow.blur * 1000} min={0} max={50} digits={1} onCommit={v => style({ shadow: { ...src.shadow!, blur: v / 1000 } })} />
          </>
        )}
      </Module>
      <Module title="Background" toggle={!!src.background} onToggle={on => style({ background: on ? [0, 0, 0, 0.6] : null })} defaultOpen={!!src.background}>
        {src.background && (
          <>
            <div className="prow">
              <span className="plabel">Color</span>
              <span className="spacer" />
              <ColorInput value={hex(src.background)} onChange={h => style({ background: fromHex(h, src.background![3]) })} />
            </div>
            <SliderRow label="Opacity" value={src.background[3] * 100} min={0} max={100} digits={0} unit="%" onCommit={v => style({ background: [src.background![0], src.background![1], src.background![2], v / 100] })} />
          </>
        )}
      </Module>
    </div>
  )
}
