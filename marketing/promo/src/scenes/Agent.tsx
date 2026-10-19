import React from "react";
import { AbsoluteFill, Easing, interpolate, useCurrentFrame } from "remotion";
import { C, fonts } from "../theme";
import { useLayout } from "../layout";
import { Backdrop, HeadlineArea, Pill, Words, clamp, useSpring } from "../components/common";
import { Timeline, type TLClip } from "../components/Editor";

const PROMPT = 'claude "corta los silencios y exporta en 1080p"';
const TYPE = [6, 38] as const;
const OUT = [
  { at: 44, tool: "jibaru.detect_silences", res: "3 silencios > 0,8 s" },
  { at: 58, tool: "jibaru.ripple_delete ×3", res: "−4,2 s" },
  { at: 74, tool: "jibaru.render 1080p", res: "listo ✓" },
];
const MARK_IN = 46;
const COLLAPSE = [60, 76] as const;
const RENDER = [76, 100] as const;

// Speech / silence segments of the take (seconds).
const SEGS: { len: number; silent: boolean }[] = [
  { len: 2.2, silent: false },
  { len: 1.4, silent: true },
  { len: 2.0, silent: false },
  { len: 1.2, silent: true },
  { len: 1.8, silent: false },
  { len: 1.6, silent: true },
];

/** "Edítalo con IA (MCP)": a terminal drives the timeline. */
export const Agent: React.FC = () => {
  const frame = useCurrentFrame();
  const { W, vertical, u, safeTop } = useLayout();
  const term = useSpring(0, { damping: 18, stiffness: 170 });
  const typed = Math.floor(interpolate(frame, TYPE as unknown as number[], [0, PROMPT.length], clamp));
  const cursorOn = Math.floor(frame / 6) % 2 === 0;
  const r = interpolate(frame, COLLAPSE as unknown as number[], [0, 1], { ...clamp, easing: Easing.inOut(Easing.cubic) });
  const markO = interpolate(frame, [MARK_IN, MARK_IN + 6], [0, 1], clamp) * (1 - r);
  const split = frame >= COLLAPSE[0] - 2;
  const render = interpolate(frame, RENDER as unknown as number[], [0, 1], clamp);

  // Layout segments, shrinking silences by (1 - r).
  let t = 0;
  const laid = SEGS.map((s) => {
    const len = s.silent ? s.len * (1 - r) : s.len;
    const o = { ...s, start: t, len };
    t += len;
    return o;
  });
  const total = SEGS.reduce((a, s) => a + s.len, 0);
  const clips: TLClip[] = split
    ? laid.flatMap((s, i) =>
        s.len < 0.05
          ? []
          : [
              { track: 0, start: s.start, len: s.len, kind: "video" as const, thumb: "frames/scene-a.jpg", label: s.silent ? "" : `toma ${i / 2 + 1}` },
              { track: 1, start: s.start, len: s.len, kind: "audio" as const, label: "", flat: s.silent ? [[0, s.len]] : [] },
            ],
      )
    : [
        { track: 0, start: 0, len: total, kind: "video", thumb: "frames/scene-a.jpg", label: "entrevista.mp4" },
        {
          track: 1,
          start: 0,
          len: total,
          kind: "audio",
          label: "voz.wav",
          flat: laid.filter((s) => s.silent).map((s) => [s.start, s.len] as [number, number]),
        },
      ];
  const marks = laid.filter((s) => s.silent).map((s) => [s.start, s.len, markO] as [number, number, number]);

  const tw = vertical ? W - 120 : 1500 * u;
  const termTop = vertical ? safeTop + 290 : 250 * u;
  const tlTop = vertical ? safeTop + 760 : 640 * u;
  const fs = (vertical ? 30 : 30) * u;

  return (
    <AbsoluteFill>
      <Backdrop />
      <HeadlineArea>
        <div style={{ display: "flex", alignItems: "center", gap: 28 * u, flexWrap: "wrap", justifyContent: "center" }}>
          <Words text="Edítalo con IA" size={(vertical ? 104 : 92) * u} accent={["IA"]} />
          <Pill size={(vertical ? 44 : 40) * u} delay={8} solid>
            MCP
          </Pill>
        </div>
      </HeadlineArea>

      {/* terminal */}
      <div
        style={{
          position: "absolute",
          left: (W - tw) / 2,
          top: termTop,
          width: tw,
          borderRadius: 18 * u,
          overflow: "hidden",
          background: "#07090a",
          border: `${2 * u}px solid ${C.line}`,
          boxShadow: `0 ${30 * u}px ${90 * u}px #000c`,
          transform: `translateY(${(1 - term) * 100 * u}px)`,
          opacity: term,
        }}
      >
        <div style={{ height: 44 * u, background: C.gutter, display: "flex", alignItems: "center", gap: 10 * u, padding: `0 ${18 * u}px` }}>
          {["#3a4246", "#3a4246", "#3a4246"].map((c, i) => (
            <span key={i} style={{ width: 14 * u, height: 14 * u, borderRadius: "50%", background: c }} />
          ))}
          <span style={{ marginLeft: 16 * u, fontFamily: fonts.mono, fontSize: 18 * u, color: C.dim }}>~/videos — zsh</span>
        </div>
        <div style={{ padding: `${24 * u}px ${30 * u}px`, fontFamily: fonts.mono, fontSize: fs, lineHeight: 1.5, color: C.text, minHeight: (vertical ? 290 : 260) * u, boxSizing: "border-box" }}>
          <div style={{ whiteSpace: "pre-wrap", wordBreak: "break-word" }}>
            <span style={{ color: C.accent }}>❯ </span>
            {PROMPT.slice(0, typed)}
            {frame < OUT[0].at && <span style={{ opacity: cursorOn ? 1 : 0, background: C.text, color: C.text }}>▌</span>}
          </div>
          {OUT.map((o) => {
            const show = frame >= o.at;
            const p = interpolate(frame, [o.at, o.at + 5], [0, 1], clamp);
            return show ? (
              <div key={o.tool} style={{ opacity: p, transform: `translateX(${(1 - p) * -20 * u}px)`, display: "flex", flexWrap: "wrap", gap: `0 ${14 * u}px`, fontSize: fs * 0.92 }}>
                <span style={{ color: C.accent }}>⏺</span>
                <span style={{ color: "#9fb6ff" }}>{o.tool}</span>
                <span style={{ color: C.dim }}>→ {o.res}</span>
              </div>
            ) : null;
          })}
        </div>
      </div>

      {/* timeline reacting */}
      <div style={{ position: "absolute", left: (W - tw) / 2, top: tlTop, opacity: interpolate(frame, [10, 20], [0, 1], clamp) }}>
        <Timeline w={tw} h={(vertical ? 260 : 250) * u} u={u} duration={total + 0.4} clips={clips} playhead={interpolate(frame, RENDER as unknown as number[], [0, t], clamp)} tracks={["V1", "A1"]} marks={marks} />
        <div style={{ marginTop: 20 * u, height: 14 * u, borderRadius: 7 * u, background: C.panel, overflow: "hidden", opacity: frame >= RENDER[0] ? 1 : 0 }}>
          <div style={{ width: `${render * 100}%`, height: "100%", background: C.accent, boxShadow: `0 0 ${20 * u}px ${C.accent}` }} />
        </div>
        <div style={{ marginTop: 10 * u, fontFamily: fonts.mono, fontSize: 22 * u, color: C.dim, display: "flex", justifyContent: "space-between", opacity: frame >= RENDER[0] ? 1 : 0 }}>
          <span>render · 1080p</span>
          <span style={{ color: render >= 1 ? C.accent : C.dim }}>{render >= 1 ? "entrevista-1080p.mp4 ✓" : `${Math.round(render * 100)}%`}</span>
        </div>
      </div>
    </AbsoluteFill>
  );
};
