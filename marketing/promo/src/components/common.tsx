import React from "react";
import {
  AbsoluteFill,
  Easing,
  interpolate,
  OffthreadVideo,
  spring,
  staticFile,
  useCurrentFrame,
  useVideoConfig,
} from "remotion";
import { C, fonts } from "../theme";
import { useLayout } from "../layout";

export const clamp = { extrapolateLeft: "clamp", extrapolateRight: "clamp" } as const;

/** Snappy spring used for most entrances. */
export const useSpring = (delay = 0, config: Partial<{ damping: number; stiffness: number; mass: number }> = {}) => {
  const frame = useCurrentFrame();
  const { fps } = useVideoConfig();
  return spring({ frame: frame - delay, fps, config: { damping: 15, stiffness: 210, mass: 0.6, ...config } });
};

/** Dark backdrop with a faint grid and a slow accent glow. */
export const Backdrop: React.FC<{ glow?: string; children?: React.ReactNode }> = ({ glow = C.accent, children }) => {
  const frame = useCurrentFrame();
  const { W, H, u } = useLayout();
  const gx = W * (0.5 + 0.18 * Math.sin(frame / 40));
  const gy = H * (0.45 + 0.08 * Math.cos(frame / 50));
  return (
    <AbsoluteFill style={{ backgroundColor: C.bg, overflow: "hidden" }}>
      <AbsoluteFill
        style={{
          backgroundImage: `linear-gradient(${C.line}33 1px, transparent 1px), linear-gradient(90deg, ${C.line}33 1px, transparent 1px)`,
          backgroundSize: `${60 * u}px ${60 * u}px`,
          backgroundPosition: `${(-frame * 0.6 * u) % (60 * u)}px 0px`,
          maskImage: "radial-gradient(ellipse at center, black 30%, transparent 75%)",
        }}
      />
      <AbsoluteFill
        style={{
          background: `radial-gradient(circle at ${gx}px ${gy}px, ${glow}2e 0%, transparent ${Math.max(W, H) * 0.45}px)`,
        }}
      />
      {children}
    </AbsoluteFill>
  );
};

type WordsProps = {
  text: string;
  size: number;
  delay?: number;
  stagger?: number;
  color?: string;
  /** Words (without punctuation) painted in the accent color. */
  accent?: string[];
  weight?: number;
  font?: string;
  align?: "center" | "left";
  lineHeight?: number;
  style?: React.CSSProperties;
};

/** Kinetic headline: each word springs up out of a blur. `\n` breaks lines. */
export const Words: React.FC<WordsProps> = ({
  text,
  size,
  delay = 0,
  stagger = 3,
  color = C.text,
  accent = [],
  weight = 700,
  font = fonts.display,
  align = "center",
  lineHeight = 1.02,
  style,
}) => {
  const frame = useCurrentFrame();
  const { fps } = useVideoConfig();
  let i = 0;
  return (
    <div
      style={{
        fontFamily: font,
        fontWeight: weight,
        fontSize: size,
        lineHeight,
        letterSpacing: "-0.035em",
        color,
        textAlign: align,
        ...style,
      }}
    >
      {text.split("\n").map((line, li) => (
        <div key={li} style={{ display: "flex", flexWrap: "wrap", justifyContent: align === "center" ? "center" : "flex-start", gap: `0 ${size * 0.24}px` }}>
          {line.split(" ").map((w, wi) => {
            const idx = i++;
            const p = spring({ frame: frame - delay - idx * stagger, fps, config: { damping: 13, stiffness: 240, mass: 0.55 } });
            const isAccent = accent.includes(w.replace(/[.,:!?¡¿]/g, ""));
            return (
              <span
                key={wi}
                style={{
                  display: "inline-block",
                  transform: `translateY(${(1 - p) * size * 0.55}px) scale(${0.82 + 0.18 * p})`,
                  opacity: interpolate(p, [0, 0.4], [0, 1], clamp),
                  filter: `blur(${(1 - Math.min(1, p)) * 12}px)`,
                  color: isAccent ? C.accent : undefined,
                }}
              >
                {w}
              </span>
            );
          })}
        </div>
      ))}
    </div>
  );
};

/** Rounded pill label. */
export const Pill: React.FC<{
  children: React.ReactNode;
  size: number;
  delay?: number;
  solid?: boolean;
  color?: string;
  style?: React.CSSProperties;
}> = ({ children, size, delay = 0, solid = false, color = C.accent, style }) => {
  const p = useSpring(delay, { damping: 11, stiffness: 260 });
  return (
    <div
      style={{
        display: "inline-flex",
        alignItems: "center",
        gap: size * 0.4,
        padding: `${size * 0.38}px ${size * 0.8}px`,
        borderRadius: size * 2,
        fontFamily: fonts.ui,
        fontWeight: 700,
        fontSize: size,
        letterSpacing: "-0.01em",
        color: solid ? C.bg : color,
        background: solid ? color : `${color}1f`,
        border: `${Math.max(2, size * 0.07)}px solid ${color}`,
        transform: `scale(${interpolate(p, [0, 1], [0.4, 1])}) rotate(${(1 - p) * -8}deg)`,
        opacity: interpolate(p, [0, 0.3], [0, 1], clamp),
        whiteSpace: "nowrap",
        ...style,
      }}
    >
      {children}
    </div>
  );
};

/** Full-bleed clip rendered by the Jibaru engine (see scripts/generate-clips.sh). */
export const Clip: React.FC<{
  name: string;
  trimBefore?: number;
  zoom?: [number, number];
  durationInFrames?: number;
  style?: React.CSSProperties;
  fit?: "cover" | "contain";
  playbackRate?: number;
}> = ({ name, trimBefore = 0, zoom = [1.04, 1.12], durationInFrames = 30, style, fit = "cover", playbackRate = 1 }) => {
  const frame = useCurrentFrame();
  const z = interpolate(frame, [0, durationInFrames], zoom, { ...clamp, easing: Easing.out(Easing.quad) });
  return (
    <AbsoluteFill style={{ overflow: "hidden", backgroundColor: "#000", ...style }}>
      <OffthreadVideo
        src={staticFile(`clips/${name}.mp4`)}
        trimBefore={trimBefore}
        playbackRate={playbackRate}
        muted
        style={{ width: "100%", height: "100%", objectFit: fit, transform: `scale(${z})` }}
      />
    </AbsoluteFill>
  );
};

/** Short white flash at the start of a beat (hides hard cuts). */
export const CutFlash: React.FC<{ frames?: number; color?: string; max?: number }> = ({ frames = 5, color = "#fff", max = 0.55 }) => {
  const frame = useCurrentFrame();
  const o = interpolate(frame, [0, frames], [max, 0], clamp);
  return <AbsoluteFill style={{ backgroundColor: color, opacity: o, pointerEvents: "none" }} />;
};

/** Entrance "punch": the whole beat scales down from 1.08 into place. */
export const Punch: React.FC<{ children: React.ReactNode; from?: number }> = ({ children, from = 1.08 }) => {
  const p = useSpring(0, { damping: 20, stiffness: 180 });
  return <AbsoluteFill style={{ transform: `scale(${interpolate(p, [0, 1], [from, 1])})` }}>{children}</AbsoluteFill>;
};

/** Headline block placed inside the safe area (top of the frame). */
export const HeadlineArea: React.FC<{ children: React.ReactNode; top?: number }> = ({ children, top }) => {
  const { safeTop, W, vertical, u } = useLayout();
  return (
    <div
      style={{
        position: "absolute",
        top: top ?? (vertical ? safeTop + 40 : 80 * u),
        left: 0,
        width: W,
        padding: `0 ${vertical ? 70 : 120 * u}px`,
        boxSizing: "border-box",
        display: "flex",
        flexDirection: "column",
        alignItems: "center",
        gap: 26 * u,
      }}
    >
      {children}
    </div>
  );
};
