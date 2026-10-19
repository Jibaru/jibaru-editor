import React from "react";
import { AbsoluteFill, Easing, Img, interpolate, spring, staticFile, useCurrentFrame, useVideoConfig } from "remotion";
import { C, fonts } from "../theme";
import { useLayout } from "../layout";
import { Backdrop, Clip, HeadlineArea, Pill, Words, clamp, useSpring } from "../components/common";
import { Timeline, type TLClip } from "../components/Editor";

/** Visual stage below the headline, inside the safe area. */
const useStage = () => {
  const { W, H, vertical, u, safeTop, safeBottom } = useLayout();
  return vertical
    ? { x: 60, y: safeTop + 330, w: W - 120, h: H - safeTop - safeBottom - 330, cx: W / 2 }
    : { x: 160 * u, y: 270 * u, w: W - 320 * u, h: H - 270 * u - 70 * u, cx: W / 2 };
};

const Headline: React.FC<{ text: string; vtext?: string; accent?: string[]; extra?: React.ReactNode }> = ({ text, vtext, accent, extra }) => {
  const { vertical, u } = useLayout();
  return (
    <HeadlineArea>
      <Words text={vertical ? vtext ?? text : text} size={(vertical ? 104 : 92) * u} accent={accent} />
      {extra}
    </HeadlineArea>
  );
};

/* ----------------------------------------------------------------- .wfp */

export const FeatureWfp: React.FC = () => {
  const frame = useCurrentFrame();
  const { vertical, u } = useLayout();
  const st = useStage();
  const file = useSpring(2, { damping: 10, stiffness: 240 });
  const open = useSpring(16, { damping: 16, stiffness: 180 });
  const fileSize = (vertical ? 250 : 200) * u;
  const tlW = vertical ? st.w : 1100 * u;
  const sp = (d: number) => springAt(frame, d);
  const clips: TLClip[] = [
    { track: 0, start: 0.6, len: 2.2, kind: "title", label: "T  Intro", p: sp(22) },
    { track: 1, start: 0, len: 3.4, kind: "video", thumb: "frames/scene-a.jpg", label: "escena-1.mp4", p: sp(18) },
    { track: 1, start: 3.4, len: 3.6, kind: "video", thumb: "frames/scene-b.jpg", label: "escena-2.mp4", p: sp(21) },
    { track: 2, start: 0, len: 7, kind: "audio", label: "musica.wav", p: sp(24) },
  ];
  return (
    <AbsoluteFill>
      <Backdrop />
      <Headline text="Abre tus proyectos .wfp" vtext={"Abre tus\nproyectos .wfp"} accent={[".wfp"]} />
      <div
        style={{
          position: "absolute",
          left: st.x,
          top: st.y,
          width: st.w,
          height: st.h,
          display: "flex",
          flexDirection: vertical ? "column" : "row",
          alignItems: "center",
          justifyContent: "center",
          gap: (vertical ? 60 : 70) * u,
        }}
      >
        {/* file icon */}
        <div style={{ transform: `scale(${file}) rotate(${(1 - file) * -20}deg)`, display: "flex", flexDirection: "column", alignItems: "center", gap: 18 * u }}>
          <svg width={fileSize * 0.8} height={fileSize} viewBox="0 0 80 100">
            <path d="M6 2 H54 L78 26 V92 a6 6 0 0 1 -6 6 H6 a6 6 0 0 1 -6 -6 V8 a6 6 0 0 1 6 -6 Z" fill={C.panel2} stroke={C.accent} strokeWidth="2.5" />
            <path d="M54 2 V20 a6 6 0 0 0 6 6 H78" fill="none" stroke={C.accent} strokeWidth="2.5" />
            <rect x="14" y="56" width="52" height="24" rx="6" fill={C.accent} />
            <text x="40" y="73" textAnchor="middle" fontFamily={fonts.display} fontWeight={700} fontSize="15" fill={C.bg}>
              .WFP
            </text>
            <rect x="14" y="36" width="34" height="5" rx="2.5" fill={C.dim} />
            <rect x="14" y="45" width="24" height="5" rx="2.5" fill={C.line} />
          </svg>
          <span style={{ fontFamily: fonts.mono, fontSize: 28 * u, color: C.text }}>proyecto.wfp</span>
        </div>
        {/* arrow */}
        <svg width={90 * u} height={90 * u} viewBox="0 0 24 24" style={{ opacity: open, transform: `rotate(${vertical ? 90 : 0}deg) translateX(${(1 - open) * -30 * u}px)` }}>
          <path d="M3 12 H19 M13 6 L19 12 L13 18" fill="none" stroke={C.accent} strokeWidth="2.6" strokeLinecap="round" strokeLinejoin="round" />
        </svg>
        <div style={{ opacity: interpolate(open, [0, 0.3], [0, 1], clamp), transform: `scale(${0.85 + 0.15 * open})` }}>
          <Timeline w={tlW} h={(vertical ? 330 : 360) * u} u={u} duration={7} clips={clips} playhead={interpolate(frame, [20, 54], [0, 3], clamp)} tracks={["V2", "V1", "A1"]} transitions={[{ at: 3.4, track: 1, p: springAt(frame, 30) }]} />
        </div>
      </div>
      <div
        style={{
          position: "absolute",
          left: 0,
          right: 0,
          top: st.y + st.h - (vertical ? 40 : 30) * u,
          textAlign: "center",
          fontFamily: fonts.ui,
          fontSize: (vertical ? 28 : 24) * u,
          color: C.dim,
          opacity: interpolate(frame, [24, 34], [0, 1], clamp),
        }}
      >
        Lee y guarda proyectos .wfp de Filmora, sin conversión.
      </div>
    </AbsoluteFill>
  );
};

/** Spring value at an explicit frame (usable inside loops/props). */
const springAt = (frame: number, delay: number) => spring({ frame: frame - delay, fps: 30, config: { damping: 15, stiffness: 200, mass: 0.6 } });

/* ------------------------------------------------------------------ GPU */

export const FeatureGpu: React.FC = () => {
  const frame = useCurrentFrame();
  const { vertical, u } = useLayout();
  const st = useStage();
  const p = useSpring(0, { damping: 18, stiffness: 160 });
  const pw = vertical ? st.w : 1240 * u;
  const ph = (pw * 9) / 16;
  const fps = 59 + ((frame * 7) % 3 === 0 ? 1 : 0);
  return (
    <AbsoluteFill>
      <Backdrop />
      <Headline text="4K en tiempo real con GPU" vtext={"4K en tiempo\nreal con GPU"} accent={["4K", "GPU"]} />
      <div style={{ position: "absolute", left: st.cx - pw / 2, top: st.y + (vertical ? 40 : 0), width: pw, transform: `perspective(${2000 * u}px) rotateX(${(1 - p) * 25}deg) scale(${0.85 + 0.15 * p})`, opacity: p }}>
        <div style={{ position: "relative", width: pw, height: ph, borderRadius: 18 * u, overflow: "hidden", border: `${2 * u}px solid ${C.line}`, boxShadow: `0 ${40 * u}px ${100 * u}px #000c, 0 0 ${80 * u}px ${C.accent}22` }}>
          <Clip name="flip_pages_right" trimBefore={8} zoom={[1, 1.04]} durationInFrames={54} />
          <div style={{ position: "absolute", left: 22 * u, top: 22 * u, display: "flex", gap: 12 * u }}>
            {["3840 × 2160", `${fps} fps`].map((t, i) => (
              <span key={t + i} style={{ fontFamily: fonts.mono, fontWeight: 700, fontSize: (vertical ? 26 : 24) * u, color: i ? C.bg : C.text, background: i ? C.accent : "#000a", padding: `${6 * u}px ${14 * u}px`, borderRadius: 8 * u }}>
                {t}
              </span>
            ))}
          </div>
          <div style={{ position: "absolute", right: 22 * u, top: 22 * u, display: "flex", alignItems: "center", gap: 10 * u, fontFamily: fonts.mono, fontSize: 22 * u, color: C.text, background: "#000a", padding: `${6 * u}px ${14 * u}px`, borderRadius: 8 * u }}>
            <span style={{ width: 12 * u, height: 12 * u, borderRadius: "50%", background: C.playhead }} />
            0 frames perdidos
          </div>
        </div>
        <div style={{ display: "flex", gap: 16 * u, justifyContent: "center", marginTop: 34 * u, flexWrap: "wrap" }}>
          {["Metal", "Direct3D 12", "Vulkan", "Rust + wgpu"].map((t, i) => (
            <Pill key={t} size={(vertical ? 30 : 26) * u} delay={10 + i * 3}>
              {t}
            </Pill>
          ))}
        </div>
      </div>
    </AbsoluteFill>
  );
};

/* --------------------------------------------------------------- export */

export const FeatureExport: React.FC = () => {
  const frame = useCurrentFrame();
  const { vertical, u } = useLayout();
  const st = useStage();
  const p = useSpring(0);
  const prog = interpolate(frame, [8, 42], [0, 1], { ...clamp, easing: Easing.inOut(Easing.quad) });
  const done = useSpring(43, { damping: 9, stiffness: 280 });
  const cw = vertical ? st.w : 1100 * u;
  const rows: [string, string][] = [
    ["Formato", "MP4 · HEVC"],
    ["Resolución", "3840 × 2160"],
    ["Encoder", "Hardware (GPU)"],
  ];
  const fpsNow = Math.round(interpolate(frame, [8, 20], [0, 100], clamp));
  return (
    <AbsoluteFill>
      <Backdrop />
      <Headline text="Exporta 4K a 100 fps" vtext={"Exporta 4K\na 100 fps"} accent={["4K", "100"]} />
      <div style={{ position: "absolute", left: st.cx - cw / 2, top: st.y + (vertical ? 60 : 10 * u), width: cw, background: C.panel, borderRadius: 22 * u, border: `1px solid ${C.line}`, padding: 44 * u, boxSizing: "border-box", transform: `translateY(${(1 - p) * 120 * u}px)`, opacity: p, boxShadow: `0 ${40 * u}px ${100 * u}px #000b` }}>
        <div style={{ fontFamily: fonts.ui, fontWeight: 700, fontSize: 40 * u, color: C.text, marginBottom: 26 * u }}>Exportar</div>
        {rows.map(([k, v]) => (
          <div key={k} style={{ display: "flex", justifyContent: "space-between", fontFamily: fonts.ui, fontSize: 28 * u, padding: `${14 * u}px 0`, borderBottom: `1px solid ${C.line}` }}>
            <span style={{ color: C.dim }}>{k}</span>
            <span style={{ color: C.text, fontWeight: 600 }}>{v}</span>
          </div>
        ))}
        <div style={{ marginTop: 40 * u, height: 30 * u, borderRadius: 15 * u, background: C.gutter, overflow: "hidden", position: "relative" }}>
          <div style={{ width: `${prog * 100}%`, height: "100%", background: `linear-gradient(90deg, #1aa889, ${C.accent})`, boxShadow: `0 0 ${30 * u}px ${C.accent}`, borderRadius: 15 * u }} />
          <div style={{ position: "absolute", inset: 0, background: `repeating-linear-gradient(110deg, #ffffff22 0 ${14 * u}px, transparent ${14 * u}px ${28 * u}px)`, backgroundPosition: `${frame * 6 * u}px 0`, width: `${prog * 100}%` }} />
        </div>
        <div style={{ display: "flex", justifyContent: "space-between", alignItems: "center", marginTop: 22 * u }}>
          <span style={{ fontFamily: fonts.mono, fontWeight: 700, fontSize: (vertical ? 64 : 56) * u, color: C.text, fontVariantNumeric: "tabular-nums" }}>{Math.round(prog * 100)}%</span>
          <span style={{ fontFamily: fonts.mono, fontWeight: 700, fontSize: (vertical ? 34 : 30) * u, color: C.accent }}>4K · {fpsNow} fps</span>
        </div>
      </div>
      {frame > 42 && (
        <div style={{ position: "absolute", left: 0, right: 0, top: st.y + (vertical ? 820 : 640 * u), display: "flex", justifyContent: "center", transform: `scale(${done})` }}>
          <Pill size={(vertical ? 44 : 38) * u} solid>
            ✓ Listo
          </Pill>
        </div>
      )}
    </AbsoluteFill>
  );
};

/* ------------------------------------------------------------ platforms */

const Window: React.FC<{ os: "mac" | "win"; w: number; u: number; delay: number }> = ({ os, w, u, delay }) => {
  const p = useSpring(delay, { damping: 13, stiffness: 200 });
  const h = w * 0.6;
  return (
    <div style={{ transform: `translateY(${(1 - p) * 200 * u}px) rotate(${(1 - p) * (os === "mac" ? -8 : 8)}deg)`, opacity: p, display: "flex", flexDirection: "column", alignItems: "center", gap: 22 * u }}>
      <div style={{ width: w, height: h, borderRadius: (os === "mac" ? 18 : 8) * u, background: C.panel, border: `${2 * u}px solid ${C.line}`, overflow: "hidden", boxShadow: `0 ${30 * u}px ${80 * u}px #000b` }}>
        <div style={{ height: 40 * u, background: C.gutter, display: "flex", alignItems: "center", padding: `0 ${16 * u}px`, gap: 10 * u, justifyContent: os === "mac" ? "flex-start" : "flex-end" }}>
          {os === "mac"
            ? ["#ff5f57", "#febc2e", "#28c840"].map((c) => <span key={c} style={{ width: 14 * u, height: 14 * u, borderRadius: "50%", background: c }} />)
            : ["—", "▢", "✕"].map((c) => (
                <span key={c} style={{ fontFamily: fonts.ui, color: C.dim, fontSize: 18 * u, width: 34 * u, textAlign: "center" }}>
                  {c}
                </span>
              ))}
        </div>
        <div style={{ display: "flex", gap: 10 * u, padding: 12 * u, height: h - 40 * u - 24 * u, boxSizing: "border-box" }}>
          <div style={{ flex: 1, display: "grid", gridTemplateColumns: "1fr 1fr", gap: 8 * u, alignContent: "start" }}>
            {["cube_rotate", "light_leak", "glitch_digital", "film_burn"].map((c) => (
              <Img key={c} src={staticFile(`cards/${c}.jpg`)} style={{ width: "100%", borderRadius: 4 * u }} />
            ))}
          </div>
          <Img src={staticFile(os === "mac" ? "frames/scene-a.jpg" : "frames/scene-b.jpg")} style={{ flex: 1.4, height: "70%", objectFit: "cover", borderRadius: 6 * u }} />
        </div>
      </div>
      <span style={{ fontFamily: fonts.display, fontWeight: 700, fontSize: 44 * u, color: C.text }}>{os === "mac" ? "macOS" : "Windows"}</span>
      <span style={{ fontFamily: fonts.ui, fontSize: 24 * u, color: C.dim, marginTop: -14 * u }}>{os === "mac" ? "Apple silicon" : "x64"}</span>
    </div>
  );
};

export const FeaturePlatforms: React.FC = () => {
  const { vertical, u } = useLayout();
  const st = useStage();
  const w = vertical ? 560 : 640 * u;
  return (
    <AbsoluteFill>
      <Backdrop />
      <Headline text="Mac y Windows" accent={["Mac", "Windows"]} />
      <div style={{ position: "absolute", left: st.x, top: st.y, width: st.w, height: st.h, display: "flex", flexDirection: vertical ? "column" : "row", alignItems: "center", justifyContent: "center", gap: (vertical ? 30 : 90) * u }}>
        <Window os="mac" w={w} u={u} delay={2} />
        <Window os="win" w={w} u={u} delay={7} />
      </div>
    </AbsoluteFill>
  );
};
